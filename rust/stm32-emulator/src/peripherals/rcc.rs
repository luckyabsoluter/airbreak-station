// SPDX-License-Identifier: GPL-3.0-or-later

use super::Peripheral;
use crate::system::System;

pub struct Rcc {
    registers: [u32; 0x90 / 4],
}

impl Default for Rcc {
    fn default() -> Self {
        Self {
            registers: [0; 0x90 / 4],
        }
    }
}

impl Rcc {
    pub fn new(name: &str) -> Option<Box<dyn Peripheral>> {
        if name == "RCC" {
            Some(Box::<Rcc>::default())
        } else {
            None
        }
    }

    fn reg_index(offset: u32) -> Option<usize> {
        if offset as usize >= 0x90 || offset % 4 != 0 {
            None
        } else {
            Some(offset as usize / 4)
        }
    }
}

impl Peripheral for Rcc {
    fn read(&mut self, _sys: &System, offset: u32) -> u32 {
        match offset {
            0x0000 => {
                // CR register
                // Return all the r to true. This is where the PLL ready flags are.
                //0b0010_0000_0010_0000_0000_0000_0010
                self.registers[0] | 0xFFFF_0003
            }
            0x0008 => {
                // CFGR register
                self.registers[2] | 0b1000
            }
            0x0070 => {
                // BDCR: if the firmware enables LSE, expose LSERDY immediately.
                let v = self.registers[0x70 / 4];
                v | ((v & 0x1) << 1)
            }
            0x0074 => {
                // CSR: if the firmware enables LSI, expose LSIRDY immediately.
                let v = self.registers[0x74 / 4];
                v | ((v & 0x1) << 1)
            }
            _ => Self::reg_index(offset)
                .map(|i| self.registers[i])
                .unwrap_or(0),
        }
    }

    fn write(&mut self, _sys: &System, offset: u32, value: u32) {
        if let Some(i) = Self::reg_index(offset) {
            self.registers[i] = value;
        }
    }
}
