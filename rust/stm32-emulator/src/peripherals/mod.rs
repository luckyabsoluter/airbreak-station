// SPDX-License-Identifier: GPL-3.0-or-later

pub mod adc;
pub mod dma;
pub mod exti;
pub mod fsmc;
pub mod gpio;
pub mod i2c;
pub mod nvic;
pub mod rcc;
pub mod scb;
pub mod spi;
pub mod sw_spi;
pub mod syscfg;
pub mod systick;
pub mod timer;
pub mod usart;

use adc::*;
use dma::*;
use exti::*;
use fsmc::*;
use gpio::*;
use i2c::*;
use nvic::*;
use rcc::*;
use scb::*;
use serde::Deserialize;
use spi::*;
use sw_spi::*;
use syscfg::*;
use systick::*;
use timer::*;
use usart::*;

use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, VecDeque},
};
use svd_parser::svd::{Device as SvdDevice, RegisterInfo};

use crate::{ext_devices::ExtDevices, system::System};

#[derive(Debug, Deserialize, Default)]
pub struct PeripheralsConfig {
    pub software_spi: Option<Vec<SoftwareSpiConfig>>,
}

#[derive(Default)]
pub struct Peripherals {
    debug_peripherals: Vec<PeripheralSlot<GenericPeripheral>>,
    peripherals: Vec<PeripheralSlot<RefCell<Box<dyn Peripheral>>>>,
    pub nvic: RefCell<Nvic>,
    pub gpio: RefCell<GpioPorts>,
    pub exti: RefCell<Exti>,
    pub syscfg: RefCell<Syscfg>,
}

pub struct PeripheralSlot<T> {
    pub start: u32,
    pub end: u32,
    pub peripheral: T,
}

enum BitBandTarget {
    Sram { addr: u32, bit: u8 },
    Peripheral { addr: u32, bit: u8 },
}

impl Peripherals {
    // start - end regions
    pub const MEMORY_MAPS: [(u32, u32); 5] = [
        (0x2200_0000, 0x2400_0000),
        (0x4000_0000, 0x6000_0000),
        (0x6400_0000, 0x6800_0000),
        (0xA000_0000, 0xA000_2000),
        (0xE000_0000, 0xE100_0000),
    ];

    pub fn register_peripheral(
        &mut self,
        name: String,
        base: u32,
        registers: &[RegisterInfo],
        ext_devices: &ExtDevices,
    ) {
        let p = GenericPeripheral::new(name.clone(), registers);

        let (start, end) = (base, base + p.size());

        trace!(
            "Peripheral start=0x{:08x} end=0x{:08x} name={}",
            start,
            end,
            p.name()
        );

        self.debug_peripherals.push(PeripheralSlot {
            start,
            end,
            peripheral: p,
        });

        // The debug peripheral is just for to print registers right now. So we
        // change the (start, end) only for the real peripheral.
        let (start, end) = match name.as_str() {
            "FSMC" => (0x6000_0000, 0xA000_1000),
            _ => (start, end),
        };

        let p = None
            .or_else(|| NvicWrapper::new(&name))
            .or_else(|| SysTick::new(&name))
            .or_else(|| Scb::new(&name))
            .or_else(|| SyscfgWrapper::new(&name))
            .or_else(|| ExtiWrapper::new(&name))
            .or_else(|| Gpio::new(&name))
            .or_else(|| Usart::new(&name, ext_devices))
            .or_else(|| Fsmc::new(&name, ext_devices))
            .or_else(|| Rcc::new(&name))
            .or_else(|| I2c::new(&name))
            .or_else(|| Dma::new(&name))
            .or_else(|| Spi::new(&name, ext_devices))
            .or_else(|| Adc::new(&name))
            .or_else(|| Timer::new(&name));

        if let Some(p) = p {
            self.peripherals.push(PeripheralSlot {
                start,
                end,
                peripheral: RefCell::new(p),
            });
        }
    }

    pub fn finish_registration(&mut self) {
        // We sort because we do binary searches to find peripherals
        self.debug_peripherals.sort_by_key(|p| p.start);
        self.peripherals.sort_by_key(|p| p.start);

        {
            // Let's check that peripherals don't overlap
            let a = self.debug_peripherals.iter();
            let mut b = self.debug_peripherals.iter();
            b.next();

            for (p1, p2) in a.zip(b) {
                assert!(
                    p1.end < p2.start,
                    "Overlapping register blocks between {} and {}",
                    p1.peripheral.name(),
                    p2.peripheral.name()
                );
            }
        }
    }

    pub fn from_svd(
        mut svd_device: SvdDevice,
        config: PeripheralsConfig,
        gpio: GpioPorts,
        ext_devices: &ExtDevices,
    ) -> Self {
        let mut peripherals = Self {
            gpio: RefCell::new(gpio),
            ..Peripherals::default()
        };

        svd_device.peripherals.sort_by_key(|f| f.base_address);
        let svd_peripherals = svd_device
            .peripherals
            .iter()
            .map(|d| (d.name.to_string(), d))
            .collect::<HashMap<_, _>>();

        for p in &svd_device.peripherals {
            let name = &p.name;
            let base = p.base_address;

            let p = if let Some(derived_from) = p.derived_from.as_ref() {
                svd_peripherals
                    .get(derived_from)
                    .as_ref()
                    .unwrap_or_else(|| panic!("Cannot find peripheral {}", derived_from))
            } else {
                p
            };

            let regs = crate::util::extract_svd_registers(p);

            peripherals.register_peripheral(name.to_string(), base as u32, &regs, ext_devices);

            if crate::verbose() >= 3 {
                for r in &regs {
                    trace!(
                        "p={} addr=0x{:08x} reg_name={}",
                        p.name,
                        p.base_address as u32 + r.address_offset,
                        r.name
                    );
                }
            }
        }

        for sw_spi_config in config.software_spi.unwrap_or_default() {
            SoftwareSpi::register(
                sw_spi_config,
                &mut peripherals.gpio.borrow_mut(),
                ext_devices,
            );
        }

        peripherals.finish_registration();
        peripherals
    }

    /////////////////////////////////////////////////////////////////////////////////////////////////////////////

    pub fn get_peripheral<T>(
        peripherals: &Vec<PeripheralSlot<T>>,
        addr: u32,
    ) -> Option<&PeripheralSlot<T>> {
        let index = peripherals
            .binary_search_by_key(&addr, |p| p.start)
            .map_or_else(|e| e.checked_sub(1), |v| Some(v));

        index
            .map(|i| peripherals.get(i).filter(|p| addr <= p.end))
            .flatten()
    }

    pub fn addr_desc(&self, addr: u32) -> String {
        if let Some(p) = Self::get_peripheral(&self.debug_peripherals, addr) {
            format!(
                "addr=0x{:08x} peri={} {}",
                addr,
                p.peripheral.name,
                p.peripheral.reg_name(addr - p.start)
            )
        } else {
            format!("addr=0x{:08x} peri=????", addr)
        }
    }

    pub fn signal_gpio_input(&self, sys: &System, pin: Pin, level: bool) {
        self.exti
            .borrow_mut()
            .signal_gpio_input(sys, &self.syscfg.borrow(), pin, level);
    }

    fn bitbanding(addr: u32) -> Option<BitBandTarget> {
        if (0x2200_0000..0x2400_0000).contains(&addr) {
            let alias_offset = addr - 0x2200_0000;
            let bit = ((alias_offset % 32) / 4) as u8;
            let addr = 0x2000_0000 + alias_offset / 32;
            Some(BitBandTarget::Sram { addr, bit })
        } else if (0x4200_0000..0x4400_0000).contains(&addr) {
            let alias_offset = addr - 0x4200_0000;
            let bit = ((alias_offset % 32) / 4) as u8;
            let addr = 0x4000_0000 + alias_offset / 32;
            Some(BitBandTarget::Peripheral { addr, bit })
        } else {
            None
        }
    }

    fn read_sram_byte(sys: &System, addr: u32) -> u8 {
        let mut byte = [0];
        if sys.uc.borrow().mem_read(addr.into(), &mut byte).is_err() {
            warn!("failed to read SRAM bit-band target addr=0x{:08x}", addr);
            0
        } else {
            byte[0]
        }
    }

    fn write_sram_byte(sys: &System, addr: u32, value: u8) {
        if sys
            .uc
            .borrow_mut()
            .mem_write(addr.into(), &[value])
            .is_err()
        {
            warn!("failed to write SRAM bit-band target addr=0x{:08x}", addr);
        }
    }

    fn read_bitband(&self, sys: &System, target: BitBandTarget) -> u32 {
        match target {
            BitBandTarget::Sram { addr, bit } => {
                ((Self::read_sram_byte(sys, addr) >> bit) & 1).into()
            }
            BitBandTarget::Peripheral { addr, bit } => (self.read(sys, addr, 1) >> bit) & 1,
        }
    }

    fn write_bitband(&self, sys: &System, target: BitBandTarget, value: u32) {
        let set = value & 1 != 0;
        match target {
            BitBandTarget::Sram { addr, bit } => {
                let mut byte = Self::read_sram_byte(sys, addr);
                if set {
                    byte |= 1 << bit;
                } else {
                    byte &= !(1 << bit);
                }
                Self::write_sram_byte(sys, addr, byte);
            }
            BitBandTarget::Peripheral { addr, bit } => {
                let mut byte = self.read(sys, addr, 1);
                if set {
                    byte |= 1 << bit;
                } else {
                    byte &= !(1 << bit);
                }
                self.write(sys, addr, 1, byte);
            }
        }
    }

    fn is_register(addr: u32) -> bool {
        // this is avoiding the FSMC banks, essentially
        !(0x6000_0000..0xA000_0000).contains(&addr)
    }

    fn align_addr_4(addr: u32) -> (u32, u8) {
        let byte_offset = (addr % 4) as u8;
        let addr = addr - byte_offset as u32;
        (addr, byte_offset)
    }

    pub fn read(&self, sys: &System, addr: u32, size: u8) -> u32 {
        if let Some(target) = Self::bitbanding(addr) {
            return self.read_bitband(sys, target);
        }

        let (addr, byte_offset) = if Self::is_register(addr) {
            // Reduce the access to 4 byte alignements to make things easier when dealing with registers
            Self::align_addr_4(addr)
        } else {
            (addr, 0)
        };

        assert!(byte_offset + size <= 4);

        let raw = if let Some(p) = Self::get_peripheral(&self.peripherals, addr) {
            p.peripheral.borrow_mut().read(sys, addr - p.start)
        } else {
            0
        };
        let shift = 8 * byte_offset;
        let mask = if size == 4 {
            u32::MAX
        } else {
            (1u32 << (8 * size)) - 1
        };
        let value = (raw >> shift) & mask;

        if crate::verbose() >= 3 {
            trace!("read:  {} read=0x{:08x}", self.addr_desc(addr), value);
        }

        value
    }

    pub fn write(&self, sys: &System, addr: u32, size: u8, mut value: u32) {
        if let Some(target) = Self::bitbanding(addr) {
            return self.write_bitband(sys, target, value);
        }

        let (addr, byte_offset) = if Self::is_register(addr) {
            // Reduce the access to 4 byte alignements to make things easier when dealing with registers
            Self::align_addr_4(addr)
        } else {
            (addr, 0)
        };

        assert!(byte_offset + size <= 4);

        if byte_offset != 0 || size != 4 {
            let current = self.read(sys, addr, 4);
            let shift = 8 * byte_offset;
            let width = 8 * size;
            let mask = if width == 32 {
                u32::MAX
            } else {
                ((1u32 << width) - 1) << shift
            };
            value = (current & !mask) | ((value << shift) & mask);
        }

        if let Some(p) = Self::get_peripheral(&self.peripherals, addr) {
            p.peripheral.borrow_mut().write(sys, addr - p.start, value)
        }

        if crate::verbose() >= 3 {
            trace!("write: {} write=0x{:08x}", self.addr_desc(addr), value);
        }
    }
}

pub trait Peripheral {
    fn read(&mut self, sys: &System, offset: u32) -> u32;
    fn write(&mut self, sys: &System, offset: u32, value: u32);

    fn read_dma(&mut self, sys: &System, offset: u32, size: usize) -> VecDeque<u8> {
        let mut v = VecDeque::with_capacity(size);
        for _ in 0..size {
            v.push_back(self.read(sys, offset) as u8);
        }
        v
    }
    fn write_dma(&mut self, sys: &System, offset: u32, value: VecDeque<u8>) {
        for v in value.into_iter() {
            self.write(sys, offset, v.into());
        }
    }
}

struct GenericPeripheral {
    pub name: String,
    // offset -> name
    pub registers: BTreeMap<u32, RegisterInfo>,
}

impl GenericPeripheral {
    pub fn new(name: String, registers: &[RegisterInfo]) -> Self {
        let registers = registers
            .iter()
            .map(|r| (r.address_offset, r.clone()))
            .collect();

        Self { name, registers }
    }

    pub fn reg_name(&self, offset: u32) -> String {
        assert!(offset % 4 == 0);
        let reg = self.registers.get(&offset);
        reg.map(|r| &r.name)
            .map(|r| format!("offset=0x{:04x} reg={}", offset, r))
            .unwrap_or_else(|| format!("offset=0x{:04x} reg=????", offset))
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn size(&self) -> u32 {
        self.registers.keys().cloned().max().unwrap_or(0) + 4
    }
}
