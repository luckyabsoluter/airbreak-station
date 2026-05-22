// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;
use serde::Deserialize;
use std::collections::VecDeque;
use std::sync::Mutex;

use sdl2::{
    event::Event,
    keyboard::Keycode,
    pixels::Color,
    rect::{Point, Rect},
    render::Canvas,
    video::Window,
};

use crate::peripherals::gpio::{GpioPorts, Pin};
use crate::system::System;

lazy_static::lazy_static! {
    static ref INPUT: Mutex<InputState> = Mutex::new(InputState::default());
}

const PANEL_WIDTH: u32 = 160;
const POWER_RECT: PanelRect = PanelRect::new(24, 16, 112, 52);
const HOME_RECT: PanelRect = PanelRect::new(24, 82, 112, 52);
const ENCODER_CX: i32 = 80;
const ENCODER_CY: i32 = 230;
const ENCODER_RING_R: i32 = 58;
const ENCODER_BUTTON_R: i32 = 24;
const BUTTON_RELEASE_HOLD_TICKS: u16 = 200;
const BUTTON_ACTIVE_READ_LOG_LIMIT: u8 = 8;
const ENCODER_PHASE_MIN_HOLD_TICKS: u16 = 4;
const ENCODER_PHASE_MAX_HOLD_TICKS: u16 = 200;
const ENCODER_ACTIVE_READ_LOG_LIMIT: u8 = 12;

#[derive(Debug, Deserialize, Default)]
pub struct FrontPanelConfig {
    pub power_button_pin: Option<String>,
    pub home_button_pin: Option<String>,
    pub encoder_button_pin: Option<String>,
    pub encoder_a_pin: Option<String>,
    pub encoder_b_pin: Option<String>,
    pub idle_high: Option<bool>,
    pub encoder_idle_high: Option<bool>,
    pub rotary_provider_obj: Option<String>,
}

pub fn register(config: FrontPanelConfig, gpio: &mut GpioPorts) -> Result<()> {
    let idle_high = config.idle_high.unwrap_or(true);
    let encoder_idle_high = config.encoder_idle_high.unwrap_or(idle_high);

    INPUT
        .lock()
        .unwrap()
        .configure(&config, idle_high, encoder_idle_high);

    if let Some(pin_name) = config.power_button_pin.as_deref() {
        gpio.add_read_callback(Pin::from_str(pin_name), |_sys| read_button(Button::Power));
    }
    if let Some(pin_name) = config.home_button_pin.as_deref() {
        gpio.add_read_callback(Pin::from_str(pin_name), |_sys| read_button(Button::Home));
    }
    if let Some(pin_name) = config.encoder_button_pin.as_deref() {
        gpio.add_read_callback(Pin::from_str(pin_name), |_sys| read_button(Button::Encoder));
    }
    if let Some(pin_name) = config.encoder_a_pin.as_deref() {
        gpio.add_read_callback(Pin::from_str(pin_name), |_sys| read_encoder(EncoderLine::A));
    }
    if let Some(pin_name) = config.encoder_b_pin.as_deref() {
        gpio.add_read_callback(Pin::from_str(pin_name), |_sys| read_encoder(EncoderLine::B));
    }

    Ok(())
}

pub fn panel_width(enabled: bool) -> u32 {
    if enabled {
        PANEL_WIDTH
    } else {
        0
    }
}

pub fn handle_sdl_event(event: &Event) -> bool {
    match *event {
        Event::KeyDown {
            keycode: Some(keycode),
            repeat: false,
            ..
        } => handle_key_down(keycode),
        Event::KeyUp {
            keycode: Some(keycode),
            repeat: false,
            ..
        } => handle_key_up(keycode),
        _ => false,
    }
}

pub fn handle_panel_mouse_down(x: i32, y: i32) -> bool {
    if x < 0 || x >= PANEL_WIDTH as i32 || y < 0 {
        return false;
    }

    let mut input = INPUT.lock().unwrap();
    if POWER_RECT.contains(x, y) {
        input.set_mouse_button(Button::Power, "mouse");
        return true;
    }
    if HOME_RECT.contains(x, y) {
        input.set_mouse_button(Button::Home, "mouse");
        return true;
    }

    let dx = x - ENCODER_CX;
    let dy = y - ENCODER_CY;
    let dist2 = dx * dx + dy * dy;
    if dist2 <= ENCODER_BUTTON_R * ENCODER_BUTTON_R {
        input.set_mouse_button(Button::Encoder, "mouse");
        return true;
    }
    if dist2 <= ENCODER_RING_R * ENCODER_RING_R {
        if dx >= 0 {
            input.rotate(Rotation::Clockwise, "mouse");
        } else {
            input.rotate(Rotation::CounterClockwise, "mouse");
        }
        return true;
    }

    false
}

pub fn handle_panel_mouse_up(_x: i32, _y: i32) {
    INPUT.lock().unwrap().release_mouse_button("mouse");
}

pub fn handle_panel_wheel(y: i32) {
    let mut input = INPUT.lock().unwrap();
    if y > 0 {
        input.rotate(Rotation::Clockwise, "mouse_wheel");
    } else if y < 0 {
        input.rotate(Rotation::CounterClockwise, "mouse_wheel");
    }
}

pub fn draw_control_panel(
    canvas: &mut Canvas<Window>,
    origin_x: i32,
    height: u32,
) -> std::result::Result<(), String> {
    let snapshot = INPUT.lock().unwrap().snapshot();

    canvas.set_draw_color(Color::RGB(24, 27, 31));
    canvas.fill_rect(Rect::new(origin_x, 0, PANEL_WIDTH, height))?;

    canvas.set_draw_color(Color::RGB(70, 75, 82));
    canvas.draw_line(Point::new(origin_x, 0), Point::new(origin_x, height as i32))?;

    draw_panel_button(canvas, origin_x, POWER_RECT, snapshot.power_pressed)?;
    draw_power_icon(
        canvas,
        origin_x + POWER_RECT.x + POWER_RECT.w / 2,
        POWER_RECT.y + POWER_RECT.h / 2,
        snapshot.power_pressed,
    )?;

    draw_panel_button(canvas, origin_x, HOME_RECT, snapshot.home_pressed)?;
    draw_home_icon(
        canvas,
        origin_x + HOME_RECT.x + HOME_RECT.w / 2,
        HOME_RECT.y + HOME_RECT.h / 2,
        snapshot.home_pressed,
    )?;

    draw_encoder(canvas, origin_x, snapshot.encoder_pressed)?;

    Ok(())
}

pub fn tick(sys: &System) {
    let (transitions, rotary_updates) = {
        let mut input = INPUT.lock().unwrap();
        let transitions = input.advance_encoder();
        let rotary_updates = input.drain_rotary_updates();
        (transitions, rotary_updates)
    };
    for transition in transitions {
        sys.p
            .signal_gpio_input(sys, Pin::from_str(&transition.pin_name), transition.level);
    }
    for update in rotary_updates {
        apply_rotary_counter_delta(sys, update.obj, update.delta);
    }
}

pub fn script_action(action: &str) -> bool {
    let normalized = action.trim().to_ascii_lowercase();
    let mut input = INPUT.lock().unwrap();

    match normalized.as_str() {
        "power" | "power_button" => {
            input.pulse_button(Button::Power, "script");
            true
        }
        "power_down" | "power_button_down" => {
            input.set_button(Button::Power, true, "script");
            true
        }
        "power_up" | "power_button_up" => {
            input.set_button(Button::Power, false, "script");
            true
        }
        "home" | "home_button" => {
            input.pulse_button(Button::Home, "script");
            true
        }
        "home_down" | "home_button_down" => {
            input.set_button(Button::Home, true, "script");
            true
        }
        "home_up" | "home_button_up" => {
            input.set_button(Button::Home, false, "script");
            true
        }
        "encoder" | "encoder_button" => {
            input.pulse_button(Button::Encoder, "script");
            true
        }
        "encoder_down" | "encoder_button_down" => {
            input.set_button(Button::Encoder, true, "script");
            true
        }
        "encoder_up" | "encoder_button_up" => {
            input.set_button(Button::Encoder, false, "script");
            true
        }
        "cw" | "clockwise" | "encoder_cw" => {
            input.rotate(Rotation::Clockwise, "script");
            true
        }
        "ccw" | "counterclockwise" | "counter_clockwise" | "encoder_ccw" => {
            input.rotate(Rotation::CounterClockwise, "script");
            true
        }
        _ => {
            warn!(
                "front_panel_event source=script action={} result=unknown_action",
                action
            );
            false
        }
    }
}

fn handle_key_down(keycode: Keycode) -> bool {
    let mut input = INPUT.lock().unwrap();
    match keycode {
        Keycode::P => input.set_button(Button::Power, true, "keyboard:P"),
        Keycode::H => input.set_button(Button::Home, true, "keyboard:H"),
        Keycode::Return => input.set_button(Button::Encoder, true, "keyboard:Return"),
        Keycode::Space => input.set_button(Button::Encoder, true, "keyboard:Space"),
        Keycode::Right => input.rotate(Rotation::Clockwise, "keyboard:Right"),
        Keycode::Down => input.rotate(Rotation::Clockwise, "keyboard:Down"),
        Keycode::Left => input.rotate(Rotation::CounterClockwise, "keyboard:Left"),
        Keycode::Up => input.rotate(Rotation::CounterClockwise, "keyboard:Up"),
        _ => return false,
    }
    true
}

fn handle_key_up(keycode: Keycode) -> bool {
    let mut input = INPUT.lock().unwrap();
    match keycode {
        Keycode::P => input.set_button(Button::Power, false, "keyboard:P"),
        Keycode::H => input.set_button(Button::Home, false, "keyboard:H"),
        Keycode::Return => input.set_button(Button::Encoder, false, "keyboard:Return"),
        Keycode::Space => input.set_button(Button::Encoder, false, "keyboard:Space"),
        _ => return false,
    }
    true
}

fn read_button(button: Button) -> bool {
    INPUT.lock().unwrap().read_button(button)
}

fn read_encoder(line: EncoderLine) -> bool {
    INPUT.lock().unwrap().read_encoder(line)
}

#[derive(Clone, Copy)]
enum Button {
    Power,
    Home,
    Encoder,
}

#[derive(Clone, Copy)]
enum EncoderLine {
    A,
    B,
}

#[derive(Clone, Copy)]
enum Rotation {
    Clockwise,
    CounterClockwise,
}

struct InputState {
    idle_high: bool,
    encoder_idle_high: bool,
    power_pin: String,
    home_pin: String,
    encoder_button_pin: String,
    encoder_a_pin: String,
    encoder_b_pin: String,
    rotary_provider_obj: Option<u32>,
    power_pressed: bool,
    home_pressed: bool,
    encoder_pressed: bool,
    encoder_a: bool,
    encoder_b: bool,
    encoder_steps: VecDeque<(bool, bool)>,
    encoder_phase_ticks: u16,
    encoder_phase_holding: bool,
    encoder_phase_observed: bool,
    pending_transitions: VecDeque<InputTransition>,
    mouse_held_button: Option<Button>,
    power_release_ticks: u16,
    home_release_ticks: u16,
    encoder_release_ticks: u16,
    power_active_read_logs: u8,
    home_active_read_logs: u8,
    encoder_active_read_logs: u8,
    encoder_a_active_read_logs: u8,
    encoder_b_active_read_logs: u8,
    pending_rotary_deltas: VecDeque<i16>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            idle_high: true,
            encoder_idle_high: true,
            power_pin: "unmapped".to_string(),
            home_pin: "unmapped".to_string(),
            encoder_button_pin: "unmapped".to_string(),
            encoder_a_pin: "unmapped".to_string(),
            encoder_b_pin: "unmapped".to_string(),
            rotary_provider_obj: None,
            power_pressed: false,
            home_pressed: false,
            encoder_pressed: false,
            encoder_a: true,
            encoder_b: true,
            encoder_steps: VecDeque::new(),
            encoder_phase_ticks: 0,
            encoder_phase_holding: false,
            encoder_phase_observed: false,
            pending_transitions: VecDeque::new(),
            mouse_held_button: None,
            power_release_ticks: 0,
            home_release_ticks: 0,
            encoder_release_ticks: 0,
            power_active_read_logs: 0,
            home_active_read_logs: 0,
            encoder_active_read_logs: 0,
            encoder_a_active_read_logs: 0,
            encoder_b_active_read_logs: 0,
            pending_rotary_deltas: VecDeque::new(),
        }
    }
}

impl InputState {
    fn configure(&mut self, config: &FrontPanelConfig, idle_high: bool, encoder_idle_high: bool) {
        self.idle_high = idle_high;
        self.encoder_idle_high = encoder_idle_high;
        self.power_pin = config
            .power_button_pin
            .clone()
            .unwrap_or_else(|| "unmapped".to_string());
        self.home_pin = config
            .home_button_pin
            .clone()
            .unwrap_or_else(|| "unmapped".to_string());
        self.encoder_button_pin = config
            .encoder_button_pin
            .clone()
            .unwrap_or_else(|| "unmapped".to_string());
        self.encoder_a_pin = config
            .encoder_a_pin
            .clone()
            .unwrap_or_else(|| "unmapped".to_string());
        self.encoder_b_pin = config
            .encoder_b_pin
            .clone()
            .unwrap_or_else(|| "unmapped".to_string());
        self.rotary_provider_obj = config
            .rotary_provider_obj
            .as_deref()
            .and_then(parse_u32_config);
        self.power_pressed = false;
        self.home_pressed = false;
        self.encoder_pressed = false;
        self.encoder_a = encoder_idle_high;
        self.encoder_b = encoder_idle_high;
        self.encoder_steps.clear();
        self.encoder_phase_ticks = 0;
        self.encoder_phase_holding = false;
        self.encoder_phase_observed = false;
        self.pending_transitions.clear();
        self.mouse_held_button = None;
        self.power_release_ticks = 0;
        self.home_release_ticks = 0;
        self.encoder_release_ticks = 0;
        self.power_active_read_logs = 0;
        self.home_active_read_logs = 0;
        self.encoder_active_read_logs = 0;
        self.encoder_a_active_read_logs = 0;
        self.encoder_b_active_read_logs = 0;
        self.pending_rotary_deltas.clear();

        info!(
            "front_panel_config power_button_pin={} home_button_pin={} encoder_button_pin={} encoder_a_pin={} encoder_b_pin={} idle_high={} encoder_idle_high={} rotary_provider_obj={}",
            self.power_pin,
            self.home_pin,
            self.encoder_button_pin,
            self.encoder_a_pin,
            self.encoder_b_pin,
            self.idle_high,
            self.encoder_idle_high,
            self.rotary_provider_obj
                .map(|obj| format!("0x{obj:08x}"))
                .unwrap_or_else(|| "none".to_string())
        );
    }

    fn set_button(&mut self, button: Button, pressed: bool, source: &str) {
        if pressed {
            match button {
                Button::Power => {
                    self.power_pressed = true;
                    self.power_release_ticks = 0;
                    self.power_active_read_logs = BUTTON_ACTIVE_READ_LOG_LIMIT;
                }
                Button::Home => {
                    self.home_pressed = true;
                    self.home_release_ticks = 0;
                    self.home_active_read_logs = BUTTON_ACTIVE_READ_LOG_LIMIT;
                }
                Button::Encoder => {
                    self.encoder_pressed = true;
                    self.encoder_release_ticks = 0;
                    self.encoder_active_read_logs = BUTTON_ACTIVE_READ_LOG_LIMIT;
                }
            }
            self.queue_button_transition(button, !self.idle_high);
            info!(
                "front_panel_event source={} event={} pin={} result=pressed active_level={}",
                source,
                button.name(),
                self.button_pin(button),
                !self.idle_high
            );
        } else {
            match button {
                Button::Power => self.power_release_ticks = BUTTON_RELEASE_HOLD_TICKS,
                Button::Home => self.home_release_ticks = BUTTON_RELEASE_HOLD_TICKS,
                Button::Encoder => self.encoder_release_ticks = BUTTON_RELEASE_HOLD_TICKS,
            }
            info!(
                "front_panel_event source={} event={} pin={} result=release_scheduled hold_ticks={}",
                source,
                button.name(),
                self.button_pin(button),
                BUTTON_RELEASE_HOLD_TICKS
            );
        }
    }

    fn pulse_button(&mut self, button: Button, source: &str) {
        self.set_button(button, true, source);
        self.set_button(button, false, source);
    }

    fn set_mouse_button(&mut self, button: Button, source: &str) {
        self.mouse_held_button = Some(button);
        self.set_button(button, true, source);
    }

    fn release_mouse_button(&mut self, source: &str) {
        if let Some(button) = self.mouse_held_button.take() {
            self.set_button(button, false, source);
        }
    }

    fn snapshot(&self) -> PanelSnapshot {
        PanelSnapshot {
            power_pressed: self.power_pressed,
            home_pressed: self.home_pressed,
            encoder_pressed: self.encoder_pressed,
        }
    }

    fn read_button(&mut self, button: Button) -> bool {
        let pressed = match button {
            Button::Power => self.power_pressed,
            Button::Home => self.home_pressed,
            Button::Encoder => self.encoder_pressed,
        };

        if pressed {
            let log_active_read = {
                let remaining = self.active_read_logs_mut(button);
                if *remaining > 0 {
                    *remaining -= 1;
                    true
                } else {
                    false
                }
            };
            if log_active_read {
                info!(
                    "front_panel_read event={} pin={} level={} result=active",
                    button.name(),
                    self.button_pin(button),
                    !self.idle_high
                );
            }
            !self.idle_high
        } else {
            self.idle_high
        }
    }

    fn read_encoder(&mut self, line: EncoderLine) -> bool {
        if self.encoder_phase_holding {
            self.encoder_phase_observed = true;
        }

        let level = match line {
            EncoderLine::A => self.encoder_a,
            EncoderLine::B => self.encoder_b,
        };

        if level != self.encoder_idle_high {
            let pin_name = self.encoder_pin(line).to_string();
            let line_name = line.name();
            let remaining = self.encoder_active_read_logs_mut(line);
            if *remaining > 0 {
                *remaining -= 1;
                info!(
                    "front_panel_read event=encoder_rotation line={} pin={} level={} result=active",
                    line_name, pin_name, level
                );
            }
        }

        level
    }

    fn rotate(&mut self, rotation: Rotation, source: &str) {
        let idle = self.encoder_idle_high;
        let active = !idle;
        let uses_rotary_provider = self.rotary_provider_obj.is_some();
        if uses_rotary_provider {
            self.encoder_steps.clear();
            self.encoder_a = idle;
            self.encoder_b = idle;
            self.encoder_phase_ticks = 0;
            self.encoder_phase_holding = false;
            self.encoder_phase_observed = false;
            self.pending_transitions.clear();
        } else {
            let steps = match rotation {
                Rotation::Clockwise => [
                    (active, idle),
                    (active, active),
                    (idle, active),
                    (idle, idle),
                ],
                Rotation::CounterClockwise => [
                    (idle, active),
                    (active, active),
                    (active, idle),
                    (idle, idle),
                ],
            };
            self.encoder_steps.extend(steps);
            self.encoder_a_active_read_logs = ENCODER_ACTIVE_READ_LOG_LIMIT;
            self.encoder_b_active_read_logs = ENCODER_ACTIVE_READ_LOG_LIMIT;
        }
        self.pending_rotary_deltas.push_back(match rotation {
            Rotation::Clockwise => 1,
            Rotation::CounterClockwise => -1,
        });
        info!(
            "front_panel_event source={} event=encoder_rotation direction={} encoder_a_pin={} encoder_b_pin={} rotary_provider_bridge={} queued_steps={} phase_min_hold_ticks={} phase_max_hold_ticks={}",
            source,
            rotation.name(),
            self.encoder_a_pin,
            self.encoder_b_pin,
            uses_rotary_provider,
            self.encoder_steps.len(),
            ENCODER_PHASE_MIN_HOLD_TICKS,
            ENCODER_PHASE_MAX_HOLD_TICKS
        );
    }

    fn advance_encoder(&mut self) -> Vec<InputTransition> {
        let hold_current_phase = if self.encoder_phase_holding {
            self.encoder_phase_ticks = self.encoder_phase_ticks.saturating_add(1);
            let observed_ready = self.encoder_phase_observed
                && self.encoder_phase_ticks >= ENCODER_PHASE_MIN_HOLD_TICKS;
            let timed_out = self.encoder_phase_ticks >= ENCODER_PHASE_MAX_HOLD_TICKS;
            !(observed_ready || timed_out)
        } else {
            false
        };

        if !hold_current_phase {
            self.advance_encoder_phase();
        }

        if Self::advance_button_release(&mut self.power_pressed, &mut self.power_release_ticks) {
            self.queue_button_transition(Button::Power, self.idle_high);
        }
        if Self::advance_button_release(&mut self.home_pressed, &mut self.home_release_ticks) {
            self.queue_button_transition(Button::Home, self.idle_high);
        }
        if Self::advance_button_release(&mut self.encoder_pressed, &mut self.encoder_release_ticks)
        {
            self.queue_button_transition(Button::Encoder, self.idle_high);
        }
        self.pending_transitions.drain(..).collect()
    }

    fn advance_encoder_phase(&mut self) {
        if let Some((a, b)) = self.encoder_steps.pop_front() {
            if self.encoder_a != a {
                let pin_name = self.encoder_a_pin.clone();
                self.queue_pin_transition(&pin_name, a);
            }
            if self.encoder_b != b {
                let pin_name = self.encoder_b_pin.clone();
                self.queue_pin_transition(&pin_name, b);
            }
            self.encoder_a = a;
            self.encoder_b = b;
            self.encoder_phase_ticks = 0;
            self.encoder_phase_holding = true;
            self.encoder_phase_observed = false;
        } else {
            self.encoder_phase_ticks = 0;
            self.encoder_phase_holding = false;
            self.encoder_phase_observed = false;
        }
    }

    fn drain_rotary_updates(&mut self) -> Vec<RotaryUpdate> {
        let Some(obj) = self.rotary_provider_obj else {
            self.pending_rotary_deltas.clear();
            return Vec::new();
        };

        self.pending_rotary_deltas
            .drain(..)
            .map(|delta| RotaryUpdate { obj, delta })
            .collect()
    }

    fn advance_button_release(pressed: &mut bool, release_ticks: &mut u16) -> bool {
        if *release_ticks == 0 {
            return false;
        }
        *release_ticks -= 1;
        if *release_ticks == 0 {
            *pressed = false;
            true
        } else {
            false
        }
    }

    fn button_pin(&self, button: Button) -> &str {
        match button {
            Button::Power => &self.power_pin,
            Button::Home => &self.home_pin,
            Button::Encoder => &self.encoder_button_pin,
        }
    }

    fn active_read_logs_mut(&mut self, button: Button) -> &mut u8 {
        match button {
            Button::Power => &mut self.power_active_read_logs,
            Button::Home => &mut self.home_active_read_logs,
            Button::Encoder => &mut self.encoder_active_read_logs,
        }
    }

    fn encoder_active_read_logs_mut(&mut self, line: EncoderLine) -> &mut u8 {
        match line {
            EncoderLine::A => &mut self.encoder_a_active_read_logs,
            EncoderLine::B => &mut self.encoder_b_active_read_logs,
        }
    }

    fn encoder_pin(&self, line: EncoderLine) -> &str {
        match line {
            EncoderLine::A => &self.encoder_a_pin,
            EncoderLine::B => &self.encoder_b_pin,
        }
    }

    fn queue_button_transition(&mut self, button: Button, level: bool) {
        let pin_name = self.button_pin(button).to_string();
        self.queue_pin_transition(&pin_name, level);
    }

    fn queue_pin_transition(&mut self, pin_name: &str, level: bool) {
        if pin_name == "unmapped" {
            return;
        }
        self.pending_transitions.push_back(InputTransition {
            pin_name: pin_name.to_string(),
            level,
        });
    }
}

struct InputTransition {
    pin_name: String,
    level: bool,
}

struct RotaryUpdate {
    obj: u32,
    delta: i16,
}

fn parse_u32_config(value: &str) -> Option<u32> {
    let text = value.trim();
    let parsed = if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16)
    } else {
        text.parse::<u32>()
    };
    match parsed {
        Ok(value) => Some(value),
        Err(err) => {
            warn!(
                "front_panel_config rotary_provider_obj={} result=parse_failed error={}",
                value, err
            );
            None
        }
    }
}

fn apply_rotary_counter_delta(sys: &System, obj: u32, delta: i16) {
    let count_addr = obj.wrapping_add(4);
    let direction_addr = obj.wrapping_add(20);
    let pending_addr = obj.wrapping_add(29);
    let mut count_bytes = [0u8; 2];
    let mut uc = sys.uc.borrow_mut();

    if let Err(err) = uc.mem_read(count_addr.into(), &mut count_bytes) {
        warn!(
            "front_panel_rotary_counter_bridge obj=0x{:08x} delta={} result=read_failed error={:?}",
            obj, delta, err
        );
        return;
    }

    let current = i16::from_le_bytes(count_bytes);
    let next = current.saturating_add(delta);
    let direction = if delta > 0 { 1 } else { 2 };

    let count_result = uc.mem_write(count_addr.into(), &next.to_le_bytes());
    let direction_result = uc.mem_write(direction_addr.into(), &[direction]);
    let pending_result = uc.mem_write(pending_addr.into(), &[0]);
    if let Err(err) = count_result.and(direction_result).and(pending_result) {
        warn!(
            "front_panel_rotary_counter_bridge obj=0x{:08x} delta={} result=write_failed error={:?}",
            obj, delta, err
        );
        return;
    }

    info!(
        "front_panel_rotary_counter_bridge obj=0x{:08x} delta={} count={} direction={} result=ok",
        obj, delta, next, direction
    );
}

impl Button {
    fn name(self) -> &'static str {
        match self {
            Button::Power => "power_button",
            Button::Home => "home_button",
            Button::Encoder => "encoder_button",
        }
    }
}

impl Rotation {
    fn name(self) -> &'static str {
        match self {
            Rotation::Clockwise => "clockwise",
            Rotation::CounterClockwise => "counterclockwise",
        }
    }
}

impl EncoderLine {
    fn name(self) -> &'static str {
        match self {
            EncoderLine::A => "A",
            EncoderLine::B => "B",
        }
    }
}

#[derive(Clone, Copy)]
struct PanelRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl PanelRect {
    const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }

    fn to_sdl(self, origin_x: i32) -> Rect {
        Rect::new(origin_x + self.x, self.y, self.w as u32, self.h as u32)
    }
}

struct PanelSnapshot {
    power_pressed: bool,
    home_pressed: bool,
    encoder_pressed: bool,
}

fn draw_panel_button(
    canvas: &mut Canvas<Window>,
    origin_x: i32,
    rect: PanelRect,
    pressed: bool,
) -> std::result::Result<(), String> {
    canvas.set_draw_color(if pressed {
        Color::RGB(65, 88, 101)
    } else {
        Color::RGB(38, 43, 49)
    });
    canvas.fill_rect(rect.to_sdl(origin_x))?;
    canvas.set_draw_color(if pressed {
        Color::RGB(134, 209, 226)
    } else {
        Color::RGB(104, 114, 124)
    });
    canvas.draw_rect(rect.to_sdl(origin_x))?;
    Ok(())
}

fn draw_power_icon(
    canvas: &mut Canvas<Window>,
    cx: i32,
    cy: i32,
    pressed: bool,
) -> std::result::Result<(), String> {
    canvas.set_draw_color(if pressed {
        Color::RGB(170, 239, 242)
    } else {
        Color::RGB(204, 215, 224)
    });
    draw_circle(canvas, cx, cy + 3, 14)?;
    canvas.draw_line(Point::new(cx, cy - 14), Point::new(cx, cy + 2))?;
    canvas.draw_line(Point::new(cx - 1, cy - 14), Point::new(cx - 1, cy + 2))?;
    Ok(())
}

fn draw_home_icon(
    canvas: &mut Canvas<Window>,
    cx: i32,
    cy: i32,
    pressed: bool,
) -> std::result::Result<(), String> {
    canvas.set_draw_color(if pressed {
        Color::RGB(170, 239, 242)
    } else {
        Color::RGB(204, 215, 224)
    });
    canvas.draw_line(Point::new(cx - 18, cy), Point::new(cx, cy - 15))?;
    canvas.draw_line(Point::new(cx, cy - 15), Point::new(cx + 18, cy))?;
    canvas.draw_line(Point::new(cx - 13, cy), Point::new(cx - 13, cy + 17))?;
    canvas.draw_line(Point::new(cx + 13, cy), Point::new(cx + 13, cy + 17))?;
    canvas.draw_line(Point::new(cx - 13, cy + 17), Point::new(cx + 13, cy + 17))?;
    Ok(())
}

fn draw_encoder(
    canvas: &mut Canvas<Window>,
    origin_x: i32,
    pressed: bool,
) -> std::result::Result<(), String> {
    let cx = origin_x + ENCODER_CX;
    let cy = ENCODER_CY;

    canvas.set_draw_color(Color::RGB(31, 36, 42));
    fill_circle(canvas, cx, cy, ENCODER_RING_R)?;
    canvas.set_draw_color(Color::RGB(109, 119, 130));
    draw_circle(canvas, cx, cy, ENCODER_RING_R)?;
    draw_circle(canvas, cx, cy, ENCODER_RING_R - 1)?;

    canvas.set_draw_color(Color::RGB(185, 198, 209));
    draw_ccw_arrow(canvas, cx - 34, cy)?;
    draw_cw_arrow(canvas, cx + 34, cy)?;

    canvas.set_draw_color(if pressed {
        Color::RGB(65, 88, 101)
    } else {
        Color::RGB(43, 49, 56)
    });
    fill_circle(canvas, cx, cy, ENCODER_BUTTON_R)?;
    canvas.set_draw_color(if pressed {
        Color::RGB(170, 239, 242)
    } else {
        Color::RGB(204, 215, 224)
    });
    draw_circle(canvas, cx, cy, ENCODER_BUTTON_R)?;
    canvas.draw_line(Point::new(cx - 9, cy), Point::new(cx + 9, cy))?;
    canvas.draw_line(Point::new(cx, cy - 9), Point::new(cx, cy + 9))?;

    Ok(())
}

fn draw_ccw_arrow(
    canvas: &mut Canvas<Window>,
    cx: i32,
    cy: i32,
) -> std::result::Result<(), String> {
    canvas.draw_line(Point::new(cx + 11, cy - 11), Point::new(cx - 7, cy))?;
    canvas.draw_line(Point::new(cx - 7, cy), Point::new(cx + 11, cy + 11))?;
    canvas.draw_line(Point::new(cx - 7, cy), Point::new(cx + 4, cy))?;
    Ok(())
}

fn draw_cw_arrow(canvas: &mut Canvas<Window>, cx: i32, cy: i32) -> std::result::Result<(), String> {
    canvas.draw_line(Point::new(cx - 11, cy - 11), Point::new(cx + 7, cy))?;
    canvas.draw_line(Point::new(cx + 7, cy), Point::new(cx - 11, cy + 11))?;
    canvas.draw_line(Point::new(cx - 4, cy), Point::new(cx + 7, cy))?;
    Ok(())
}

fn draw_circle(
    canvas: &mut Canvas<Window>,
    cx: i32,
    cy: i32,
    r: i32,
) -> std::result::Result<(), String> {
    let mut x = r;
    let mut y = 0;
    let mut err = 1 - x;
    while x >= y {
        draw_circle_points(canvas, cx, cy, x, y)?;
        y += 1;
        if err < 0 {
            err += 2 * y + 1;
        } else {
            x -= 1;
            err += 2 * (y - x) + 1;
        }
    }
    Ok(())
}

fn draw_circle_points(
    canvas: &mut Canvas<Window>,
    cx: i32,
    cy: i32,
    x: i32,
    y: i32,
) -> std::result::Result<(), String> {
    canvas.draw_point(Point::new(cx + x, cy + y))?;
    canvas.draw_point(Point::new(cx + y, cy + x))?;
    canvas.draw_point(Point::new(cx - y, cy + x))?;
    canvas.draw_point(Point::new(cx - x, cy + y))?;
    canvas.draw_point(Point::new(cx - x, cy - y))?;
    canvas.draw_point(Point::new(cx - y, cy - x))?;
    canvas.draw_point(Point::new(cx + y, cy - x))?;
    canvas.draw_point(Point::new(cx + x, cy - y))?;
    Ok(())
}

fn fill_circle(
    canvas: &mut Canvas<Window>,
    cx: i32,
    cy: i32,
    r: i32,
) -> std::result::Result<(), String> {
    for dy in -r..=r {
        let dx = ((r * r - dy * dy) as f64).sqrt() as i32;
        canvas.draw_line(Point::new(cx - dx, cy + dy), Point::new(cx + dx, cy + dy))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoder_button_release_keeps_active_level_for_scan_window() {
        let mut input = InputState::default();
        input.configure(&FrontPanelConfig::default(), true, true);

        input.set_button(Button::Encoder, true, "test");
        assert!(!input.read_button(Button::Encoder));

        input.set_button(Button::Encoder, false, "test");
        assert!(!input.read_button(Button::Encoder));

        for _ in 0..BUTTON_RELEASE_HOLD_TICKS - 1 {
            input.advance_encoder();
            assert!(!input.read_button(Button::Encoder));
        }

        input.advance_encoder();
        assert!(input.read_button(Button::Encoder));
    }

    #[test]
    fn encoder_rotation_waits_for_a_firmware_scan_before_advancing_phase() {
        let mut input = InputState::default();
        input.configure(&FrontPanelConfig::default(), true, true);

        input.rotate(Rotation::Clockwise, "test");
        input.advance_encoder();
        assert!(!input.read_encoder(EncoderLine::A));
        assert!(input.read_encoder(EncoderLine::B));

        for _ in 0..ENCODER_PHASE_MIN_HOLD_TICKS - 1 {
            input.advance_encoder();
            assert!(!input.read_encoder(EncoderLine::A));
            assert!(input.read_encoder(EncoderLine::B));
        }

        input.advance_encoder();
        assert!(!input.read_encoder(EncoderLine::A));
        assert!(!input.read_encoder(EncoderLine::B));
    }

    #[test]
    fn rotary_provider_bridge_does_not_queue_raw_encoder_phases() {
        let mut input = InputState::default();
        input.configure(
            &FrontPanelConfig {
                rotary_provider_obj: Some("0x200174e4".to_string()),
                ..FrontPanelConfig::default()
            },
            true,
            true,
        );

        input.rotate(Rotation::Clockwise, "test");

        assert!(input.advance_encoder().is_empty());
        assert_eq!(input.encoder_steps.len(), 0);
        assert!(input.read_encoder(EncoderLine::A));
        assert!(input.read_encoder(EncoderLine::B));

        let updates = input.drain_rotary_updates();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].obj, 0x200174e4);
        assert_eq!(updates[0].delta, 1);
    }
}
