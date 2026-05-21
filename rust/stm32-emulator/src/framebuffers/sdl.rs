// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::mouse::MouseButton;
use sdl2::{pixels::PixelFormatEnum, rect::Rect, render::Canvas, surface::Surface, video::Window};

use super::{sdl_engine::SDL, Framebuffer, FramebufferConfig};

pub const REFRESH_DURATION_MILLIS: u64 = 20;

pub struct Sdl {
    pub config: FramebufferConfig,
    canvas: Canvas<Window>,
    framebuffer: Surface<'static>,
    need_redraw: bool,
    last_redraw: Instant,
    pub window_id: u32,
    touch_position: Option<(u16, u16)>,
    control_panel: bool,
}

impl Sdl {
    pub fn new(config: FramebufferConfig) -> Self {
        let format = match config.mode.as_str() {
            "rgb565" => PixelFormatEnum::RGB565,
            // can't figure out how to do grayscale. See palette below.
            // "gray8" => PixelFormatEnum::Index8,
            "gray8" => PixelFormatEnum::RGB888,
            _ => unimplemented!(),
        };
        let control_panel = config.control_panel.unwrap_or(false);
        let window_width =
            config.width as u32 + crate::ext_devices::front_panel::panel_width(control_panel);
        let window_title = if control_panel {
            format!("{} + front panel", config.name)
        } else {
            config.name.clone()
        };
        let mut canvas =
            SDL.lock()
                .unwrap()
                .new_canvas(&window_title, window_width, config.height.into());
        let framebuffer = Surface::new(config.width.into(), config.height.into(), format).unwrap();

        /*
        // Can't figure out how to use Index8.
        let colors: Vec<_> = (0..0xff).map(|u| Color::RGB(u, u, u)).collect();
        let palette = Palette::with_colors(&colors).unwrap();
        framebuffer.set_palette(&palette).unwrap();
        */

        if let Some(downscale) = config.downscale {
            canvas
                .window_mut()
                .set_size(window_width / downscale, config.height as u32 / downscale)
                .unwrap();
        }

        canvas.window_mut().raise();

        if control_panel {
            crate::ext_devices::front_panel::draw_control_panel(
                &mut canvas,
                config.width as i32,
                config.height as u32,
            )
            .unwrap();
            canvas.present();
        }

        let last_redraw = Instant::now();
        let need_redraw = false;
        let window_id = canvas.window().id();

        let touch_position = None;

        Self {
            config,
            canvas,
            framebuffer,
            need_redraw,
            last_redraw,
            window_id,
            touch_position,
            control_panel,
        }
    }

    fn should_redraw(&mut self) -> bool {
        if !self.need_redraw {
            return false;
        }

        let now = Instant::now();
        if now.duration_since(self.last_redraw) > Duration::from_millis(REFRESH_DURATION_MILLIS) {
            self.last_redraw = now;
            self.need_redraw = false;
            true
        } else {
            false
        }
    }

    pub fn request_redraw(&mut self) {
        self.need_redraw = true;
    }

    pub fn maybe_redraw(&mut self) {
        if !self.should_redraw() {
            return;
        }

        let tc = self.canvas.texture_creator();
        let texture = self.framebuffer.as_texture(&tc).unwrap();
        let lcd_rect = Rect::new(0, 0, self.config.width as u32, self.config.height as u32);
        self.canvas.copy(&texture, None, lcd_rect).unwrap();
        if self.control_panel {
            crate::ext_devices::front_panel::draw_control_panel(
                &mut self.canvas,
                self.config.width as i32,
                self.config.height as u32,
            )
            .unwrap();
        }

        self.canvas.present();
    }

    pub fn process_event(&mut self, event: Event) {
        match event {
            Event::MouseMotion { x, y, .. } => {
                if self.control_panel && x >= self.config.width as i32 {
                    return;
                }
                if self.touch_position.is_some() {
                    self.touch_position = Some((x as u16, y as u16));
                }
            }
            Event::MouseButtonDown {
                mouse_btn: MouseButton::Left,
                x,
                y,
                ..
            } => {
                if self.control_panel
                    && crate::ext_devices::front_panel::handle_panel_mouse_down(
                        x - self.config.width as i32,
                        y,
                    )
                {
                    self.request_redraw();
                    return;
                }
                self.touch_position = Some((x as u16, y as u16));
            }
            Event::MouseButtonUp {
                mouse_btn: MouseButton::Left,
                x,
                y,
                ..
            } => {
                if self.control_panel {
                    crate::ext_devices::front_panel::handle_panel_mouse_up(
                        x - self.config.width as i32,
                        y,
                    );
                    self.request_redraw();
                }
                self.touch_position = None;
            }
            Event::MouseWheel { y, .. } => {
                if self.control_panel {
                    crate::ext_devices::front_panel::handle_panel_wheel(y);
                    self.request_redraw();
                }
            }
            _ => {}
        }
    }
}

impl<Color> Framebuffer<Color> for Sdl {
    fn get_config(&self) -> &FramebufferConfig {
        &self.config
    }

    fn get_pixels(&mut self) -> &mut [Color] {
        self.need_redraw = true;

        let fb = self.framebuffer.without_lock_mut().unwrap();

        unsafe {
            std::slice::from_raw_parts_mut(
                fb.as_mut_ptr() as *mut Color,
                fb.len() / std::mem::size_of::<Color>(),
            )
        }
    }

    fn get_touch_position(&self) -> Option<(u16, u16)> {
        self.touch_position
    }
}
