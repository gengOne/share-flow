use rdev::{simulate, Button, EventType, Key};
use std::thread;
use std::time::Duration;

pub struct InputSimulator;

impl InputSimulator {
    pub fn new() -> Self {
        Self
    }

    pub fn mouse_move(&self, dx: i32, dy: i32) {
        println!("[InputSimulator] 鼠标移动: dx={}, dy={}", dx, dy);
        
        // 使用 Windows API 进行鼠标移动
        #[cfg(windows)]
        {
            #[repr(C)]
            struct POINT {
                x: i32,
                y: i32,
            }
            
            extern "system" {
                fn GetCursorPos(point: *mut POINT) -> i32;
                fn SetCursorPos(x: i32, y: i32) -> i32;
            }
            
            unsafe {
                // 使用 SetCursorPos 直接设置鼠标位置
                let mut point = POINT { x: 0, y: 0 };
                if GetCursorPos(&mut point) != 0 {
                    let new_x = point.x + dx;
                    let new_y = point.y + dy;
                    SetCursorPos(new_x, new_y);
                    println!("  移动鼠标从 ({}, {}) 到 ({}, {})", point.x, point.y, new_x, new_y);
                } else {
                    eprintln!("  获取鼠标位置失败");
                }
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
