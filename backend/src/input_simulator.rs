use rdev::{simulate, Button, EventType, Key, SimulateError};
use std::thread;
use std::time::Duration;

pub struct InputSimulator;

impl InputSimulator {
    pub fn new() -> Self {
        Self
    }

    pub fn mouse_move(&self, dx: i32, dy: i32) {
        println!("[InputSimulator] 鼠标移动: dx={}, dy={}", dx, dy);
        
        // 使用 Windows API 进行相对鼠标移动
        #[cfg(windows)]
        {
            use std::ffi::c_void;
            use std::mem;
            
            #[repr(C)]
            struct INPUT {
                input_type: u32,
                data: [u8; 24], // 足够大的空间
            }
            
            #[repr(C)]
            struct MOUSEINPUT {
                dx: i32,
                dy: i32,
                mouse_data: u32,
                flags: u32,
                time: u32,
                extra_info: *mut c_void,
            }
            
            const INPUT_MOUSE: u32 = 0;
            const MOUSEEVENTF_MOVE: u32 = 0x0001;
            
            extern "system" {
                fn SendInput(inputs: u32, input: *const INPUT, size: i32) -> u32;
            }
            
            let mouse_input = MOUSEINPUT {
                dx,
                dy,
                mouse_data: 0,
                flags: MOUSEEVENTF_MOVE,
                time: 0,
                extra_info: std::ptr::null_mut(),
            };
            
            let mut input = INPUT {
                input_type: INPUT_MOUSE,
                data: [0; 24],
            };
            
            unsafe {
                std::ptr::copy_nonoverlapping(
                    &mouse_input as *const _ as *const u8,
                    input.data.as_mut_ptr(),
                    mem::size_of::<MOUSEINPUT>(),
                );
                
                SendInput(1, &input, mem::size_of::<INPUT>() as i32);
            }
        }
        
        #[cfg(not(windows))]
        {
            // 非 Windows 系统使用 rdev（需要实现绝对坐标转换）
            eprintln!("鼠标移动暂不支持此平台");
        }
    }

    pub fn mouse_click(&self, button: u8, is_down: bool) {
        let rdev_button = match button {
            0 => Button::Left,
            1 => Button::Right,
            2 => Button::Middle,
            _ => Button::Left,
        };

        let event_type = if is_down {
            EventType::ButtonPress(rdev_button)
        } else {
            EventType::ButtonRelease(rdev_button)
        };

        println!("[InputSimulator] 鼠标点击: button={}, down={}", button, is_down);
        
        if let Err(e) = simulate(&event_type) {
            eprintln!("模拟鼠标点击失败: {:?}", e);
        }
        
        // 添加小延迟以确保事件被处理
        thread::sleep(Duration::from_millis(10));
    }

    pub fn key_press(&self, key_code: u32, is_down: bool) {
        // 将虚拟键码转换为 rdev Key
        // 这是一个简化的映射，实际需要完整的键码映射表
        let key = self.map_key_code(key_code);
        
        if let Some(rdev_key) = key {
            let event_type = if is_down {
                EventType::KeyPress(rdev_key)
            } else {
                EventType::KeyRelease(rdev_key)
            };

            println!("[InputSimulator] 按键: code={}, down={}", key_code, is_down);
            
            if let Err(e) = simulate(&event_type) {
                eprintln!("模拟按键失败: {:?}", e);
            }
            
            thread::sleep(Duration::from_millis(10));
        } else {
            println!("[InputSimulator] 未知键码: {}", key_code);
        }
    }

    fn map_key_code(&self, code: u32) -> Option<Key> {
        // 简化的键码映射
        match code {
            // 字母 A-Z (ASCII)
            65 => Some(Key::KeyA), 66 => Some(Key::KeyB), 67 => Some(Key::KeyC),
            68 => Some(Key::KeyD), 69 => Some(Key::KeyE), 70 => Some(Key::KeyF),
            71 => Some(Key::KeyG), 72 => Some(Key::KeyH), 73 => Some(Key::KeyI),
            74 => Some(Key::KeyJ), 75 => Some(Key::KeyK), 76 => Some(Key::KeyL),
            77 => Some(Key::KeyM), 78 => Some(Key::KeyN), 79 => Some(Key::KeyO),
            80 => Some(Key::KeyP), 81 => Some(Key::KeyQ), 82 => Some(Key::KeyR),
            83 => Some(Key::KeyS), 84 => Some(Key::KeyT), 85 => Some(Key::KeyU),
            86 => Some(Key::KeyV), 87 => Some(Key::KeyW), 88 => Some(Key::KeyX),
            89 => Some(Key::KeyY), 90 => Some(Key::KeyZ),
            
            // 数字 0-9
            48 => Some(Key::Num0), 49 => Some(Key::Num1), 50 => Some(Key::Num2),
            51 => Some(Key::Num3), 52 => Some(Key::Num4), 53 => Some(Key::Num5),
            54 => Some(Key::Num6), 55 => Some(Key::Num7), 56 => Some(Key::Num8),
            57 => Some(Key::Num9),
            
            // 特殊键
            13 => Some(Key::Return),
            27 => Some(Key::Escape),
            32 => Some(Key::Space),
            8 => Some(Key::Backspace),
            9 => Some(Key::Tab),
            16 => Some(Key::ShiftLeft),
            17 => Some(Key::ControlLeft),
            18 => Some(Key::Alt),
            
            _ => None,
        }
    }
}
