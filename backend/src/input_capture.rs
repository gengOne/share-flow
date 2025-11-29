use rdev::{grab, Event, EventType, Key};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct InputEventData {
    pub event_type: String,
    pub key: Option<String>,
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
        
        // Spawn blocking thread for rdev grab
        std::thread::spawn(move || {
            let ctrl_pressed_clone = Arc::clone(&ctrl_pressed);
            let alt_pressed_clone = Arc::clone(&alt_pressed);
            let tx_clone = tx.clone();
            let should_stop_clone = Arc::clone(&should_stop);
            
            let callback = move |event: Event| -> Option<Event> {
                // Check if we should stop
                if should_stop_clone.load(Ordering::Relaxed) {
                    return Some(event); // Pass through all events
                }
                
                // Track modifier keys
                match &event.event_type {
                    EventType::KeyPress(Key::ControlLeft) | EventType::KeyPress(Key::ControlRight) => {
                        ctrl_pressed_clone.store(true, Ordering::Relaxed);
                        return Some(event); // Pass through
                    }
                    EventType::KeyRelease(Key::ControlLeft) | EventType::KeyRelease(Key::ControlRight) => {
                        ctrl_pressed_clone.store(false, Ordering::Relaxed);
                        return Some(event); // Pass through
                    }
                    EventType::KeyPress(Key::Alt) | EventType::KeyPress(Key::AltGr) => {
                        alt_pressed_clone.store(true, Ordering::Relaxed);
                        return Some(event); // Pass through
                    }
                    EventType::KeyRelease(Key::Alt) | EventType::KeyRelease(Key::AltGr) => {
                        alt_pressed_clone.store(false, Ordering::Relaxed);
                        return Some(event); // Pass through
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
                
                // Convert event to our format
                let input_event = match event.event_type {
                    EventType::MouseMove { x, y } => {
                        Some(InputEventData {
                            event_type: "mousemove".to_string(),
                            key: None,
                            x: Some(x),
                            y: Some(y),
                            dx: None,
                            dy: None,
                        })
                    }
                    EventType::KeyPress(key) => {
                        Some(InputEventData {
                            event_type: "keydown".to_string(),
                            key: Some(format!("{:?}", key)),
                            x: None,
                            y: None,
                            dx: None,
                            dy: None,
                        })
                    }
                    EventType::KeyRelease(key) => {
                        Some(InputEventData {
                            event_type: "keyup".to_string(),
                            key: Some(format!("{:?}", key)),
                            x: None,
                            y: None,
                            dx: None,
                            dy: None,
                        })
                    }
                    EventType::ButtonPress(button) => {
                        let button_name = match button {
                            rdev::Button::Left => "button0",
                            rdev::Button::Right => "button2",
                            rdev::Button::Middle => "button1",
                            _ => "button0",
                        };
                        Some(InputEventData {
                            event_type: "mousedown".to_string(),
                            key: Some(button_name.to_string()),
                            x: None,
                            y: None,
                            dx: None,
                            dy: None,
                        })
                    }
                    EventType::ButtonRelease(button) => {
                        let button_name = match button {
                            rdev::Button::Left => "button0",
                            rdev::Button::Right => "button2",
                            rdev::Button::Middle => "button1",
                            _ => "button0",
                        };
                        Some(InputEventData {
                            event_type: "mouseup".to_string(),
                            key: Some(button_name.to_string()),
                            x: None,
                            y: None,
                            dx: None,
                            dy: None,
                        })
                    }
                    EventType::Wheel { delta_x, delta_y } => {
                        Some(InputEventData {
                            event_type: "wheel".to_string(),
                            key: None,
                            x: None,
                            y: None,
                            dx: Some(delta_x as f64),
                            dy: Some(delta_y as f64),
                        })
                    }
                };

                if let Some(evt) = input_event {
                    let _ = tx_clone.send(CaptureControl::InputEvent(evt));
                }
                
                // Block the event from propagating (return None)
                None
            };

            println!("Starting global input capture (blocking mode)...");
            println!("Press Ctrl+Alt+Q to exit capture mode");
            if let Err(error) = grab(callback) {
                eprintln!("Input capture error: {:?}", error);
            }
        });
    }

    pub fn stop_capture(&self) {
        self.should_stop.store(true, Ordering::Relaxed);
        println!("Input capture stop requested");
    }
}
