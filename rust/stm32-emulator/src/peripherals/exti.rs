// SPDX-License-Identifier: GPL-3.0-or-later

use super::{gpio::Pin, syscfg::Syscfg, Peripheral};
use crate::system::System;

#[derive(Default)]
pub struct Exti {
    imr: u32,
    emr: u32,
    rtsr: u32,
    ftsr: u32,
    swier: u32,
    pr: u32,
    line_levels: [Option<bool>; 16],
}

impl Exti {
    pub fn signal_gpio_input(&mut self, sys: &System, syscfg: &Syscfg, pin: Pin, level: bool) {
        let line = pin.pin();
        if line >= 16 {
            return;
        }

        let configured_port = syscfg.exti_port(line);
        if configured_port != pin.port() {
            debug!(
                "exti_event pin={} line={} level={} result=wrong_port configured_port={}",
                pin.name(),
                line,
                level,
                configured_port
            );
            return;
        }

        let previous = self.line_levels[line as usize].unwrap_or(!level);
        self.line_levels[line as usize] = Some(level);

        let rising = !previous && level;
        let falling = previous && !level;
        let line_mask = 1u32 << line;
        let edge_enabled =
            (rising && self.rtsr & line_mask != 0) || (falling && self.ftsr & line_mask != 0);

        if edge_enabled {
            self.trigger_line(sys, line, if rising { "rising" } else { "falling" });
        } else {
            debug!(
                "exti_event pin={} line={} level={} previous={} result=edge_masked rtsr=0x{:08x} ftsr=0x{:08x}",
                pin.name(),
                line,
                level,
                previous,
                self.rtsr,
                self.ftsr
            );
        }
    }

    fn irq_for_line(line: u8) -> i32 {
        match line {
            0 => 6,
            1 => 7,
            2 => 8,
            3 => 9,
            4 => 10,
            5..=9 => 23,
            _ => 40,
        }
    }

    fn trigger_line(&mut self, sys: &System, line: u8, edge: &str) {
        let line_mask = 1u32 << line;
        self.pr |= line_mask;
        self.swier &= !line_mask;

        if self.imr & line_mask != 0 {
            let irq = Self::irq_for_line(line);
            info!(
                "exti_event line={} edge={} irq={} imr=0x{:08x} result=pending",
                line, edge, irq, self.imr
            );
            sys.p.nvic.borrow_mut().set_intr_pending(irq);
        } else {
            debug!(
                "exti_event line={} edge={} imr=0x{:08x} result=interrupt_masked",
                line, edge, self.imr
            );
        }
    }

    fn read(&mut self, offset: u32) -> u32 {
        match offset {
            0x0000 => self.imr,
            0x0004 => self.emr,
            0x0008 => self.rtsr,
            0x000c => self.ftsr,
            0x0010 => self.swier,
            0x0014 => self.pr,
            _ => {
                warn!("EXTI invalid offset=0x{:08x}", offset);
                0
            }
        }
    }

    fn write(&mut self, sys: &System, offset: u32, value: u32) {
        match offset {
            0x0000 => self.imr = value,
            0x0004 => self.emr = value,
            0x0008 => self.rtsr = value,
            0x000c => self.ftsr = value,
            0x0010 => {
                self.swier |= value;
                for line in 0..16 {
                    if value & (1u32 << line) != 0 {
                        self.trigger_line(sys, line, "software");
                    }
                }
            }
            0x0014 => self.pr &= !value,
            _ => warn!("EXTI invalid offset=0x{:08x}", offset),
        }
    }
}

pub struct ExtiWrapper;

impl ExtiWrapper {
    pub fn new(name: &str) -> Option<Box<dyn Peripheral>> {
        if name == "EXTI" {
            Some(Box::new(Self))
        } else {
            None
        }
    }
}

impl Peripheral for ExtiWrapper {
    fn read(&mut self, sys: &System, offset: u32) -> u32 {
        sys.p.exti.borrow_mut().read(offset)
    }

    fn write(&mut self, sys: &System, offset: u32, value: u32) {
        sys.p.exti.borrow_mut().write(sys, offset, value);
    }
}
