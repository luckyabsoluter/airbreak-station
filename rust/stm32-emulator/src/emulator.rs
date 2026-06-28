// SPDX-License-Identifier: GPL-3.0-or-later

use std::{mem::MaybeUninit, sync::atomic::{AtomicU64, Ordering, AtomicBool}, cell::RefCell, collections::BTreeMap, path::Path, rc::Rc};
use svd_parser::svd::Device as SvdDevice;
use unicorn_engine::{unicorn_const::{Arch, Mode, HookType, MemType}, Unicorn, RegisterARM};
use crate::{config::Config, util::UniErr, Args, system::System, framebuffers::sdl_engine::{PUMP_EVENT_INST_INTERVAL, SDL}};
use anyhow::{Context as _, Result, bail};
use capstone::prelude::*;

#[repr(C)]
struct VectorTable {
    pub sp: u32,
    pub reset: u32,
}

impl VectorTable {
    pub fn from_memory(uc: &Unicorn<()>, addr: u32) -> Result<Self> {
        unsafe {
            let mut self_ = MaybeUninit::<Self>::uninit();
            let buf = std::slice::from_raw_parts_mut(self_.as_mut_ptr() as *mut u8, std::mem::size_of::<Self>());
            uc.mem_read(addr.into(), buf).map_err(UniErr)?;
            Ok(self_.assume_init())
        }
    }
}

fn thumb(pc: u64) -> u64 {
    pc | 1
}

// PC + instruction size
pub static mut LAST_INSTRUCTION: (u32, u8) = (0,0);
pub static NUM_INSTRUCTIONS: AtomicU64 = AtomicU64::new(0);
static CONTINUE_EXECUTION: AtomicBool = AtomicBool::new(false);
static BUSY_LOOP_REACHED: AtomicBool = AtomicBool::new(false);
static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

fn disassemble_instruction(diassembler: &Capstone, uc: &Unicorn<()>, pc: u64) -> String {
    let mut instr = [0; 4];
    if uc.mem_read(pc, &mut instr).is_err() {
        return "failed to read memory at pc".to_string();
    }

    if let Ok(disasm) = diassembler.disasm_count(&instr, pc, 1) {
        if let Some(instr) = disasm.first() {
            return format!("{:5} {}", instr.mnemonic().unwrap(), instr.op_str().unwrap());
        }
    }

    return "??".to_string();
}

fn read_reg_or_zero(uc: &Unicorn<()>, reg: RegisterARM) -> u64 {
    uc.reg_read(reg).unwrap_or(0)
}

fn log_register_sample(uc: &Unicorn<()>, label: &str, addr: u32, hit: u64) {
    info!(
        "{} addr=0x{:08x} hit={} r0=0x{:08x} r1=0x{:08x} r2=0x{:08x} r3=0x{:08x} r12=0x{:08x} lr=0x{:08x} sp=0x{:08x} psp=0x{:08x} msp=0x{:08x} basepri=0x{:02x} primask=0x{:x} ipsr=0x{:03x}",
        label,
        addr,
        hit,
        read_reg_or_zero(uc, RegisterARM::R0) as u32,
        read_reg_or_zero(uc, RegisterARM::R1) as u32,
        read_reg_or_zero(uc, RegisterARM::R2) as u32,
        read_reg_or_zero(uc, RegisterARM::R3) as u32,
        read_reg_or_zero(uc, RegisterARM::R12) as u32,
        read_reg_or_zero(uc, RegisterARM::LR) as u32,
        read_reg_or_zero(uc, RegisterARM::SP) as u32,
        read_reg_or_zero(uc, RegisterARM::PSP) as u32,
        read_reg_or_zero(uc, RegisterARM::MSP) as u32,
        read_reg_or_zero(uc, RegisterARM::BASEPRI) as u8,
        read_reg_or_zero(uc, RegisterARM::PRIMASK),
        read_reg_or_zero(uc, RegisterARM::IPSR) as u32 & 0x1ff,
    );
}

fn parse_front_panel_sequence(sequence: &str) -> Vec<String> {
    sequence
        .split(',')
        .map(str::trim)
        .filter(|action| !action.is_empty())
        .map(|action| action.to_ascii_lowercase())
        .collect()
}

struct ScheduledFrontPanelAction {
    action: String,
    at: u64,
    label: String,
    sent: bool,
    after_snapshot: bool,
}

fn parse_front_panel_schedule(sequence: &str, default_at: u64) -> Vec<ScheduledFrontPanelAction> {
    sequence
        .split(',')
        .map(str::trim)
        .filter(|action| !action.is_empty())
        .map(|token| {
            let mut parts = token.splitn(2, '@');
            let action = parts.next().unwrap().trim().to_ascii_lowercase();
            let at = parts
                .next()
                .and_then(|value| value.trim().parse::<u64>().ok())
                .unwrap_or(default_at);
            let label = sanitize_snapshot_component(&action);
            ScheduledFrontPanelAction {
                action,
                at,
                label,
                sent: false,
                after_snapshot: false,
            }
        })
        .collect()
}

fn front_panel_sequence_label(actions: &[String]) -> String {
    if actions.is_empty() {
        return "none".to_string();
    }

    actions
        .iter()
        .map(|action| sanitize_snapshot_component(action))
        .collect::<Vec<_>>()
        .join("-")
}

fn sanitize_snapshot_component(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "unnamed".to_string()
    } else {
        trimmed.to_string()
    }
}

fn write_front_panel_snapshots(
    images: &[Rc<RefCell<crate::framebuffers::image::Image>>],
    snapshot_dir: &str,
    label: &str,
    instruction: u64,
) {
    if images.is_empty() {
        warn!(
            "front_panel_snapshot label={} instruction={} result=no_image_backend",
            label, instruction
        );
        return;
    }

    if let Err(e) = std::fs::create_dir_all(snapshot_dir) {
        warn!(
            "front_panel_snapshot label={} dir={} instruction={} result=create_dir_failed error={}",
            label, snapshot_dir, instruction, e
        );
        return;
    }

    let multi_image = images.len() > 1;
    for image in images {
        let image = image.borrow();
        let filename = if multi_image {
            format!(
                "{}-{}.png",
                label,
                sanitize_snapshot_component(&image.config.name)
            )
        } else {
            format!("{}.png", label)
        };
        let path = Path::new(snapshot_dir).join(filename);
        match image.write_to_path(&path) {
            Ok(()) => info!(
                "front_panel_snapshot label={} path={} instruction={} result=ok",
                label,
                path.display(),
                instruction
            ),
            Err(e) => warn!(
                "front_panel_snapshot label={} path={} instruction={} result=write_failed error={}",
                label,
                path.display(),
                instruction,
                e
            ),
        }
    }
}

fn maybe_run_front_panel_schedule(
    schedule: &mut [ScheduledFrontPanelAction],
    images: &[Rc<RefCell<crate::framebuffers::image::Image>>],
    snapshot_dir: Option<&str>,
    settle: u64,
    instruction: u64,
) {
    for scheduled in schedule {
        if !scheduled.sent && instruction >= scheduled.at {
            if let Some(snapshot_dir) = snapshot_dir {
                write_front_panel_snapshots(
                    images,
                    snapshot_dir,
                    &format!("before-{}", scheduled.label),
                    instruction,
                );
            }
            let accepted = crate::ext_devices::front_panel::script_action(&scheduled.action);
            info!(
                "front_panel_script event={} at_instruction={} result={}",
                scheduled.action,
                instruction,
                if accepted { "sent" } else { "ignored" }
            );
            scheduled.sent = true;
        }

        if scheduled.sent
            && !scheduled.after_snapshot
            && instruction >= scheduled.at.saturating_add(settle)
        {
            if let Some(snapshot_dir) = snapshot_dir {
                write_front_panel_snapshots(
                    images,
                    snapshot_dir,
                    &format!("after-{}", scheduled.label),
                    instruction,
                );
            }
            scheduled.after_snapshot = true;
        }
    }
}

pub fn dump_stack(uc: &mut Unicorn<()>, count: usize) {
    let mut sp = uc.reg_read(RegisterARM::SP).unwrap();

    for _ in 0..count {
        let mut v = [0,0,0,0];
        if uc.mem_read(sp, &mut v).is_err() {
            info!("stack dump finished due to mem read error");
            return;
        }
        let v = u32::from_le_bytes(v);

        if (0x0800_0000..0x0810_0000).contains(&v) {
            // Probably a return address
            info!("*** 0x{:08x} (sp=0x{:08x})", v, sp);
        } else {
            info!("    0x{:08x} (sp=0x{:08x})", v, sp);
        }

        sp += 4;
    }
}

pub fn run_emulator(config: Config, svd_device: SvdDevice, args: Args) -> Result<()> {
    let mut uc = Unicorn::new(Arch::ARM, Mode::MCLASS | Mode::LITTLE_ENDIAN)
        .map_err(UniErr)
        .context("Failed to initialize Unicorn instance")?;

    let vector_table_addr = config.cpu.vector_table;

    let (sys, framebuffers) = crate::system::prepare(&mut uc, config, svd_device)?;
    let sdl_framebuffers = framebuffers.sdls.clone();
    let image_framebuffers = framebuffers.images.clone();
    let front_panel_sequence = args.front_panel_sequence.clone().unwrap_or_default();
    let front_panel_timed = front_panel_sequence
        .split(',')
        .map(str::trim)
        .any(|token| token.contains('@'));
    let front_panel_actions = if front_panel_timed {
        Vec::new()
    } else {
        parse_front_panel_sequence(&front_panel_sequence)
    };
    let front_panel_schedule = if front_panel_timed {
        parse_front_panel_schedule(&front_panel_sequence, args.front_panel_at)
    } else {
        Vec::new()
    };
    let front_panel_label = front_panel_sequence_label(&front_panel_actions);
    if !front_panel_actions.is_empty() || !front_panel_schedule.is_empty() {
        info!(
            "front_panel_script_config sequence={} at_instruction={} settle_instructions={} snapshot_dir={} mode={}",
            front_panel_sequence,
            args.front_panel_at,
            args.front_panel_settle,
            args.front_panel_snapshot_dir.as_deref().unwrap_or("none"),
            if front_panel_timed { "timed" } else { "single" }
        );
    }

    let diassembler = Capstone::new()
        .arm()
        .mode(arch::arm::ArchMode::Thumb)
        .build()
        .expect("failed to initialize capstone");

    // We hook on each instructions, but we could skip this.
    // The slowdown is less than 50%. It's okay for now.
    let count_addrs = args.count_addr.clone();
    let trace_addrs = args.trace_addr.clone();
    let trace_addr_limit = args.trace_addr_limit;
    let addr_hits: Rc<RefCell<BTreeMap<u32, u64>>> = Rc::new(RefCell::new(BTreeMap::new()));
    let trace_hits: Rc<RefCell<BTreeMap<u32, u64>>> = Rc::new(RefCell::new(BTreeMap::new()));

    {
        let trace_instructions = crate::verbose() >= 4;
        let busy_loop_stop = args.busy_loop_stop;
        let p = sys.p.clone();
        let d = sys.d.clone();
        let interrupt_period = args.interrupt_period;
        let addr_hits_hook = addr_hits.clone();
        let trace_hits_hook = trace_hits.clone();
        let sdl_framebuffers_hook = sdl_framebuffers.clone();
        let has_sdl_framebuffers = !sdl_framebuffers_hook.is_empty();
        let image_framebuffers_hook = image_framebuffers.clone();
        let front_panel_snapshot_dir = args.front_panel_snapshot_dir.clone();
        let front_panel_at = args.front_panel_at;
        let front_panel_settle = args.front_panel_settle;
        let mut front_panel_sent = false;
        let mut front_panel_after_snapshot = false;
        let mut front_panel_schedule = front_panel_schedule;
        sys.uc
            .borrow_mut()
            .add_code_hook(0, u64::MAX, move |uc, pc, size| {
                unsafe {
                    if busy_loop_stop && LAST_INSTRUCTION.0 == pc as u32 {
                        info!("Busy loop reached");
                        uc.emu_stop().unwrap();
                        BUSY_LOOP_REACHED.store(true, Ordering::Release);
                    }
                    LAST_INSTRUCTION = (pc as u32, size as u8);
                }

                let n = NUM_INSTRUCTIONS.fetch_add(1, Ordering::Acquire);
                let pc32 = pc as u32;

                for addr in &count_addrs {
                    if *addr == pc32 {
                        let mut addr_hits = addr_hits_hook.borrow_mut();
                        *addr_hits.entry(pc32).or_insert(0) += 1;
                    }
                }

                for addr in &trace_addrs {
                    if *addr == pc32 {
                        let mut trace_hits = trace_hits_hook.borrow_mut();
                        let hit = trace_hits.entry(pc32).or_insert(0);
                        *hit += 1;
                        if *hit <= trace_addr_limit {
                            log_register_sample(uc, "trace", pc32, *hit);
                        }
                    }
                }

                if trace_instructions {
                    info!("{}", disassemble_instruction(&diassembler, uc, pc));
                }

                if !front_panel_schedule.is_empty() {
                    maybe_run_front_panel_schedule(
                        &mut front_panel_schedule,
                        &image_framebuffers_hook,
                        front_panel_snapshot_dir.as_deref(),
                        front_panel_settle,
                        n,
                    );
                } else if !front_panel_actions.is_empty()
                    && !front_panel_sent
                    && n >= front_panel_at
                {
                    if let Some(snapshot_dir) = front_panel_snapshot_dir.as_deref() {
                        write_front_panel_snapshots(
                            &image_framebuffers_hook,
                            snapshot_dir,
                            &format!("before-{}", front_panel_label),
                            n,
                        );
                    }
                    for action in &front_panel_actions {
                        let accepted = crate::ext_devices::front_panel::script_action(action);
                        info!(
                            "front_panel_script event={} at_instruction={} result={}",
                            action,
                            n,
                            if accepted { "sent" } else { "ignored" }
                        );
                    }
                    front_panel_sent = true;
                }

                if front_panel_schedule.is_empty()
                    && front_panel_sent
                    && !front_panel_after_snapshot
                    && n >= front_panel_at.saturating_add(front_panel_settle)
                {
                    if let Some(snapshot_dir) = front_panel_snapshot_dir.as_deref() {
                        write_front_panel_snapshots(
                            &image_framebuffers_hook,
                            snapshot_dir,
                            &format!("after-{}", front_panel_label),
                            n,
                        );
                    }
                    front_panel_after_snapshot = true;
                }

                if n % interrupt_period as u64 == 0 {
                    let sys = System {
                        uc: RefCell::new(uc),
                        p: p.clone(),
                        d: d.clone(),
                    };
                    p.nvic
                        .borrow_mut()
                        .run_pending_interrupts(&sys, vector_table_addr);
                }

                if n % PUMP_EVENT_INST_INTERVAL == 0 {
                    {
                        let sys = System {
                            uc: RefCell::new(uc),
                            p: p.clone(),
                            d: d.clone(),
                        };
                        crate::ext_devices::front_panel::tick(&sys);
                    }
                    if has_sdl_framebuffers {
                        for fb in &sdl_framebuffers_hook {
                            fb.borrow_mut().maybe_redraw();
                        }
                        if !SDL.lock().unwrap().pump_events(&sdl_framebuffers_hook) {
                            STOP_REQUESTED.store(true, Ordering::Relaxed);
                            uc.emu_stop().unwrap();
                        }
                    }
                }
            })
            .expect("add_code_hook failed");
    }

    {
        let p = sys.p.clone();
        let d = sys.d.clone();
        sys.uc
            .borrow_mut()
            .add_intr_hook(move |uc, exception| {
                match exception {
                    /*
                    EXCP_UDEF            1   /* undefined instruction */
                    EXCP_SWI             2   /* software interrupt */
                    EXCP_PREFETCH_ABORT  3
                    EXCP_DATA_ABORT      4
                    EXCP_IRQ             5
                    EXCP_FIQ             6
                    EXCP_BKPT            7
                    EXCP_EXCEPTION_EXIT  8   /* Return from v7M exception.  */
                    EXCP_KERNEL_TRAP     9   /* Jumped to kernel code page.  */
                    EXCP_HVC            11   /* HyperVisor Call */
                    EXCP_HYP_TRAP       12
                    EXCP_SMC            13   /* Secure Monitor Call */
                    EXCP_VIRQ           14
                    EXCP_VFIQ           15
                    EXCP_SEMIHOST       16   /* semihosting call */
                    EXCP_NOCP           17   /* v7M NOCP UsageFault */
                    EXCP_INVSTATE       18   /* v7M INVSTATE UsageFault */
                    EXCP_STKOF          19   /* v8M STKOF UsageFault */
                    EXCP_LAZYFP         20   /* v7M fault during lazy FP stacking */
                    EXCP_LSERR          21   /* v8M LSERR SecureFault */
                    EXCP_UNALIGNED      22   /* v7M UNALIGNED UsageFault */
                    */
                    8 => {
                        // Return from interrupt
                        let sys = System {
                            uc: RefCell::new(uc),
                            p: p.clone(),
                            d: d.clone(),
                        };
                        p.nvic.borrow_mut().return_from_interrupt(&sys);
                        p.nvic
                            .borrow_mut()
                            .run_pending_interrupts(&sys, vector_table_addr);
                    }
                    3 => {
                        error!("intr_hook intno={:08x}", exception);
                        STOP_REQUESTED.store(true, Ordering::Relaxed);
                        uc.emu_stop().unwrap();
                    }
                    _ => {
                        error!("intr_hook intno={:08x}", exception);
                        std::process::exit(1);
                    }
                }
            })
            .expect("add_intr_hook failed");
    }

    if !args.watch_write.is_empty() {
        let watch_addrs = args.watch_write.clone();
        let watch_write_limit = args.watch_write_limit;
        let watch_hits: Rc<RefCell<BTreeMap<u32, u64>>> = Rc::new(RefCell::new(BTreeMap::new()));
        let watch_hits_hook = watch_hits.clone();
        uc.add_mem_hook(
            HookType::MEM_WRITE,
            0,
            u64::MAX,
            move |uc, type_, addr, size, value| {
                if type_ != MemType::WRITE {
                    return true;
                }
                let write_start = addr as u32;
                let write_end = write_start.wrapping_add(size.saturating_sub(1) as u32);
                for watch_addr in &watch_addrs {
                    if *watch_addr < write_start || *watch_addr > write_end {
                        continue;
                    }
                    let mut watch_hits = watch_hits_hook.borrow_mut();
                    let hit = watch_hits.entry(*watch_addr).or_insert(0);
                    *hit += 1;
                    if *hit <= watch_write_limit {
                        let pc = uc.reg_read(RegisterARM::PC).unwrap_or(0) as u32;
                        info!(
                            "watch_write addr=0x{:08x} hit={} pc=0x{:08x} write_addr=0x{:08x} size={} value=0x{:08x} r0=0x{:08x} r1=0x{:08x} r2=0x{:08x} r3=0x{:08x} lr=0x{:08x} sp=0x{:08x}",
                            watch_addr,
                            *hit,
                            pc,
                            write_start,
                            size,
                            value as u32,
                            read_reg_or_zero(uc, RegisterARM::R0) as u32,
                            read_reg_or_zero(uc, RegisterARM::R1) as u32,
                            read_reg_or_zero(uc, RegisterARM::R2) as u32,
                            read_reg_or_zero(uc, RegisterARM::R3) as u32,
                            read_reg_or_zero(uc, RegisterARM::LR) as u32,
                            read_reg_or_zero(uc, RegisterARM::SP) as u32,
                        );
                    }
                }
                true
            },
        )
        .expect("add watch_write mem hook failed");
    }

    uc.add_mem_hook(
        HookType::MEM_UNMAPPED,
        0,
        u64::MAX,
        |uc, type_, addr, size, value| {
            if type_ == MemType::WRITE_UNMAPPED {
                warn!(
                    "{:?} addr=0x{:08x} size={} value=0x{:08x}",
                    type_, addr, size, value
                );
            } else {
                warn!("{:?} addr=0x{:08x} size={}", type_, addr, size);
            }

            unsafe {
                let pc = uc.reg_read(RegisterARM::PC).expect("failed to get pc");
                assert!(pc as u32 == LAST_INSTRUCTION.0);
                uc.reg_write(
                    RegisterARM::PC,
                    thumb(pc as u64 + LAST_INSTRUCTION.1 as u64),
                )
                .unwrap();
            }

            CONTINUE_EXECUTION.store(true, Ordering::Release);

            false
        },
    )
    .expect("add_mem_hook failed");

    let vector_table = VectorTable::from_memory(&uc, vector_table_addr)?;
    let mut pc = vector_table.reset as u64;
    uc.reg_write(RegisterARM::SP, vector_table.sp.into())
        .map_err(UniErr)?;
    //uc.reg_write(RegisterARM::LR, 0xFFFF_FFFF).map_err(UniErr)?;

    info!("Starting emulation");

    loop {
        let max_instructions = args.max_instructions.map(|c|
            // yes, we want to panic if this goes negative.
            c - NUM_INSTRUCTIONS.load(Ordering::Relaxed));
        if max_instructions == Some(0) {
            info!("Reached target number of instructions. Done");
            break;
        }

        let result = uc
            .emu_start(
                pc,
                args.stop_addr.unwrap_or(0) as u64,
                0,
                max_instructions.unwrap_or(0) as usize,
            )
            .map_err(UniErr);
        pc = uc.reg_read(RegisterARM::PC).expect("failed to get pc");

        if STOP_REQUESTED.load(Ordering::Relaxed) {
            info!("Stop requested");
            break;
        }

        if let Err(e) = result {
            if CONTINUE_EXECUTION.swap(false, Ordering::AcqRel) {
                // This was a bad memory access, we keep going.
                if crate::verbose() >= 3 {
                    trace!("Resuming execution pc={:08x}", pc);
                }
                pc = thumb(pc);
                continue;
            } else {
                bail!(e);
            }
        }

        if args.stop_addr == Some(pc as u32) {
            info!("Stop address reached, stopping");
            break;
        }

        if BUSY_LOOP_REACHED.load(Ordering::Relaxed) {
            break;
        }
    }

    if let Some(n) = args.dump_stack {
        dump_stack(&mut uc, n);
    }

    for addr in &args.dump_mem32 {
        let mut bytes = [0; 4];
        uc.mem_read((*addr).into(), &mut bytes).map_err(UniErr)?;
        info!(
            "mem32 addr=0x{:08x} value=0x{:08x}",
            addr,
            u32::from_le_bytes(bytes)
        );
    }

    {
        let addr_hits = addr_hits.borrow();
        for addr in &args.count_addr {
            info!(
                "count addr=0x{:08x} hits={}",
                addr,
                addr_hits.get(addr).copied().unwrap_or(0)
            );
        }
    }

    for fb in framebuffers.images {
        fb.borrow().write_to_disk()?;
    }

    Ok(())
}
