// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::atomic::Ordering;

use super::Peripheral;
use crate::system::System;

const CR1: u32 = 0x00;
const DIER: u32 = 0x0c;
const SR: u32 = 0x10;
const EGR: u32 = 0x14;
const CNT: u32 = 0x24;
const PSC: u32 = 0x28;
const ARR: u32 = 0x2c;
const UIF: u32 = 1 << 0;
const CEN: u32 = 1 << 0;

pub struct Timer {
    name: String,
    irq: Option<i32>,
    registers: [u32; 0x54 / 4],
    cnt: u32,
    last_instruction: u64,
    prescaler_remainder: u64,
}

impl Timer {
    pub fn new(name: &str) -> Option<Box<dyn Peripheral>> {
        if !name.starts_with("TIM") {
            return None;
        }

        let mut timer = Self {
            name: name.to_string(),
            irq: irq_for_timer(name),
            registers: [0; 0x54 / 4],
            cnt: 0,
            last_instruction: Self::instruction_count(),
            prescaler_remainder: 0,
        };
        timer.registers[(ARR / 4) as usize] = u32::MAX;

        Some(Box::new(timer))
    }

    fn instruction_count() -> u64 {
        crate::emulator::NUM_INSTRUCTIONS.load(Ordering::Relaxed)
    }

    fn reg_index(offset: u32) -> Option<usize> {
        if offset as usize >= 0x54 || offset % 4 != 0 {
            None
        } else {
            Some(offset as usize / 4)
        }
    }

    fn prescaler_divisor(&self) -> u64 {
        (self.registers[(PSC / 4) as usize] as u64)
            .saturating_add(1)
            .max(1)
    }

    fn reload_value(&self) -> u32 {
        let arr = self.registers[(ARR / 4) as usize];
        if arr == 0 {
            u32::MAX
        } else {
            arr
        }
    }

    fn set_update_event(&mut self, sys: &System) {
        self.registers[(SR / 4) as usize] |= UIF;
        if self.registers[(DIER / 4) as usize] & UIF != 0 {
            if let Some(irq) = self.irq {
                sys.p.nvic.borrow_mut().set_intr_pending(irq);
            }
        }
    }

    fn advance_counter(&mut self, sys: &System) {
        let now = Self::instruction_count();
        let elapsed = now.saturating_sub(self.last_instruction);
        self.last_instruction = now;

        if self.registers[(CR1 / 4) as usize] & CEN == 0 || elapsed == 0 {
            return;
        }

        let divisor = self.prescaler_divisor();
        let total = self.prescaler_remainder.saturating_add(elapsed);
        let ticks = total / divisor;
        self.prescaler_remainder = total % divisor;
        if ticks == 0 {
            return;
        }

        let reload = self.reload_value() as u64;
        let old = self.cnt as u64;
        let next = old.saturating_add(ticks);
        if next > reload {
            self.set_update_event(sys);
            self.cnt = (next % (reload + 1)) as u32;
        } else {
            self.cnt = next as u32;
        }
    }
}

impl Peripheral for Timer {
    fn read(&mut self, sys: &System, offset: u32) -> u32 {
        self.advance_counter(sys);

        match offset {
            CNT => self.cnt,
            _ => Self::reg_index(offset)
                .map(|i| self.registers[i])
                .unwrap_or(0),
        }
    }

    fn write(&mut self, sys: &System, offset: u32, value: u32) {
        self.advance_counter(sys);

        match offset {
            CR1 | DIER | ARR => {
                if let Some(i) = Self::reg_index(offset) {
                    self.registers[i] = value;
                }
            }
            SR => {
                self.registers[(SR / 4) as usize] = value;
            }
            EGR => {
                self.registers[(EGR / 4) as usize] = value;
                if value & UIF != 0 {
                    self.set_update_event(sys);
                }
            }
            CNT => {
                self.cnt = value;
                self.prescaler_remainder = 0;
                self.last_instruction = Self::instruction_count();
            }
            PSC => {
                self.registers[(PSC / 4) as usize] = value;
                self.prescaler_remainder = 0;
            }
            _ => {
                if let Some(i) = Self::reg_index(offset) {
                    self.registers[i] = value;
                } else {
                    debug!(
                        "{} ignored write offset=0x{:04x} value=0x{:08x}",
                        self.name, offset, value
                    );
                }
            }
        }
    }
}

fn irq_for_timer(name: &str) -> Option<i32> {
    match name {
        "TIM2" => Some(28),
        "TIM3" => Some(29),
        "TIM4" => Some(30),
        "TIM5" => Some(50),
        "TIM6" => Some(54),
        "TIM7" => Some(55),
        _ => None,
    }
}
