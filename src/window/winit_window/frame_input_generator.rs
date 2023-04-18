use super::FrameInput;
use crate::control::*;
use crate::core::*;
#[cfg(target_arch = "wasm32")]
use instant::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
use winit::dpi::PhysicalSize;
use winit::event::TouchPhase;
use winit::event::WindowEvent;

///
/// Use this to generate [FrameInput] for a new frame with a custom [winit](https://crates.io/crates/winit) window.
/// [FrameInput] is automatically generated if using the default [Window](crate::window::Window).
///
pub struct FrameInputGenerator {
    last_time: Instant,
    first_frame: bool,
    events: Vec<Event>,
    accumulated_time: f64,
    viewport: Viewport,
    window_width: u32,
    window_height: u32,
    device_pixel_ratio: f64,
    cursor_pos: Option<LogicalPoint>,
    finger_id: Option<u64>,
    secondary_cursor_pos: Option<LogicalPoint>,
    secondary_finger_id: Option<u64>,
    modifiers: Modifiers,
    mouse_pressed: Option<MouseButton>,
}

impl FrameInputGenerator {
    ///
    /// Creates a new frame input generator.
    ///
    fn new(size: PhysicalSize<u32>, device_pixel_ratio: f64) -> Self {
        let (window_width, window_height): (u32, u32) =
            size.to_logical::<f32>(device_pixel_ratio).into();
        Self {
            events: Vec::new(),
            accumulated_time: 0.0,
            viewport: Viewport::new_at_origo(size.width, size.height),
            window_width,
            window_height,
            device_pixel_ratio,
            first_frame: true,
            last_time: Instant::now(),
            cursor_pos: None,
            finger_id: None,
            secondary_cursor_pos: None,
            secondary_finger_id: None,
            modifiers: Modifiers::default(),
            mouse_pressed: None,
        }
    }

    ///
    /// Creates a new frame input generator from a [winit](https://crates.io/crates/winit) window.
    ///
    pub fn from_winit_window(window: &winit::window::Window) -> Self {
        Self::new(window.inner_size(), window.scale_factor())
    }

    ///
    /// Generates [FrameInput] for a new frame. This should be called each frame and the generated data should only be used for one frame.
    ///
    pub fn generate(&mut self, context: &Context) -> FrameInput {
        let now = Instant::now();
        let duration = now.duration_since(self.last_time);
        let elapsed_time =
            duration.as_secs() as f64 * 1000.0 + duration.subsec_nanos() as f64 * 1e-6;
        self.accumulated_time += elapsed_time;
        self.last_time = now;

        let frame_input = FrameInput {
            events: self.events.drain(..).collect(),
            elapsed_time,
            accumulated_time: self.accumulated_time,
            viewport: self.viewport,
            window_width: self.window_width,
            window_height: self.window_height,
            device_pixel_ratio: self.device_pixel_ratio as f32,
            first_frame: self.first_frame,
            context: context.clone(),
        };
        self.first_frame = false;

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(exit_time) = option_env!("THREE_D_EXIT").map(|v| v.parse::<f64>().unwrap()) {
            if exit_time < frame_input.accumulated_time {
                #[cfg(feature = "image")]
                if let Some(path) = option_env!("THREE_D_SCREENSHOT") {
                    let pixels = frame_input.screen().read_color::<[u8; 4]>();
                    let img = image::DynamicImage::ImageRgba8(
                        image::ImageBuffer::from_raw(
                            frame_input.viewport.width,
                            frame_input.viewport.height,
                            pixels.into_iter().flatten().collect::<Vec<_>>(),
                        )
                        .unwrap(),
                    );
                    img.resize(
                        frame_input.window_width,
                        frame_input.window_height,
                        image::imageops::FilterType::Triangle,
                    )
                    .save(path)
                    .unwrap();
                }
                std::process::exit(0);
            }
        }
        frame_input
    }

    ///
    /// Handle the [WindowEvent] generated by a [winit](https://crates.io/crates/winit) event loop.
    ///
    pub fn handle_winit_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::Resized(physical_size) => {
                self.viewport = Viewport::new_at_origo(physical_size.width, physical_size.height);
                let logical_size = physical_size.to_logical(self.device_pixel_ratio);
                self.window_width = logical_size.width;
                self.window_height = logical_size.height;
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                self.device_pixel_ratio = *scale_factor;
                self.viewport = Viewport::new_at_origo(new_inner_size.width, new_inner_size.height);
                let logical_size = new_inner_size.to_logical(self.device_pixel_ratio);
                self.window_width = logical_size.width;
                self.window_height = logical_size.height;
            }
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(keycode) = input.virtual_keycode {
                    use winit::event::VirtualKeyCode;
                    let state = input.state == winit::event::ElementState::Pressed;
                    if let Some(kind) = translate_virtual_key_code(keycode) {
                        self.events.push(if state {
                            crate::Event::KeyPress {
                                kind,
                                modifiers: self.modifiers,
                                handled: false,
                            }
                        } else {
                            crate::Event::KeyRelease {
                                kind,
                                modifiers: self.modifiers,
                                handled: false,
                            }
                        });
                    } else if keycode == VirtualKeyCode::LControl
                        || keycode == VirtualKeyCode::RControl
                    {
                        self.modifiers.ctrl = state;
                        if !cfg!(target_os = "macos") {
                            self.modifiers.command = state;
                        }
                        self.events.push(crate::Event::ModifiersChange {
                            modifiers: self.modifiers,
                        });
                    } else if keycode == VirtualKeyCode::LAlt || keycode == VirtualKeyCode::RAlt {
                        self.modifiers.alt = state;
                        self.events.push(crate::Event::ModifiersChange {
                            modifiers: self.modifiers,
                        });
                    } else if keycode == VirtualKeyCode::LShift || keycode == VirtualKeyCode::RShift
                    {
                        self.modifiers.shift = state;
                        self.events.push(crate::Event::ModifiersChange {
                            modifiers: self.modifiers,
                        });
                    } else if (keycode == VirtualKeyCode::LWin || keycode == VirtualKeyCode::RWin)
                        && cfg!(target_os = "macos")
                    {
                        self.modifiers.command = state;
                        self.events.push(crate::Event::ModifiersChange {
                            modifiers: self.modifiers,
                        });
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(position) = self.cursor_pos {
                    match delta {
                        winit::event::MouseScrollDelta::LineDelta(x, y) => {
                            let line_height = 24.0; // TODO
                            self.events.push(crate::Event::MouseWheel {
                                delta: (*x * line_height, *y * line_height),
                                position,
                                modifiers: self.modifiers,
                                handled: false,
                            });
                        }
                        winit::event::MouseScrollDelta::PixelDelta(delta) => {
                            let d = delta.to_logical(self.device_pixel_ratio);
                            self.events.push(crate::Event::MouseWheel {
                                delta: (d.x, d.y),
                                position,
                                modifiers: self.modifiers,
                                handled: false,
                            });
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(position) = self.cursor_pos {
                    let button = match button {
                        winit::event::MouseButton::Left => Some(crate::MouseButton::Left),
                        winit::event::MouseButton::Middle => Some(crate::MouseButton::Middle),
                        winit::event::MouseButton::Right => Some(crate::MouseButton::Right),
                        _ => None,
                    };
                    if let Some(b) = button {
                        self.events
                            .push(if *state == winit::event::ElementState::Pressed {
                                self.mouse_pressed = Some(b);
                                crate::Event::MousePress {
                                    button: b,
                                    position,
                                    modifiers: self.modifiers,
                                    handled: false,
                                }
                            } else {
                                self.mouse_pressed = None;
                                crate::Event::MouseRelease {
                                    button: b,
                                    position,
                                    modifiers: self.modifiers,
                                    handled: false,
                                }
                            });
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let p = position.to_logical(self.device_pixel_ratio);
                let delta = if let Some(last_pos) = self.cursor_pos {
                    (p.x - last_pos.x, p.y - last_pos.y)
                } else {
                    (0.0, 0.0)
                };
                let position = LogicalPoint {
                    x: p.x,
                    y: p.y,
                    device_pixel_ratio: self.device_pixel_ratio as f32,
                    height: self.viewport.height as f32,
                };
                self.events.push(crate::Event::MouseMotion {
                    button: self.mouse_pressed,
                    delta,
                    position,
                    modifiers: self.modifiers,
                    handled: false,
                });
                self.cursor_pos = Some(position);
            }
            WindowEvent::ReceivedCharacter(ch) => {
                if is_printable_char(*ch) && !self.modifiers.ctrl && !self.modifiers.command {
                    self.events.push(crate::Event::Text(ch.to_string()));
                }
            }
            WindowEvent::CursorEntered { .. } => {
                self.events.push(crate::Event::MouseEnter);
            }
            WindowEvent::CursorLeft { .. } => {
                self.mouse_pressed = None;
                self.events.push(crate::Event::MouseLeave);
            }
            WindowEvent::Touch(touch) => {
                let position = touch.location.to_logical::<f32>(self.device_pixel_ratio);
                let position = LogicalPoint {
                    x: position.x,
                    y: position.y,
                    device_pixel_ratio: self.device_pixel_ratio as f32,
                    height: self.viewport.height as f32,
                };
                match touch.phase {
                    TouchPhase::Started => {
                        if self.finger_id.is_none() {
                            self.events.push(crate::Event::MousePress {
                                button: MouseButton::Left,
                                position,
                                modifiers: self.modifiers,
                                handled: false,
                            });
                            self.cursor_pos = Some(position);
                            self.finger_id = Some(touch.id);
                        } else if self.secondary_finger_id.is_none() {
                            self.secondary_cursor_pos = Some(position);
                            self.secondary_finger_id = Some(touch.id);
                        }
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        if self.finger_id.map(|id| id == touch.id).unwrap_or(false) {
                            self.events.push(crate::Event::MouseRelease {
                                button: MouseButton::Left,
                                position,
                                modifiers: self.modifiers,
                                handled: false,
                            });
                            self.cursor_pos = None;
                            self.finger_id = None;
                        } else if self
                            .secondary_finger_id
                            .map(|id| id == touch.id)
                            .unwrap_or(false)
                        {
                            self.secondary_cursor_pos = None;
                            self.secondary_finger_id = None;
                        }
                    }
                    TouchPhase::Moved => {
                        if self.finger_id.map(|id| id == touch.id).unwrap_or(false) {
                            let last_pos = self.cursor_pos.unwrap();
                            if let Some(p) = self.secondary_cursor_pos {
                                self.events.push(crate::Event::MouseWheel {
                                    position,
                                    modifiers: self.modifiers,
                                    handled: false,
                                    delta: (
                                        (position.x - p.x).abs() - (last_pos.x - p.x).abs(),
                                        (position.y - p.y).abs() - (last_pos.y - p.y).abs(),
                                    ),
                                });
                            } else {
                                self.events.push(crate::Event::MouseMotion {
                                    button: Some(MouseButton::Left),
                                    position,
                                    modifiers: self.modifiers,
                                    handled: false,
                                    delta: (position.x - last_pos.x, position.y - last_pos.y),
                                });
                            }
                            self.cursor_pos = Some(position);
                        } else if self
                            .secondary_finger_id
                            .map(|id| id == touch.id)
                            .unwrap_or(false)
                        {
                            let last_pos = self.secondary_cursor_pos.unwrap();
                            if let Some(p) = self.cursor_pos {
                                self.events.push(crate::Event::MouseWheel {
                                    position: p,
                                    modifiers: self.modifiers,
                                    handled: false,
                                    delta: (
                                        (position.x - p.x).abs() - (last_pos.x - p.x).abs(),
                                        (position.y - p.y).abs() - (last_pos.y - p.y).abs(),
                                    ),
                                });
                            }
                            self.secondary_cursor_pos = Some(position);
                        }
                    }
                }
            }
            _ => (),
        }
    }
}

fn is_printable_char(chr: char) -> bool {
    let is_in_private_use_area = ('\u{e000}'..='\u{f8ff}').contains(&chr)
        || ('\u{f0000}'..='\u{ffffd}').contains(&chr)
        || ('\u{100000}'..='\u{10fffd}').contains(&chr);

    !is_in_private_use_area && !chr.is_ascii_control()
}

fn translate_virtual_key_code(key: winit::event::VirtualKeyCode) -> Option<crate::Key> {
    use winit::event::VirtualKeyCode::*;

    Some(match key {
        Down => Key::ArrowDown,
        Left => Key::ArrowLeft,
        Right => Key::ArrowRight,
        Up => Key::ArrowUp,

        Escape => Key::Escape,
        Tab => Key::Tab,
        Back => Key::Backspace,
        Return => Key::Enter,
        Space => Key::Space,

        Insert => Key::Insert,
        Delete => Key::Delete,
        Home => Key::Home,
        End => Key::End,
        PageUp => Key::PageUp,
        PageDown => Key::PageDown,

        Key0 | Numpad0 => Key::Num0,
        Key1 | Numpad1 => Key::Num1,
        Key2 | Numpad2 => Key::Num2,
        Key3 | Numpad3 => Key::Num3,
        Key4 | Numpad4 => Key::Num4,
        Key5 | Numpad5 => Key::Num5,
        Key6 | Numpad6 => Key::Num6,
        Key7 | Numpad7 => Key::Num7,
        Key8 | Numpad8 => Key::Num8,
        Key9 | Numpad9 => Key::Num9,

        A => Key::A,
        B => Key::B,
        C => Key::C,
        D => Key::D,
        E => Key::E,
        F => Key::F,
        G => Key::G,
        H => Key::H,
        I => Key::I,
        J => Key::J,
        K => Key::K,
        L => Key::L,
        M => Key::M,
        N => Key::N,
        O => Key::O,
        P => Key::P,
        Q => Key::Q,
        R => Key::R,
        S => Key::S,
        T => Key::T,
        U => Key::U,
        V => Key::V,
        W => Key::W,
        X => Key::X,
        Y => Key::Y,
        Z => Key::Z,

        _ => {
            return None;
        }
    })
}
