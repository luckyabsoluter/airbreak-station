// SPDX-License-Identifier: GPL-3.0-or-later

mod display;
pub mod front_panel;
mod lcd;
mod m95m02;
mod spi_flash;
mod touchscreen;
mod usart_probe;

use display::{Display, DisplayConfig};
use front_panel::FrontPanelConfig;
use lcd::{Lcd, LcdConfig};
use m95m02::{M95M02Config, M95M02};
use spi_flash::{SpiFlash, SpiFlashConfig};
use touchscreen::{Touchscreen, TouchscreenConfig};
use usart_probe::{UsartProbe, UsartProbeConfig};

use anyhow::Result;
use serde::Deserialize;
use std::{cell::RefCell, rc::Rc};

use crate::{
    framebuffers::Framebuffers,
    peripherals::gpio::{GpioPorts, Pin},
    system::System,
};

#[derive(Debug, Deserialize, Default)]
pub struct ExtDevicesConfig {
    pub front_panel: Option<Vec<FrontPanelConfig>>,
    pub spi_flash: Option<Vec<SpiFlashConfig>>,
    pub m95m02: Option<Vec<M95M02Config>>,
    pub usart_probe: Option<Vec<UsartProbeConfig>>,
    pub display: Option<Vec<DisplayConfig>>,
    pub lcd: Option<Vec<LcdConfig>>,
    pub touchscreen: Option<Vec<TouchscreenConfig>>,
}

pub struct ExtDevices {
    pub spi_flashes: Vec<Rc<RefCell<SpiFlash>>>,
    pub m95m02s: Vec<Rc<RefCell<M95M02>>>,
    pub usart_probes: Vec<Rc<RefCell<UsartProbe>>>,
    pub displays: Vec<Rc<RefCell<Display>>>,
    pub lcds: Vec<Rc<RefCell<Lcd>>>,
    pub touchscreens: Vec<Rc<RefCell<Touchscreen>>>,
}

impl ExtDevices {
    pub fn find_serial_device(
        &self,
        peri_name: &str,
    ) -> Option<Rc<RefCell<dyn ExtDevice<(), u8>>>> {
        self.spi_flashes
            .iter()
            .filter(|d| d.borrow().config.peripheral == peri_name)
            .next()
            .map(|d| d.clone() as Rc<RefCell<dyn ExtDevice<(), u8>>>)
            .or_else(|| {
                self.m95m02s
                    .iter()
                    .filter(|d| d.borrow().config.peripheral == peri_name)
                    .next()
                    .map(|d| d.clone() as Rc<RefCell<dyn ExtDevice<(), u8>>>)
            })
            .or_else(|| {
                self.usart_probes
                    .iter()
                    .filter(|d| d.borrow().config.peripheral == peri_name)
                    .next()
                    .map(|d| d.clone() as Rc<RefCell<dyn ExtDevice<(), u8>>>)
            })
            .or_else(|| {
                self.lcds
                    .iter()
                    .filter(|d| d.borrow().config.peripheral == peri_name)
                    .next()
                    .map(|d| d.clone() as Rc<RefCell<dyn ExtDevice<(), u8>>>)
            })
            .or_else(|| {
                self.touchscreens
                    .iter()
                    .filter(|d| d.borrow().config.peripheral == peri_name)
                    .next()
                    .map(|d| d.clone() as Rc<RefCell<dyn ExtDevice<(), u8>>>)
            })
    }

    pub fn find_mem_device(&self, peri_name: &str) -> Option<Rc<RefCell<dyn ExtDevice<u32, u32>>>> {
        self.displays
            .iter()
            .filter(|d| d.borrow().config.peripheral == peri_name)
            .next()
            .map(|d| d.clone() as Rc<RefCell<dyn ExtDevice<u32, u32>>>)
    }
}

impl ExtDevicesConfig {
    pub fn into_ext_devices(
        self,
        gpio: &mut GpioPorts,
        framebuffers: &Framebuffers,
    ) -> Result<ExtDevices> {
        let spi_flashes = self
            .spi_flash
            .unwrap_or_default()
            .into_iter()
            .map(|config| SpiFlash::new(config).map(RefCell::new).map(Rc::new))
            .collect::<Result<_>>()?;

        for config in self.front_panel.unwrap_or_default() {
            front_panel::register(config, gpio)?;
        }

        let m95m02s = self
            .m95m02
            .unwrap_or_default()
            .into_iter()
            .map(|config| M95M02::new(config).map(RefCell::new).map(Rc::new))
            .collect::<Result<Vec<_>>>()?;

        for device in &m95m02s {
            let chip_select_pin = device.borrow().config.chip_select_pin.clone();
            if let Some(pin_name) = chip_select_pin {
                let pin = Pin::from_str(&pin_name);
                let device = device.clone();
                gpio.add_write_callback(pin, move |_sys, value| {
                    if value {
                        device.borrow_mut().finish_transmission();
                    }
                });
            }
        }

        let usart_probes = self
            .usart_probe
            .unwrap_or_default()
            .into_iter()
            .map(|config| UsartProbe::new(config).map(RefCell::new).map(Rc::new))
            .collect::<Result<_>>()?;

        let displays = self
            .display
            .unwrap_or_default()
            .into_iter()
            .map(|config| {
                Display::new(config, framebuffers)
                    .map(RefCell::new)
                    .map(Rc::new)
            })
            .collect::<Result<_>>()?;

        let lcds = self
            .lcd
            .unwrap_or_default()
            .into_iter()
            .map(|config| {
                Lcd::new(config, framebuffers)
                    .map(RefCell::new)
                    .map(Rc::new)
            })
            .collect::<Result<_>>()?;

        let touchscreens = self
            .touchscreen
            .unwrap_or_default()
            .into_iter()
            .map(|config| {
                Touchscreen::new(config, gpio, framebuffers)
                    .map(RefCell::new)
                    .map(Rc::new)
            })
            .collect::<Result<_>>()?;

        Ok(ExtDevices {
            spi_flashes,
            m95m02s,
            usart_probes,
            displays,
            lcds,
            touchscreens,
        })
    }
}

///////////////////////////////////////////////////////////////////////////////////////

pub trait ExtDevice<A, T> {
    /// Should returns "{peri_name} {ext_device_name}"
    fn connect_peripheral<'a>(&mut self, peri_name: &str) -> String;
    fn read(&mut self, sys: &System, addr: A) -> T;
    fn write(&mut self, sys: &System, addr: A, v: T);
}
