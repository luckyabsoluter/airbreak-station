// SPDX-License-Identifier: GPL-3.0-or-later

use super::Peripheral;
use crate::system::System;

#[derive(Default)]
pub struct I2c {
    name: String,
    cr1: u32,
    cr2: u32,
    sr1: u32,
    sr2: u32,
    dr: u32,
    event_irq: Option<i32>,
}

impl I2c {
    pub fn new(name: &str) -> Option<Box<dyn Peripheral>> {
        if name.starts_with("I2C") {
            let name = name.to_string();
            let event_irq = match name.as_str() {
                "I2C1" => Some(31),
                "I2C2" => Some(33),
                "I2C3" => Some(72),
                _ => None,
            };
            Some(Box::new(Self {
                name,
                event_irq,
                ..I2c::default()
            }))
        } else {
            None
        }
    }

    fn event_interrupt_enabled(&self) -> bool {
        // ITEVTEN or ITBUFEN in CR2.
        self.cr2 & 0x600 != 0
    }

    fn request_event(&self, sys: &System) {
        if self.event_interrupt_enabled() {
            if let Some(irq) = self.event_irq {
                sys.p.nvic.borrow_mut().set_intr_pending(irq);
            }
        }
    }
}

impl Peripheral for I2c {
    fn read(&mut self, sys: &System, offset: u32) -> u32 {
        match offset {
            0x0000 => self.cr1,
            0x0004 => self.cr2,
            0x0010 => {
                // DR
                debug!("{} READ", self.name);
                self.sr1 |= 0x84;
                self.request_event(sys);
                self.dr
            }
            0x0014 => {
                // SR1
                self.sr1
            }
            0x0018 => {
                // SR2
                self.sr1 &= !0x2;
                self.sr2
            }
            _ => 0,
        }
    }

    fn write(&mut self, sys: &System, offset: u32, value: u32) {
        match offset {
            0x0000 => {
                self.cr1 = value;
                if value & 0x100 != 0 {
                    // START condition: SB plus BTF are enough for the firmware's
                    // interrupt-driven transfer state machine to start feeding DR.
                    self.sr1 |= 0x5;
                    self.request_event(sys);
                }
                if value & 0x200 != 0 {
                    self.sr2 &= !0x2;
                }
            }
            0x0004 => {
                self.cr2 = value;
                if self.sr1 != 0 {
                    self.request_event(sys);
                }
            }
            0x0010 => {
                self.dr = value & 0xff;
                self.sr1 &= !0x1;
                self.sr1 |= 0x84;
                debug!("{} WRITE value=0x{:08x}", self.name, value);
                self.request_event(sys);
            }
            0x0014 => {
                self.sr1 = value;
            }
            _ => {}
        }
    }
}
