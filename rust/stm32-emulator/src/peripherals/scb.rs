// SPDX-License-Identifier: GPL-3.0-or-later

use super::Peripheral;
use crate::system::System;

#[derive(Default)]
pub struct Scb {}

impl Scb {
    pub fn new(name: &str) -> Option<Box<dyn Peripheral>> {
        if name == "SCB" {
            Some(Box::new(Self::default()))
        } else {
            None
        }
    }
}

impl Peripheral for Scb {
    fn read(&mut self, sys: &System, offset: u32) -> u32 {
        sys.p.nvic.borrow_mut().read_scb(sys, offset)
    }

    fn write(&mut self, sys: &System, offset: u32, value: u32) {
        sys.p.nvic.borrow_mut().write_scb(sys, offset, value);
    }
}
