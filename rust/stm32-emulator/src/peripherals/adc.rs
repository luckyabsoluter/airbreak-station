// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::VecDeque;

use super::Peripheral;
use crate::system::System;

const CHANNEL_COUNT: usize = 32;
const DEFAULT_SAMPLE: u16 = 2254;
const STATUS_REGISTER_OFFSET: u32 = 0x00;
const CONTROL_REGISTER1_OFFSET: u32 = 0x04;
const CONTROL_REGISTER2_OFFSET: u32 = 0x08;
const SAMPLE_TIME_REGISTER1_OFFSET: u32 = 0x0C;
const SAMPLE_TIME_REGISTER2_OFFSET: u32 = 0x10;
const REGULAR_SEQUENCE_REGISTER1_OFFSET: u32 = 0x2C;
const REGULAR_SEQUENCE_REGISTER2_OFFSET: u32 = 0x30;
const REGULAR_SEQUENCE_REGISTER3_OFFSET: u32 = 0x34;
const INJECTED_SEQUENCE_REGISTER_OFFSET: u32 = 0x38;
const DATA_REGISTER_OFFSET: u32 = 0x4C;
const END_OF_CONVERSION_BIT: u32 = 1 << 1;

pub struct Adc {
    status_register: u32,
    control_register1: u32,
    control_register2: u32,
    sample_time_register1: u32,
    sample_time_register2: u32,
    injected_sequence_register: u32,
    regular_sequence_register1: u32,
    regular_sequence_register2: u32,
    regular_sequence_register3: u32,
    sequence_index: usize,
    channel_samples: [u16; CHANNEL_COUNT],
    shadow: [u8; 0x100],
}

impl Default for Adc {
    fn default() -> Self {
        Self {
            status_register: END_OF_CONVERSION_BIT,
            control_register1: 0,
            control_register2: 0,
            sample_time_register1: 0,
            sample_time_register2: 0,
            injected_sequence_register: 0,
            regular_sequence_register1: 0,
            regular_sequence_register2: 0,
            regular_sequence_register3: 0,
            sequence_index: 0,
            channel_samples: [DEFAULT_SAMPLE; CHANNEL_COUNT],
            shadow: [0; 0x100],
        }
    }
}

impl Adc {
    pub fn new(name: &str) -> Option<Box<dyn Peripheral>> {
        if name.starts_with("ADC") {
            Some(Box::<Self>::default())
        } else {
            None
        }
    }

    fn current_regular_channel(&self) -> usize {
        let shift = (self.sequence_index % 6) * 5;
        let channel = ((self.regular_sequence_register3 >> shift) & 0x1F) as usize;
        channel.min(CHANNEL_COUNT - 1)
    }

    fn next_sample(&mut self) -> u32 {
        let channel = self.current_regular_channel();
        self.sequence_index = (self.sequence_index + 1) & 0x7;
        self.status_register |= END_OF_CONVERSION_BIT;
        self.channel_samples[channel] as u32
    }

    fn read_shadow(&self, offset: u32) -> u32 {
        let offset = offset as usize;
        if offset + 3 >= self.shadow.len() {
            return 0;
        }
        u32::from_le_bytes([
            self.shadow[offset],
            self.shadow[offset + 1],
            self.shadow[offset + 2],
            self.shadow[offset + 3],
        ])
    }

    fn write_shadow(&mut self, offset: u32, value: u32) {
        let offset = offset as usize;
        if offset + 3 >= self.shadow.len() {
            return;
        }
        self.shadow[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
}

impl Peripheral for Adc {
    fn read(&mut self, _sys: &System, offset: u32) -> u32 {
        match offset {
            STATUS_REGISTER_OFFSET => self.status_register | END_OF_CONVERSION_BIT,
            CONTROL_REGISTER1_OFFSET => self.control_register1,
            CONTROL_REGISTER2_OFFSET => self.control_register2,
            SAMPLE_TIME_REGISTER1_OFFSET => self.sample_time_register1,
            SAMPLE_TIME_REGISTER2_OFFSET => self.sample_time_register2,
            INJECTED_SEQUENCE_REGISTER_OFFSET => self.injected_sequence_register,
            REGULAR_SEQUENCE_REGISTER1_OFFSET => self.regular_sequence_register1,
            REGULAR_SEQUENCE_REGISTER2_OFFSET => self.regular_sequence_register2,
            REGULAR_SEQUENCE_REGISTER3_OFFSET => self.regular_sequence_register3,
            DATA_REGISTER_OFFSET => self.next_sample(),
            _ => self.read_shadow(offset),
        }
    }

    fn write(&mut self, _sys: &System, offset: u32, value: u32) {
        self.write_shadow(offset, value);
        match offset {
            STATUS_REGISTER_OFFSET => self.status_register = value | END_OF_CONVERSION_BIT,
            CONTROL_REGISTER1_OFFSET => self.control_register1 = value,
            CONTROL_REGISTER2_OFFSET => {
                self.control_register2 = value;
                self.status_register |= END_OF_CONVERSION_BIT;
            }
            SAMPLE_TIME_REGISTER1_OFFSET => self.sample_time_register1 = value,
            SAMPLE_TIME_REGISTER2_OFFSET => self.sample_time_register2 = value,
            INJECTED_SEQUENCE_REGISTER_OFFSET => self.injected_sequence_register = value,
            REGULAR_SEQUENCE_REGISTER1_OFFSET => {
                self.regular_sequence_register1 = value;
                self.sequence_index = 0;
            }
            REGULAR_SEQUENCE_REGISTER2_OFFSET => {
                self.regular_sequence_register2 = value;
                self.sequence_index = 0;
            }
            REGULAR_SEQUENCE_REGISTER3_OFFSET => {
                self.regular_sequence_register3 = value;
                self.sequence_index = 0;
            }
            _ => {}
        }
    }

    fn read_dma(&mut self, sys: &System, offset: u32, size: usize) -> VecDeque<u8> {
        let mut bytes = VecDeque::with_capacity(size);
        while bytes.len() < size {
            let sample = self.read(sys, offset).to_le_bytes();
            for byte in sample {
                if bytes.len() == size {
                    break;
                }
                bytes.push_back(byte);
            }
        }
        bytes
    }
}
