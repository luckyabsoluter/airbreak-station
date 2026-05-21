// SPDX-License-Identifier: GPL-3.0-or-later

use super::Peripheral;
use super::Peripherals;
use crate::system::System;
use crate::util::UniErr;

const SPI3_DATA_REGISTER_ADDRESS: u32 = 0x4000_3c0c;

#[derive(Default)]
pub struct Dma {
    name: String,
    lisr: u32,
    hisr: u32,
    streams: [Stream; 8],
}

impl Dma {
    pub fn new(name: &str) -> Option<Box<dyn Peripheral>> {
        if name.starts_with("DMA") {
            let name = name.to_string();
            Some(Box::new(Self {
                name,
                ..Self::default()
            }))
        } else {
            None
        }
    }
}

impl Peripheral for Dma {
    fn read(&mut self, sys: &System, offset: u32) -> u32 {
        match Access::from_offset(offset) {
            Access::Reg(0x0000) => self.lisr,
            Access::Reg(0x0004) => self.hisr,
            Access::StreamReg(i, offset) => self.streams[i].read(&self.name, sys, offset),
            _ => 0,
        }
    }

    fn write(&mut self, sys: &System, offset: u32, value: u32) {
        match Access::from_offset(offset) {
            Access::Reg(0x0008) => self.lisr &= !value,
            Access::Reg(0x000c) => self.hisr &= !value,
            Access::StreamReg(i, offset) => {
                if self.try_handle_spi3_full_duplex_dma(sys, i, offset, value) {
                    return;
                }

                if self.streams[i].write(&self.name, sys, offset, value) {
                    self.mark_transfer_complete(sys, i);
                }
            }
            _ => {}
        }
    }
}

impl Dma {
    fn transfer_complete_mask(stream: usize) -> (bool, u32) {
        let shift = match stream % 4 {
            0 => 0,
            1 => 6,
            2 => 16,
            _ => 22,
        };
        (stream >= 4, 1 << (shift + 5))
    }

    fn stream_irq(&self, stream: usize) -> Option<i32> {
        const DMA1_IRQS: [i32; 8] = [11, 12, 13, 14, 15, 16, 17, 47];
        const DMA2_IRQS: [i32; 8] = [56, 57, 58, 59, 60, 68, 69, 70];

        match self.name.as_str() {
            "DMA1" => Some(DMA1_IRQS[stream]),
            "DMA2" => Some(DMA2_IRQS[stream]),
            _ => None,
        }
    }

    fn mark_transfer_complete(&mut self, sys: &System, stream: usize) {
        let (high, tcif) = Self::transfer_complete_mask(stream);
        if high {
            self.hisr |= tcif;
        } else {
            self.lisr |= tcif;
        }

        if self.streams[stream].transfer_complete_interrupt_enabled() {
            if let Some(irq) = self.stream_irq(stream) {
                sys.p.nvic.borrow_mut().set_intr_pending(irq);
            }
        }
    }

    fn try_handle_spi3_full_duplex_dma(
        &mut self,
        sys: &System,
        stream: usize,
        offset: u32,
        value: u32,
    ) -> bool {
        if self.name != "DMA1" || offset != 0x0000 || value & 1 == 0 {
            return false;
        }

        if stream == 0
            && self.streams[0].is_spi3_rx_control(value)
            && self.streams[7].is_spi3_tx_configured()
        {
            self.streams[0].write_register_only(offset, value);
            if !self.try_complete_spi3_full_duplex_dma(sys) {
                debug!("DMA1 defer SPI3 RX stream0 until TX stream7 is enabled");
            }
            return true;
        }

        if stream == 7
            && self.streams[7].is_spi3_tx_control(value)
            && self.streams[0].is_spi3_rx_pending()
        {
            self.streams[7].write_register_only(offset, value);
            if self.try_complete_spi3_full_duplex_dma(sys) {
                return true;
            }
        }

        false
    }

    fn try_complete_spi3_full_duplex_dma(&mut self, sys: &System) -> bool {
        if !self.streams[0].is_spi3_rx_pending() || !self.streams[7].is_spi3_tx_pending() {
            return false;
        }

        let rx_addr = self.streams[0].data_addr();
        let tx_addr = self.streams[7].data_addr();
        let rx_size = self.streams[0].data_size();
        let tx_size = self.streams[7].data_size();
        let size = rx_size.min(tx_size);
        if size == 0 {
            return false;
        }

        if rx_size != tx_size {
            warn!(
                "DMA1 SPI3 full-duplex size mismatch rx={} tx={}",
                rx_size, tx_size
            );
        }

        let tx = match sys.uc.borrow().mem_read_as_vec(tx_addr.into(), size) {
            Ok(tx) => tx,
            Err(e) => {
                warn!(
                    "DMA1 SPI3 full-duplex TX read failed addr=0x{:08x} size={} e={}",
                    tx_addr,
                    size,
                    UniErr(e)
                );
                return false;
            }
        };

        let peri = match Peripherals::get_peripheral(&sys.p.peripherals, SPI3_DATA_REGISTER_ADDRESS)
        {
            Some(peri) => peri,
            None => {
                warn!("DMA1 SPI3 full-duplex could not find SPI3 data register");
                return false;
            }
        };

        let mut rx = Vec::with_capacity(size);
        {
            let mut peripheral = peri.peripheral.borrow_mut();
            for tx_byte in tx {
                peripheral.write(sys, SPI3_DATA_REGISTER_ADDRESS - peri.start, tx_byte.into());
                rx.push(peripheral.read(sys, SPI3_DATA_REGISTER_ADDRESS - peri.start) as u8);
            }
        }

        if let Err(e) = sys.uc.borrow_mut().mem_write(rx_addr.into(), &rx) {
            warn!(
                "DMA1 SPI3 full-duplex RX write failed addr=0x{:08x} size={} e={}",
                rx_addr,
                rx.len(),
                UniErr(e)
            );
            return false;
        }

        self.streams[0].finish_transfer();
        self.streams[7].finish_transfer();
        self.mark_transfer_complete(sys, 0);
        self.mark_transfer_complete(sys, 7);
        debug!("DMA1 SPI3 full-duplex transfer bytes={}", rx.len());
        true
    }
}

#[derive(Default)]
struct Stream {
    pub cr: u32,
    pub next_cr: Option<u32>,
    pub ndtr: u32,
    pub par: u32,
    pub m0ar: u32,
    pub m1ar: u32,
    pub fcr: u32,
}

impl Stream {
    fn channel(&self) -> u8 {
        ((self.cr >> 25) & 0b111) as u8
    }

    fn dir(&self) -> Dir {
        match (self.cr >> 6) & 0b11 {
            0b00 => Dir::Read,
            0b01 => Dir::Write,
            0b10 => Dir::MemCopy,
            _ => Dir::Invalid,
        }
    }

    // 1, 2, 4 (8bit, 16bit, 32bit)
    fn word_size(&self) -> usize {
        match (self.cr >> 11) & 0b11 {
            0b00 => 1,
            0b01 => 2,
            0b10 => 4,
            _ => 1,
        }
    }

    fn data_size(&self) -> usize {
        self.word_size() * self.ndtr as usize
    }

    fn data_addr(&self) -> u32 {
        if (self.cr >> 19) & 1 != 0 {
            self.m1ar
        } else {
            self.m0ar
        }
    }

    fn transfer_complete_interrupt_enabled(&self) -> bool {
        self.cr & (1 << 4) != 0
    }

    fn enabled(&self) -> bool {
        self.cr & 1 != 0
    }

    fn is_spi3_rx_control(&self, cr: u32) -> bool {
        let direction = match (cr >> 6) & 0b11 {
            0b00 => Dir::Read,
            0b01 => Dir::Write,
            0b10 => Dir::MemCopy,
            _ => Dir::Invalid,
        };
        direction == Dir::Read && self.par == SPI3_DATA_REGISTER_ADDRESS && self.ndtr != 0
    }

    fn is_spi3_tx_control(&self, cr: u32) -> bool {
        let direction = match (cr >> 6) & 0b11 {
            0b00 => Dir::Read,
            0b01 => Dir::Write,
            0b10 => Dir::MemCopy,
            _ => Dir::Invalid,
        };
        direction == Dir::Write && self.par == SPI3_DATA_REGISTER_ADDRESS && self.ndtr != 0
    }

    fn is_spi3_rx_pending(&self) -> bool {
        self.enabled()
            && self.dir() == Dir::Read
            && self.par == SPI3_DATA_REGISTER_ADDRESS
            && self.ndtr != 0
    }

    fn is_spi3_tx_pending(&self) -> bool {
        self.enabled()
            && self.dir() == Dir::Write
            && self.par == SPI3_DATA_REGISTER_ADDRESS
            && self.ndtr != 0
    }

    fn is_spi3_tx_configured(&self) -> bool {
        self.dir() == Dir::Write && self.par == SPI3_DATA_REGISTER_ADDRESS && self.ndtr != 0
    }

    fn finish_transfer(&mut self) {
        let next_cr = self.cr & !1;
        self.ndtr = 0;
        self.next_cr = Some(next_cr);
    }

    fn write_register_only(&mut self, offset: u32, value: u32) {
        match offset {
            0x0000 => self.cr = value,
            0x0004 => self.ndtr = value & 0xFFFF,
            0x0008 => self.par = value,
            0x000c => self.m0ar = value,
            0x0010 => self.m1ar = value,
            0x0014 => self.fcr = value,
            _ => {}
        }
    }

    fn do_xfer(&self, name: &str, sys: &System) {
        let dir = self.dir();
        let data_addr = self.data_addr();
        let size = self.data_size();
        let peri_addr = self.par;

        let peri = Peripherals::get_peripheral(&sys.p.peripherals, peri_addr);

        let (src, dst) = match dir {
            Dir::Read => (peri_addr, data_addr),
            Dir::Write => (data_addr, peri_addr),
            Dir::MemCopy => (peri_addr, data_addr),
            Dir::Invalid => (0, 0),
        };

        if log::log_enabled!(log::Level::Debug) {
            let peri_desc = sys.p.addr_desc(peri_addr);
            debug!(
                "{} xfer initiated channel={} peri_{} dir={:?} addr=0x{:08x} size={}",
                name,
                self.channel(),
                peri_desc,
                dir,
                data_addr,
                size
            );
        }

        let buf = match dir {
            Dir::Read => peri.map(|p| {
                p.peripheral
                    .borrow_mut()
                    .read_dma(sys, peri_addr - p.start, size)
            }),
            Dir::Write | Dir::MemCopy => sys
                .uc
                .borrow()
                .mem_read_as_vec(src.into(), size)
                .map_err(|e| {
                    warn!(
                        "DMA read failed addr=0x{:08x} size={} e={}",
                        src,
                        size,
                        UniErr(e)
                    )
                })
                .map(|v| v.into())
                .ok(),
            Dir::Invalid => Some(vec![].into()),
        };

        let mut buf = buf.unwrap_or_else(|| {
            let mut rx = vec![];
            rx.resize(size, 0);
            rx.into()
        });

        trace!("{} xfer buf={:x?}", name, buf);

        match dir {
            Dir::Write => {
                peri.map(|p| {
                    p.peripheral
                        .borrow_mut()
                        .write_dma(sys, peri_addr - p.start, buf)
                });
            }
            Dir::Read | Dir::MemCopy => {
                if let Err(e) = sys
                    .uc
                    .borrow_mut()
                    .mem_write(dst.into(), buf.make_contiguous())
                {
                    warn!(
                        "DMA read failed addr=0x{:08x} size={} e={}",
                        dst,
                        size,
                        UniErr(e)
                    );
                }
            }
            Dir::Invalid => {}
        }
    }

    pub fn read(&mut self, _name: &str, _sys: &System, offset: u32) -> u32 {
        match offset {
            0x0000 => {
                let v = self.cr;
                if let Some(next_cr) = self.next_cr.take() {
                    self.cr = next_cr;
                }

                // The saturn firmware is a bit buggy. When doing a DMA write
                // with size=0, they don't enable the DMA channel, but they
                // wait for it to go to 1 and then 0, with a timeout. So they
                // are consistently hitting the timeout.
                // We'll do toggles on the ready flag to speed things up avoiding the timeout.
                if self.dir() == Dir::Write && self.data_size() == 0 {
                    self.next_cr = Some(self.cr ^ 1)
                }

                v
            }
            0x0004 => self.ndtr,
            0x0008 => self.par,
            0x000c => self.m0ar,
            0x0010 => self.m1ar,
            0x0014 => self.fcr,
            _ => 0,
        }
    }

    pub fn write(&mut self, name: &str, sys: &System, offset: u32, mut value: u32) -> bool {
        match offset {
            0x0000 => {
                self.cr = value;

                // CRx register
                if value & 1 != 0 {
                    // Enable is on. do the transfer.
                    self.do_xfer(name, sys);

                    value &= !1;
                    self.ndtr = 0;
                    self.next_cr = Some(value);
                    return true;
                }
            }
            _ => self.write_register_only(offset, value),
        }
        false
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Dir {
    Read,
    Write,
    MemCopy,
    Invalid,
}

enum Access {
    Reg(u32),
    /// CR0, CR1, etc.
    StreamReg(usize, u32),
}

impl Access {
    pub fn from_offset(offset: u32) -> Self {
        if offset < 0x10 {
            Access::Reg(offset)
        } else {
            let stride = 0x18;
            let start = 0x10;

            let offset = offset - start;
            Access::StreamReg((offset / stride) as usize, offset % stride)
        }
    }
}
