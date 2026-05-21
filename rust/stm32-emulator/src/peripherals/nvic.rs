// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::atomic::Ordering;

use unicorn_engine::{RegisterARM, Unicorn};

use super::Peripheral;
use crate::system::System;

pub struct Nvic {
    pub vector_table_addr: u32,
    pub last_systick_trigger: u64,

    systick_ctrl: u32,
    systick_load: u32,
    scs_regs: [u32; 1024],

    // 128 exception slots is enough for the STM32F405 external IRQ range.
    enabled: u128,
    pending: u128,
    active: u128,
    in_interrupt: bool,
}

impl Default for Nvic {
    fn default() -> Self {
        Self {
            vector_table_addr: 0,
            last_systick_trigger: 0,
            systick_ctrl: 0,
            systick_load: 0,
            scs_regs: [0; 1024],
            enabled: 0,
            pending: 0,
            active: 0,
            in_interrupt: false,
        }
    }
}

const IRQ_OFFSET: i32 = 16;

pub mod irq {
    pub const PENDSV: i32 = -2;
    pub const SYSTICK: i32 = -1;
}

// This is all poorly implemented. If this is not making much sense, it might be
// best to re-implement everything correctly. Right now, I'm just trying to get
// the saturn firmware to work just well enough.

impl Nvic {
    const SYSTICK_ENABLE: u32 = 1 << 0;
    const SYSTICK_TICKINT: u32 = 1 << 1;
    const SYSTICK_CLKSOURCE: u32 = 1 << 2;
    const SYSTICK_COUNTFLAG: u32 = 1 << 16;

    fn exception_number(irq: i32) -> u32 {
        (IRQ_OFFSET + irq) as u32
    }

    fn irq_from_exception(exception: u32) -> i32 {
        exception as i32 - IRQ_OFFSET
    }

    fn exception_bit(exception: u32) -> u128 {
        1u128 << exception
    }

    fn scs_index(offset: u32) -> Option<usize> {
        let index = (offset / 4) as usize;
        if offset % 4 == 0 && index < 1024 {
            Some(index)
        } else {
            None
        }
    }

    fn scb_scs_index(offset: u32) -> Option<usize> {
        Self::scs_index(0x0d00 + offset)
    }

    fn nvic_word_index(offset: u32, base: u32) -> Option<u32> {
        if offset >= base && offset < base + 0x0c && offset % 4 == 0 {
            Some((offset - base) / 4)
        } else {
            None
        }
    }

    fn external_word(mask: u128, word_index: u32) -> u32 {
        ((mask >> (IRQ_OFFSET as u32 + word_index * 32)) & 0xffff_ffff) as u32
    }

    fn set_external_word(mask: &mut u128, word_index: u32, value: u32) {
        *mask |= (value as u128) << (IRQ_OFFSET as u32 + word_index * 32);
    }

    fn clear_external_word(mask: &mut u128, word_index: u32, value: u32) {
        *mask &= !((value as u128) << (IRQ_OFFSET as u32 + word_index * 32));
    }

    pub fn set_intr_pending(&mut self, irq: i32) {
        trace!("Set irq pending irq={}", irq);
        let bit = Self::exception_number(irq);
        assert!(bit > 0);
        self.pending |= Self::exception_bit(bit);
    }

    fn clear_intr_pending(&mut self, irq: i32) {
        let bit = Self::exception_number(irq);
        self.pending &= !Self::exception_bit(bit);
    }

    fn priority(&self, exception: u32) -> u8 {
        match exception {
            4..=6 => ((self.scs_regs[(0x0d18 / 4) as usize] >> ((exception - 4) * 8)) & 0xff) as u8,
            11 => ((self.scs_regs[(0x0d1c / 4) as usize] >> 24) & 0xff) as u8,
            14..=15 => {
                ((self.scs_regs[(0x0d20 / 4) as usize] >> ((exception - 12) * 8)) & 0xff) as u8
            }
            16..=127 => {
                let irq = exception - IRQ_OFFSET as u32;
                let offset = 0x0400 + (irq / 4) * 4;
                let shift = (irq % 4) * 8;
                ((self.scs_regs[(offset / 4) as usize] >> shift) & 0xff) as u8
            }
            _ => 0,
        }
    }

    fn is_exception_enabled(&self, exception: u32) -> bool {
        exception < IRQ_OFFSET as u32 || (self.enabled & Self::exception_bit(exception)) != 0
    }

    fn is_exception_masked(&self, sys: &System, exception: u32) -> bool {
        let uc = sys.uc.borrow();
        let primask = uc.reg_read(RegisterARM::PRIMASK).unwrap();
        let faultmask = uc.reg_read(RegisterARM::FAULTMASK).unwrap();
        let basepri = uc.reg_read(RegisterARM::BASEPRI).unwrap_or(0) as u8;
        drop(uc);

        if faultmask != 0 {
            return true;
        }
        if primask != 0 && exception >= 14 {
            return true;
        }
        basepri != 0 && self.priority(exception) >= basepri
    }

    fn pending_priority_key(&self, exception: u32) -> (u8, u32) {
        (self.priority(exception), exception)
    }

    fn next_pending_exception(&self, sys: &System) -> Option<u32> {
        if self.pending == 0 {
            return None;
        }

        let mut selected = None;
        for exception in 1u32..128 {
            if self.pending & Self::exception_bit(exception) == 0 {
                continue;
            }
            if !self.is_exception_enabled(exception) || self.is_exception_masked(sys, exception) {
                continue;
            }
            if selected.map_or(true, |current| {
                self.pending_priority_key(exception) < self.pending_priority_key(current)
            }) {
                selected = Some(exception);
            }
        }

        selected
    }

    pub fn get_and_clear_next_intr_pending(&mut self, sys: &System) -> Option<i32> {
        let selected = self.next_pending_exception(sys);

        if let Some(bit) = selected {
            self.pending &= !Self::exception_bit(bit);
            Some(Self::irq_from_exception(bit))
        } else {
            None
        }
    }

    pub fn maybe_set_systick_intr_pending(&mut self) {
        if self.systick_ctrl & (Self::SYSTICK_ENABLE | Self::SYSTICK_TICKINT)
            == (Self::SYSTICK_ENABLE | Self::SYSTICK_TICKINT)
        {
            let systick_period = self.systick_load.wrapping_add(1);
            let n = crate::emulator::NUM_INSTRUCTIONS.load(Ordering::Relaxed);
            let delta_num_instructions = n - self.last_systick_trigger;
            if delta_num_instructions >= (systick_period as u64) {
                self.last_systick_trigger = n;
                self.systick_ctrl |= Self::SYSTICK_COUNTFLAG;
                self.set_intr_pending(irq::SYSTICK);
            }
        }
    }

    pub fn configure_systick(&mut self, ctrl: u32, load: u32) {
        self.systick_ctrl =
            ctrl & (Self::SYSTICK_ENABLE | Self::SYSTICK_TICKINT | Self::SYSTICK_CLKSOURCE);
        self.systick_load = load & 0x00ff_ffff;
        self.last_systick_trigger = crate::emulator::NUM_INSTRUCTIONS.load(Ordering::Relaxed);
    }

    pub fn run_pending_interrupts(&mut self, sys: &System, vector_table_addr: u32) {
        self.maybe_set_systick_intr_pending();

        if self.in_interrupt {
            return;
        }

        if let Some(irq) = self.get_and_clear_next_intr_pending(sys) {
            let vector_table_addr = if self.vector_table_addr != 0 {
                self.vector_table_addr
            } else {
                vector_table_addr
            };
            self.run_interrupt(sys, vector_table_addr, irq);
        }
    }

    fn read_vector_addr(sys: &System, vector_table_addr: u32, irq: i32) -> u32 {
        // 4 because of ptr size
        let vaddr = vector_table_addr + 4 * (IRQ_OFFSET + irq) as u32;

        let mut vector = [0, 0, 0, 0];
        sys.uc.borrow().mem_read(vaddr as u64, &mut vector).unwrap();
        u32::from_le_bytes(vector)
    }

    // SPSEL, bit[1], 0 means we use MSP, 1 means we use PSP.
    // FPCA, bit[2], if the processor includes the FP extension.

    fn run_interrupt(&mut self, sys: &System, vector_table_addr: u32, irq: i32) {
        let vector = Self::read_vector_addr(sys, vector_table_addr, irq);

        let mut uc = sys.uc.borrow_mut();

        // SPSEL, bit[1], 0 means we use MSP, 1 means we use PSP.
        // FPCA, bit[2], if the processor includes the FP extension.
        let control_reg = uc.reg_read(RegisterARM::CONTROL).unwrap();
        let spsel = control_reg & (1 << 1) != 0;
        let fpca = control_reg & (2 << 1) != 0;

        trace!(
            "Running interrupt irq={} spsel={} fpca={} vector={:#08x}",
            irq,
            spsel,
            fpca,
            vector
        );

        Self::push_regs(&mut uc, spsel, fpca);

        // LR meaning:
        //   EXC_RETURN    Return to      Return stack Frame type
        //   0xFFFF_FFE1   Handler mode   Main         Extended
        //   0xFFFF_FFE9   Thread mode    Main         Extended
        //   0xFFFF_FFED   Thread mode    Process      Extended
        //   0xFFFF_FFF1   Handler mode   Main         Basic
        //   0xFFFF_FFF9   Thread mode    Main         Basic
        //   0xFFFF_FFFD   Thread mode    Process      Basic

        // Right now, we don't supposed nested interrupts.
        let mut lr: u32 = 0xFFFF_FFE9;
        if spsel {
            lr |= 0b0000_0100;
        }
        if !fpca {
            lr |= 0b0001_0000;
        } // Yes, no fpca means the bit is set
        uc.reg_write(RegisterARM::LR, lr.into()).unwrap();

        uc.reg_write(RegisterARM::IPSR, Self::exception_number(irq).into())
            .unwrap();
        Self::sync_sp_alias(&mut uc, false);
        uc.reg_write(RegisterARM::PC, vector as u64).unwrap();

        self.active |= Self::exception_bit(Self::exception_number(irq));
        self.in_interrupt = true;
    }

    pub fn return_from_interrupt(&mut self, sys: &System) {
        let mut uc = sys.uc.borrow_mut();
        let active_exception = uc.reg_read(RegisterARM::IPSR).unwrap() as u32 & 0x1ff;

        let lr = uc.reg_read(RegisterARM::LR).unwrap();
        let return_spsel;
        if lr & 0xFFFF_FF00 == 0xFFFF_FF00 {
            let spsel = lr & 0b0000_0100 != 0;
            let fpca = lr & 0b0001_0000 == 0; // 0 means yes here
            return_spsel = spsel;

            Self::pop_regs(&mut uc, spsel, fpca);

            trace!(
                "Return from interrupt spsel={} fpca={} pc=0x{:08x}",
                spsel,
                fpca,
                uc.reg_read(RegisterARM::PC).unwrap()
            );

            // SPSEL, bit[1], 0 means we use MSP, 1 means we use PSP.
            // FPCA, bit[2], if the processor includes the FP extension.
            let mut control_reg = 0;
            if spsel {
                control_reg |= 1 << 1;
            }
            if fpca {
                control_reg |= 2 << 1;
            }
            uc.reg_write(RegisterARM::CONTROL, control_reg).unwrap();
        } else {
            let control_reg = uc.reg_read(RegisterARM::CONTROL).unwrap();
            let spsel = control_reg & (1 << 1) != 0;
            let fpca = control_reg & (2 << 1) != 0;
            return_spsel = spsel;
            Self::pop_regs(&mut uc, spsel, fpca);

            trace!(
                "Return from interrupt spsel={} fpca={} pc=0x{:08x} -- LR was not right",
                spsel,
                fpca,
                uc.reg_read(RegisterARM::PC).unwrap()
            );
        }

        self.in_interrupt = false;
        uc.reg_write(RegisterARM::IPSR, 0).unwrap();
        Self::sync_sp_alias(&mut uc, return_spsel);
        if active_exception != 0 {
            self.active &= !Self::exception_bit(active_exception);
        }
    }

    fn read_icsr(&self, sys: &System) -> u32 {
        let ipsr = sys.uc.borrow().reg_read(RegisterARM::IPSR).unwrap() as u32 & 0x1ff;
        let pending_exception = self.next_pending_exception(sys);

        let mut value = ipsr;
        if let Some(exception) = pending_exception {
            value |= (exception & 0x1ff) << 12;
            if exception >= IRQ_OFFSET as u32 {
                value |= 1 << 22;
            }
        }
        if self.pending & Self::exception_bit(Self::exception_number(irq::SYSTICK)) != 0 {
            value |= 1 << 26;
        }
        if self.pending & Self::exception_bit(Self::exception_number(irq::PENDSV)) != 0 {
            value |= 1 << 28;
        }
        value
    }

    fn write_icsr(&mut self, value: u32) {
        if value & (1 << 25) != 0 {
            self.clear_intr_pending(irq::SYSTICK);
        }
        if value & (1 << 26) != 0 {
            self.set_intr_pending(irq::SYSTICK);
        }
        if value & (1 << 27) != 0 {
            self.clear_intr_pending(irq::PENDSV);
        }
        if value & (1 << 28) != 0 {
            self.set_intr_pending(irq::PENDSV);
        }
    }

    pub fn read_scb(&mut self, sys: &System, offset: u32) -> u32 {
        match offset {
            0x0004 => self.read_icsr(sys),
            0x0008 => self.vector_table_addr,
            _ => Self::scb_scs_index(offset)
                .map(|index| self.scs_regs[index])
                .unwrap_or(0),
        }
    }

    pub fn write_scb(&mut self, _sys: &System, offset: u32, value: u32) {
        match offset {
            0x0004 => self.write_icsr(value),
            0x0008 => {
                self.vector_table_addr = value;
            }
            _ => {
                if let Some(index) = Self::scb_scs_index(offset) {
                    self.scs_regs[index] = value;
                }
            }
        }
    }

    const CONTEXT_REGS_EXTENDED: [RegisterARM; 17] = [
        RegisterARM::FPSCR,
        RegisterARM::S15,
        RegisterARM::S14,
        RegisterARM::S13,
        RegisterARM::S12,
        RegisterARM::S11,
        RegisterARM::S10,
        RegisterARM::S9,
        RegisterARM::S8,
        RegisterARM::S7,
        RegisterARM::S6,
        RegisterARM::S5,
        RegisterARM::S4,
        RegisterARM::S3,
        RegisterARM::S2,
        RegisterARM::S1,
        RegisterARM::S0,
    ];

    const CONTEXT_REGS: [RegisterARM; 8] = [
        RegisterARM::XPSR,
        RegisterARM::PC,
        RegisterARM::LR,
        RegisterARM::R12,
        RegisterARM::R3,
        RegisterARM::R2,
        RegisterARM::R1,
        RegisterARM::R0,
    ];

    fn push_regs(uc: &mut Unicorn<()>, spsel: bool, fpca: bool) {
        let sp_reg = Self::stack_pointer_reg(spsel);
        let mut sp = uc.reg_read(sp_reg).unwrap();

        let mut push_reg = |reg| {
            let v = uc.reg_read(reg).unwrap() as u32;
            //trace!("push sp=0x{:08x} {:5?}=0x{:08x}", sp, reg, v);
            sp -= 4;
            uc.mem_write(sp, &v.to_le_bytes())
                .expect("Invalid SP pointer during interrupt");
        };

        if fpca {
            for reg in Self::CONTEXT_REGS_EXTENDED {
                push_reg(reg);
            }
        }
        for reg in Self::CONTEXT_REGS {
            push_reg(reg);
        }
        uc.reg_write(sp_reg, sp).unwrap();
    }

    fn pop_regs(uc: &mut Unicorn<()>, spsel: bool, fpca: bool) {
        let sp_reg = Self::stack_pointer_reg(spsel);
        let mut sp = uc.reg_read(sp_reg).unwrap();

        let mut pop_reg = |reg| {
            let mut v = [0, 0, 0, 0];
            uc.mem_read(sp, &mut v)
                .expect("Invalid SP pointer during interrupt return");
            let v = u32::from_le_bytes(v);
            //trace!("pop sp=0x{:08x} {:5?}=0x{:08x}", sp, reg, v);
            sp += 4;
            uc.reg_write(reg, v as u64).unwrap();
        };

        for reg in Self::CONTEXT_REGS.iter().rev() {
            pop_reg(*reg);
        }
        if fpca {
            for reg in Self::CONTEXT_REGS_EXTENDED.iter().rev() {
                pop_reg(*reg);
            }
        }
        uc.reg_write(sp_reg, sp).unwrap();
    }

    fn stack_pointer_reg(spsel: bool) -> RegisterARM {
        if spsel {
            RegisterARM::PSP
        } else {
            RegisterARM::MSP
        }
    }

    fn sync_sp_alias(uc: &mut Unicorn<()>, spsel: bool) {
        let sp = uc.reg_read(Self::stack_pointer_reg(spsel)).unwrap();
        uc.reg_write(RegisterARM::SP, sp).unwrap();
    }
}

impl Peripheral for Nvic {
    fn read(&mut self, sys: &System, offset: u32) -> u32 {
        match offset {
            0x0010 => {
                let value = self.systick_ctrl;
                self.systick_ctrl &= !Self::SYSTICK_COUNTFLAG;
                value
            }
            0x0014 => self.systick_load,
            0x0018 => {
                if self.systick_ctrl & Self::SYSTICK_ENABLE == 0 || self.systick_load == 0 {
                    0
                } else {
                    let n = crate::emulator::NUM_INSTRUCTIONS.load(Ordering::Relaxed);
                    let period = self.systick_load.wrapping_add(1) as u64;
                    let elapsed = n.saturating_sub(self.last_systick_trigger) % period;
                    self.systick_load.saturating_sub(elapsed as u32)
                }
            }
            0x0d04 => self.read_icsr(sys),
            0x0d08 => self.vector_table_addr,
            _ => Self::scs_index(offset)
                .map(|index| {
                    if let Some(word_index) = Self::nvic_word_index(offset, 0x0100) {
                        Self::external_word(self.enabled, word_index)
                    } else if let Some(word_index) = Self::nvic_word_index(offset, 0x0180) {
                        Self::external_word(self.enabled, word_index)
                    } else if let Some(word_index) = Self::nvic_word_index(offset, 0x0200) {
                        Self::external_word(self.pending, word_index)
                    } else if let Some(word_index) = Self::nvic_word_index(offset, 0x0280) {
                        Self::external_word(self.pending, word_index)
                    } else if let Some(word_index) = Self::nvic_word_index(offset, 0x0300) {
                        Self::external_word(self.active, word_index)
                    } else {
                        self.scs_regs[index]
                    }
                })
                .unwrap_or(0),
        }
    }

    fn write(&mut self, _sys: &System, offset: u32, value: u32) {
        match offset {
            0x0010 => {
                // SysTick CTRL inside System Control Space.
                self.systick_ctrl = value
                    & (Self::SYSTICK_ENABLE | Self::SYSTICK_TICKINT | Self::SYSTICK_CLKSOURCE);
                self.last_systick_trigger =
                    crate::emulator::NUM_INSTRUCTIONS.load(Ordering::Relaxed);
            }
            0x0014 => {
                // SysTick LOAD is a 24-bit reload value.
                self.systick_load = value & 0x00ff_ffff;
                self.last_systick_trigger =
                    crate::emulator::NUM_INSTRUCTIONS.load(Ordering::Relaxed);
            }
            0x0018 => {
                // Writing SysTick VAL clears the current count and COUNTFLAG.
                self.systick_ctrl &= !Self::SYSTICK_COUNTFLAG;
                self.last_systick_trigger =
                    crate::emulator::NUM_INSTRUCTIONS.load(Ordering::Relaxed);
            }
            0x0d04 => self.write_icsr(value),
            0x0d08 => {
                // VTOR inside System Control Space.
                self.vector_table_addr = value;
            }
            _ => {
                if let Some(word_index) = Self::nvic_word_index(offset, 0x0100) {
                    Self::set_external_word(&mut self.enabled, word_index, value);
                } else if let Some(word_index) = Self::nvic_word_index(offset, 0x0180) {
                    Self::clear_external_word(&mut self.enabled, word_index, value);
                } else if let Some(word_index) = Self::nvic_word_index(offset, 0x0200) {
                    Self::set_external_word(&mut self.pending, word_index, value);
                } else if let Some(word_index) = Self::nvic_word_index(offset, 0x0280) {
                    Self::clear_external_word(&mut self.pending, word_index, value);
                } else if let Some(index) = Self::scs_index(offset) {
                    self.scs_regs[index] = value;
                }
            }
        }
    }
}

/// The next part is glue. Maybe we could have a better architecture.

pub struct NvicWrapper;

impl NvicWrapper {
    pub fn new(name: &str) -> Option<Box<dyn Peripheral>> {
        if name == "NVIC" {
            Some(Box::new(Self))
        } else {
            None
        }
    }
}

impl Peripheral for NvicWrapper {
    fn read(&mut self, sys: &System, offset: u32) -> u32 {
        sys.p.nvic.borrow_mut().read(sys, offset)
    }

    fn write(&mut self, sys: &System, offset: u32, value: u32) {
        sys.p.nvic.borrow_mut().write(sys, offset, value)
    }
}

/*
0xE000E100 B  REGISTER ISER0 (rw): Interrupt Set-Enable Register
0xE000E104 B  REGISTER ISER1 (rw): Interrupt Set-Enable Register
0xE000E108 B  REGISTER ISER2 (rw): Interrupt Set-Enable Register

0xE000E180 B  REGISTER ICER0 (rw): Interrupt Clear-Enable Register
0xE000E184 B  REGISTER ICER1 (rw): Interrupt Clear-Enable Register
0xE000E188 B  REGISTER ICER2 (rw): Interrupt Clear-Enable Register

0xE000E200 B  REGISTER ISPR0 (rw): Interrupt Set-Pending Register
0xE000E204 B  REGISTER ISPR1 (rw): Interrupt Set-Pending Register
0xE000E208 B  REGISTER ISPR2 (rw): Interrupt Set-Pending Register

0xE000E280 B  REGISTER ICPR0 (rw): Interrupt Clear-Pending Register
0xE000E284 B  REGISTER ICPR1 (rw): Interrupt Clear-Pending Register
0xE000E288 B  REGISTER ICPR2 (rw): Interrupt Clear-Pending Register

0xE000E300 B  REGISTER IABR0 (ro): Interrupt Active Bit Register
0xE000E304 B  REGISTER IABR1 (ro): Interrupt Active Bit Register
0xE000E308 B  REGISTER IABR2 (ro): Interrupt Active Bit Register

0xE000E400 B  REGISTER IPR0 (rw): Interrupt Priority Register
0xE000E404 B  REGISTER IPR1 (rw): Interrupt Priority Register
0xE000E408 B  REGISTER IPR2 (rw): Interrupt Priority Register
0xE000E40C B  REGISTER IPR3 (rw): Interrupt Priority Register
0xE000E410 B  REGISTER IPR4 (rw): Interrupt Priority Register
0xE000E414 B  REGISTER IPR5 (rw): Interrupt Priority Register
0xE000E418 B  REGISTER IPR6 (rw): Interrupt Priority Register
0xE000E41C B  REGISTER IPR7 (rw): Interrupt Priority Register
0xE000E420 B  REGISTER IPR8 (rw): Interrupt Priority Register
0xE000E424 B  REGISTER IPR9 (rw): Interrupt Priority Register
0xE000E428 B  REGISTER IPR10 (rw): Interrupt Priority Register
0xE000E42C B  REGISTER IPR11 (rw): Interrupt Priority Register
0xE000E430 B  REGISTER IPR12 (rw): Interrupt Priority Register
0xE000E434 B  REGISTER IPR13 (rw): Interrupt Priority Register
0xE000E438 B  REGISTER IPR14 (rw): Interrupt Priority Register
0xE000E43C B  REGISTER IPR15 (rw): Interrupt Priority Register
0xE000E440 B  REGISTER IPR16 (rw): Interrupt Priority Register
0xE000E444 B  REGISTER IPR17 (rw): Interrupt Priority Register
0xE000E448 B  REGISTER IPR18 (rw): Interrupt Priority Register
0xE000E44C B  REGISTER IPR19 (rw): Interrupt Priority Register
*/
