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
        
        // Track key press times for long-press detection
        use std::collections::HashMap;
        use std::sync::Mutex;
        use std::time::Instant;
        let key_press_times = Arc::new(Mutex::new(HashMap::<String, Instant>::new()));
        
        // Spawn long-press detection task
        let tx_longpress = tx.clone();
        let key_press_times_clone = Arc::clone(&key_press_times);
        let should_stop_longpress = Arc::clone(&should_stop);
        std::thread::spawn(move || {
            const LONG_PRESS_THRESHOLD: std::time::Duration = std::time::Duration::from_millis(500);
            const CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);
            
            while !should_stop_longpress.load(Ordering::Relaxed) {
                std::thread::sleep(CHECK_INTERVAL);
                
                let mut times = key_press_times_clone.lock().unwrap();
                let now = Instant::now();
                
                // Check for long presses
                let long_pressed: Vec<String> = times.iter()
                    .filter(|(_, &press_time)| now.duration_since(press_time) >= LONG_PRESS_THRESHOLD)
                    .map(|(key, _)| key.clone())
                    .collect();
                
                // Send long-press events and remove from tracking
                for key in long_pressed {
                    times.remove(&key);
                    let _ = tx_longpress.send(CaptureControl::InputEvent(InputEventData {
                        event_type: "longpress".to_string(),
                        key: Some(key),
                        key_code: None,
                        x: None,
                        y: None,
                        dx: None,
                        dy: None,
                    }));
                }
            }
        });
        
        // Spawn blocking thread for rdev grab
        std::thread::spawn(move || {
            let ctrl_pressed_clone = Arc::clone(&ctrl_pressed);
            let alt_pressed_clone = Arc::clone(&alt_pressed);
            let tx_clone = tx.clone();
            let should_stop_clone = Arc::clone(&should_stop);
            
            // Track previous mouse position for delta calculation
            use std::sync::Mutex;
            let last_mouse_pos = Arc::new(Mutex::new(Option::<(f64, f64)>::None));
            let last_mouse_pos_clone = Arc::clone(&last_mouse_pos);
            
            let callback = move |event: Event| -> Option<Event> {
                // Debug: print every event
                // println!("[Capture] 捕获到事件: {:?}", event.event_type);
                
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
                        // Hijack mode: Block mouse movement but capture deltas
                        let mut last_pos = last_mouse_pos_clone.lock().unwrap();
                        
                        // Initialize anchor position if not set
                        if last_pos.is_none() {
                            *last_pos = Some((x, y));
                        }
                        
                        let (anchor_x, anchor_y) = last_pos.unwrap();
                        
                        // Calculate delta relative to the ANCHOR position (since cursor is frozen)
                        let dx = x - anchor_x;
                        let dy = y - anchor_y;
                        
                        // Note: We do NOT update last_pos because we are blocking the event,
                        // so the cursor stays at anchor_x, anchor_y.
                        // The OS calculates the new 'x, y' based on the current cursor position (anchor) + movement.
                        // So 'x - anchor' is the true delta.
                        
                        // Only send if there's actual movement
                        if dx != 0.0 || dy != 0.0 {
                            (Some(InputEventData {
                                event_type: "mousemove".to_string(),
                                key: None,
                                key_code: None,
                                x: None,
                                y: None,
                                dx: Some(dx),
                                dy: Some(dy),
                            }), true) // BLOCK mouse move (Hijack)
                        } else {
                            (None, true) // BLOCK even if no movement
                        }
                    }
                    EventType::KeyPress(key) => {
                        let key_str = format!("{:?}", key);
                        
                        // Track key press time for long-press detection
                        {
                            let mut times = key_press_times.lock().unwrap();
                            times.entry(key_str.clone()).or_insert_with(Instant::now);
                        }
                        
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
                        
                        // Remove from long-press tracking
                        {
                            let mut times = key_press_times.lock().unwrap();
                            times.remove(&key_str);
                        }
                        
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
                        
                        // Track button press time for long-press detection
                        {
                            let mut times = key_press_times.lock().unwrap();
                            times.entry(button_name.to_string()).or_insert_with(Instant::now);
                        }
                        
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
                        
                        // Remove from long-press tracking
                        {
                            let mut times = key_press_times.lock().unwrap();
                            times.remove(button_name);
                        }
                        
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
                    // println!("[Capture] 发送事件到主循环: {:?}", evt.event_type);
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
            println!("Starting global input capture (blocking mode)...");
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
        
        _ => 0,
    }
}
