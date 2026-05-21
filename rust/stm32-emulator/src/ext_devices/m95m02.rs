// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::{system::System, util};

use super::ExtDevice;

const DEFAULT_SIZE: usize = 256 * 1024;
const STATUS_WRITABLE_MASK: u8 = 0x8c;
const WRITE_ENABLE_LATCH_BIT: u8 = 0x02;

#[derive(Debug, Deserialize, Default)]
pub struct M95M02Config {
    pub peripheral: String,
    pub chip_select_pin: Option<String>,
    pub file: Option<String>,
    pub size: Option<usize>,
}

#[derive(Default)]
pub struct M95M02 {
    pub config: M95M02Config,
    name: String,
    storage: Vec<u8>,
    status: u8,
    write_enable_latch: bool,
    state: State,
    current_command: u8,
    address: usize,
    next_response: u8,
}

impl M95M02 {
    pub fn new(config: M95M02Config) -> Result<Self> {
        let size = config.size.unwrap_or(DEFAULT_SIZE);
        if size == 0 {
            bail!("M95M02 size must be non-zero");
        }
        let mut storage = vec![0xff; size];

        if let Some(path) = config.file.as_ref().filter(|path| !path.is_empty()) {
            let data = util::read_file(path).with_context(|| format!("Failed to read {}", path))?;
            let copy_len = data.len().min(storage.len());
            storage[..copy_len].copy_from_slice(&data[..copy_len]);
        }

        Ok(Self {
            config,
            storage,
            state: State::Command,
            next_response: 0xff,
            ..Self::default()
        })
    }

    pub fn finish_transmission(&mut self) {
        if matches!(self.state, State::WriteData | State::WriteStatus) {
            self.write_enable_latch = false;
        }

        self.state = State::Command;
        self.current_command = 0;
        self.address = 0;
        self.next_response = 0xff;
    }

    fn handle_command(&mut self, command: u8) {
        self.current_command = command;
        match command {
            0x06 => {
                self.write_enable_latch = true;
                self.state = State::Command;
            }
            0x04 => {
                self.write_enable_latch = false;
                self.state = State::Command;
            }
            0x05 => {
                self.next_response = self.status_value();
                self.state = State::ReadStatus;
            }
            0x01 => {
                self.state = State::WriteStatus;
            }
            0x03 | 0x02 => {
                self.address = 0;
                self.state = State::AddressHigh;
            }
            _ => {
                self.state = State::Command;
            }
        }
    }

    fn status_value(&self) -> u8 {
        (self.status & STATUS_WRITABLE_MASK)
            | if self.write_enable_latch {
                WRITE_ENABLE_LATCH_BIT
            } else {
                0
            }
    }

    fn advance_address(&mut self) {
        self.address = (self.address + 1) % self.storage.len();
    }
}

impl ExtDevice<(), u8> for M95M02 {
    fn connect_peripheral(&mut self, peri_name: &str) -> String {
        self.name = format!("{} m95m02", peri_name);
        self.name.clone()
    }

    fn read(&mut self, _sys: &System, _addr: ()) -> u8 {
        let response = self.next_response;
        self.next_response = 0xff;
        response
    }

    fn write(&mut self, _sys: &System, _addr: (), v: u8) {
        match self.state {
            State::Command => self.handle_command(v),
            State::AddressHigh => {
                self.address = (v as usize) << 16;
                self.state = State::AddressMid;
            }
            State::AddressMid => {
                self.address |= (v as usize) << 8;
                self.state = State::AddressLow;
            }
            State::AddressLow => {
                self.address = (self.address | v as usize) % self.storage.len();
                self.state = if self.current_command == 0x03 {
                    State::ReadData
                } else {
                    State::WriteData
                };

                if matches!(self.state, State::ReadData) {
                    self.next_response = self.storage[self.address];
                    self.advance_address();
                }
            }
            State::ReadData => {
                self.next_response = self.storage[self.address];
                self.advance_address();
            }
            State::WriteData => {
                if self.write_enable_latch {
                    self.storage[self.address] = v;
                    self.advance_address();
                }
            }
            State::ReadStatus => {
                self.next_response = self.status_value();
            }
            State::WriteStatus => {
                if self.write_enable_latch {
                    self.status = v & STATUS_WRITABLE_MASK;
                }
            }
        }
    }
}

#[derive(Default, Clone, Copy)]
enum State {
    #[default]
    Command,
    AddressHigh,
    AddressMid,
    AddressLow,
    ReadData,
    WriteData,
    ReadStatus,
    WriteStatus,
}
