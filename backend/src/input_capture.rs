use rdev::{grab, Event, EventType, Key};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct InputEventData {
    pub event_type: String,
    pub key: Option<String>,
    pub key_code: Option<u32>, // Added key_code
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub dx: Option<f64>,
    pub dy: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum CaptureControl {
    InputEvent(InputEventData),
    ExitRequested,
}


#[cfg(windows)]
extern "system" {
    fn SetCursorPos(x: i32, y: i32) -> i32;
}

pub struct InputCapture {
    tx: mpsc::UnboundedSender<CaptureControl>,
    should_stop: Arc<AtomicBool>,
}

impl InputCapture {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<CaptureControl>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let should_stop = Arc::new(AtomicBool::new(false));
        (Self { tx, should_stop }, rx)
    }

    pub fn start_capture(self: Arc<Self>) {
        let tx = self.tx.clone();
        let should_stop = Arc::clone(&self.should_stop);
        
        // Track modifier keys
        let ctrl_pressed = Arc::new(AtomicBool::new(false));
        let alt_pressed = Arc::new(AtomicBool::new(false));
        
        // Spawn blocking thread for rdev grab
        std::thread::spawn(move || {
            let ctrl_pressed_clone = Arc::clone(&ctrl_pressed);
            let alt_pressed_clone = Arc::clone(&alt_pressed);
            let tx_clone = tx.clone();
            let should_stop_clone = Arc::clone(&should_stop);
            
            // Center position for virtual mouse trap
            const CENTER_X: i32 = 500;
            const CENTER_Y: i32 = 500;
            
            // Track previous mouse position for delta calculation
            use std::sync::Mutex;
            let last_mouse_pos = Arc::new(Mutex::new(Option::<(f64, f64)>::None));
            let last_mouse_pos_clone = Arc::clone(&last_mouse_pos);
            
            // Initialize cursor to center
            #[cfg(windows)]
            unsafe {
                SetCursorPos(CENTER_X, CENTER_Y);
            }
            *last_mouse_pos.lock().unwrap() = Some((CENTER_X as f64, CENTER_Y as f64));
            
            let callback = move |event: Event| -> Option<Event> {
                // Check if we should stop
                if should_stop_clone.load(Ordering::Relaxed) {
                    return Some(event); // Pass through all events
                }
                
                // Track modifier keys
                match &event.event_type {
                    EventType::KeyPress(Key::ControlLeft) | EventType::KeyPress(Key::ControlRight) => {
                        ctrl_pressed_clone.store(true, Ordering::Relaxed);
                    }
                    EventType::KeyRelease(Key::ControlLeft) | EventType::KeyRelease(Key::ControlRight) => {
                        ctrl_pressed_clone.store(false, Ordering::Relaxed);
                    }
                    EventType::KeyPress(Key::Alt) | EventType::KeyPress(Key::AltGr) => {
                        alt_pressed_clone.store(true, Ordering::Relaxed);
                    }
                    EventType::KeyRelease(Key::Alt) | EventType::KeyRelease(Key::AltGr) => {
                        alt_pressed_clone.store(false, Ordering::Relaxed);
                    }
                    EventType::KeyPress(Key::KeyQ) => {
                        if ctrl_pressed_clone.load(Ordering::Relaxed) && alt_pressed_clone.load(Ordering::Relaxed) {
                            println!("Exit shortcut detected (Ctrl+Alt+Q) - stopping capture");
                            let _ = tx_clone.send(CaptureControl::ExitRequested);
                            should_stop_clone.store(true, Ordering::Relaxed);
                            return Some(event); // Pass through the Q key
                        }
                    }
                    _ => {}
                }
                
                // Convert event to our format and decide whether to block
                let (input_event, should_block) = match event.event_type {
                    EventType::MouseMove { x, y } => {
                        let mut last_pos = last_mouse_pos_clone.lock().unwrap();
                        
                        if let Some((prev_x, prev_y)) = *last_pos {
                            // Calculate delta relative to PREVIOUS position
                            let dx = x - prev_x;
                            let dy = y - prev_y;
                            
                            // Only process if there's actual movement
                            if dx != 0.0 || dy != 0.0 {
                                // Reset cursor to center to prevent hitting screen edges
                                #[cfg(windows)]
                                unsafe {
                                    SetCursorPos(CENTER_X, CENTER_Y);
                                }
                                
                                // Update last_pos to CENTER (where we just moved the cursor)
                                // The next event will be relative to this center
                                *last_pos = Some((CENTER_X as f64, CENTER_Y as f64));
                                
                                (Some(InputEventData {
                                    event_type: "mousemove".to_string(),
                                    key: None,
                                    key_code: None,
                                    x: None,
                                    y: None,
                                    dx: Some(dx),
                                    dy: Some(dy),
                                }), true) // BLOCK mouse move (keep cursor centered)
                            } else {
                                (None, true) // Block even if no movement (keep centered)
                            }
                        } else {
                            // First event, initialize to current pos
                            *last_pos = Some((x, y));
                            (None, true)
                        }
                    }
                    EventType::KeyPress(key) => {
                        let key_str = format!("{:?}", key);
                        (Some(InputEventData {
                            event_type: "keydown".to_string(),
                            key: Some(key_str),
                            key_code: Some(rdev_key_to_code(key)),
                            x: None,
                            y: None,
                            dx: None,
                            dy: None,
                        }), true) // Block keyboard events
                    }
                    EventType::KeyRelease(key) => {
                        let key_str = format!("{:?}", key);
                        (Some(InputEventData {
                            event_type: "keyup".to_string(),
                            key: Some(key_str),
                            key_code: Some(rdev_key_to_code(key)),
                            x: None,
                            y: None,
                            dx: None,
                            dy: None,
                        }), true) // Block keyboard events
                    }
                    EventType::ButtonPress(button) => {
                        let button_name = match button {
                            rdev::Button::Left => "button0",
                            rdev::Button::Right => "button1",
                            rdev::Button::Middle => "button2",
                            _ => "button0",
                        };
                        
                        (Some(InputEventData {
                            event_type: "mousedown".to_string(),
                            key: Some(button_name.to_string()),
                            key_code: None,
                            x: None,
                            y: None,
                            dx: None,
                            dy: None,
                        }), true) // Block mouse clicks
                    }
                    EventType::ButtonRelease(button) => {
                        let button_name = match button {
                            rdev::Button::Left => "button0",
                            rdev::Button::Right => "button1",
                            rdev::Button::Middle => "button2",
                            _ => "button0",
                        };
                        
                        (Some(InputEventData {
                            event_type: "mouseup".to_string(),
                            key: Some(button_name.to_string()),
                            key_code: None,
                            x: None,
                            y: None,
                            dx: None,
                            dy: None,
                        }), true) // Block mouse clicks
                    }
                    EventType::Wheel { delta_x, delta_y } => {
                        (Some(InputEventData {
                            event_type: "wheel".to_string(),
                            key: None,
                            key_code: None,
                            x: None,
                            y: None,
                            dx: Some(delta_x as f64),
                            dy: Some(delta_y as f64),
                        }), true) // Block wheel events
                    }
                };

                if let Some(evt) = input_event {
                    if let Err(e) = tx_clone.send(CaptureControl::InputEvent(evt)) {
                        eprintln!("[Capture] 发送事件失败: {:?}", e);
                    }
                }
                
                // Block or pass through based on event type
                if should_block {
                    None // Block the event
                } else {
                    Some(event) // Pass through
                }
            };

            println!("\n========================================");
            println!("Starting global input capture (Virtual Mouse Trap mode)...");
            println!("Press Ctrl+Alt+Q to exit capture mode");
            println!("========================================\n");
            
            match grab(callback) {
                Ok(_) => {
                    println!("Input capture ended normally");
                }
                Err(error) => {
                    eprintln!("❌ Input capture error: {:?}", error);
                    eprintln!("提示: 请确保程序以管理员身份运行！");
                }
            }
        });
    }

    pub fn stop_capture(&self) {
        self.should_stop.store(true, Ordering::Relaxed);
        println!("Input capture stop requested");
    }
}

// Helper function to map rdev Key to u32 code
fn rdev_key_to_code(key: Key) -> u32 {
    match key {
        // Letters
        Key::KeyA => 65, Key::KeyB => 66, Key::KeyC => 67, Key::KeyD => 68,
        Key::KeyE => 69, Key::KeyF => 70, Key::KeyG => 71, Key::KeyH => 72,
        Key::KeyI => 73, Key::KeyJ => 74, Key::KeyK => 75, Key::KeyL => 76,
        Key::KeyM => 77, Key::KeyN => 78, Key::KeyO => 79, Key::KeyP => 80,
        Key::KeyQ => 81, Key::KeyR => 82, Key::KeyS => 83, Key::KeyT => 84,
        Key::KeyU => 85, Key::KeyV => 86, Key::KeyW => 87, Key::KeyX => 88,
        Key::KeyY => 89, Key::KeyZ => 90,

        // Numbers
        Key::Num0 => 48, Key::Num1 => 49, Key::Num2 => 50, Key::Num3 => 51,
        Key::Num4 => 52, Key::Num5 => 53, Key::Num6 => 54, Key::Num7 => 55,
        Key::Num8 => 56, Key::Num9 => 57,

        // Special Keys
        Key::Return => 13,
        Key::Escape => 27,
        Key::Space => 32,
        Key::Backspace => 8,
        Key::Tab => 9,
        
        // Punctuation
        Key::Minus => 45,
        Key::Equal => 61,
        Key::LeftBracket => 91,
        Key::RightBracket => 93,
        Key::BackSlash => 92,
        Key::SemiColon => 59,
        Key::Quote => 39,
        Key::Comma => 44,
        Key::Dot => 46,
        Key::Slash => 47,
        Key::BackQuote => 96,

        // Function Keys (Mapped to custom range or standard VK codes if needed)
        // For now, we map them to 0 or specific codes if the simulator supports them
        // Adding F1-F12 support would require updating simulator as well
        
        // Modifiers
        Key::ShiftLeft => 160,
        Key::ShiftRight => 161,
        Key::ControlLeft => 162,
        Key::ControlRight => 163,
        Key::Alt => 164,
        Key::AltGr => 165,
        Key::MetaLeft => 91,
        Key::MetaRight => 92,

        _ => 0,
    }
}
