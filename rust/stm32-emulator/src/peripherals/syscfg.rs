// SPDX-License-Identifier: GPL-3.0-or-later

use super::Peripheral;
use crate::system::System;

#[derive(Default)]
pub struct Syscfg {
    memrmp: u32,
    pmc: u32,
    exticr: [u32; 4],
    cmpcr: u32,
}

impl Syscfg {
    pub fn exti_port(&self, line: u8) -> u8 {
        let index = (line / 4) as usize;
        let shift = (line % 4) * 4;
        ((self.exticr[index] >> shift) & 0xf) as u8
    }

    fn read(&mut self, offset: u32) -> u32 {
        match offset {
            0x0000 => self.memrmp,
            0x0004 => self.pmc,
            0x0008..=0x0014 if offset % 4 == 0 => self.exticr[((offset - 0x0008) / 4) as usize],
            0x0020 => self.cmpcr,
            _ => {
                warn!("SYSCFG invalid offset=0x{:08x}", offset);
                0
            }
        }
    }

    fn write(&mut self, offset: u32, value: u32) {
        match offset {
            0x0000 => self.memrmp = value,
            0x0004 => self.pmc = value,
            0x0008..=0x0014 if offset % 4 == 0 => {
                let index = ((offset - 0x0008) / 4) as usize;
                self.exticr[index] = value;
                debug!("SYSCFG EXTICR{}=0x{:08x}", index + 1, value);
            }
            0x0020 => self.cmpcr = value,
            _ => warn!("SYSCFG invalid offset=0x{:08x}", offset),
        }
    }
}

pub struct SyscfgWrapper;

impl SyscfgWrapper {
    pub fn new(name: &str) -> Option<Box<dyn Peripheral>> {
        if name == "SYSCFG" {
            Some(Box::new(Self))
        } else {
            None
        }
    }
}

impl Peripheral for SyscfgWrapper {
    fn read(&mut self, sys: &System, offset: u32) -> u32 {
        sys.p.syscfg.borrow_mut().read(offset)
    }

    fn write(&mut self, sys: &System, offset: u32, value: u32) {
        sys.p.syscfg.borrow_mut().write(offset, value);
    }
}
