use rdev::{simulate, Button, EventType, Key, SimulateError};
use std::thread;
use std::time::Duration;

pub struct InputSimulator;

impl InputSimulator {
    pub fn new() -> Self {
        Self
    }

    pub fn mouse_move(&self, dx: i32, dy: i32) {
        // rdev 使用绝对坐标，需要获取当前位置并计算新位置
        // 但为了简单起见，我们直接使用相对移动
        // 注意：这可能需要根据实际情况调整
        
        // 暂时使用简单的实现
        println!("[InputSimulator] 鼠标移动: dx={}, dy={}", dx, dy);
        
        // TODO: 实现实际的鼠标移动
        // rdev 的 simulate 需要绝对坐标，这里需要更复杂的逻辑
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
        // ASCII 字符
        if code >= 65 && code <= 90 {
            // A-Z
            let ch = (code as u8) as char;
            return Some(Key::KeyA); // 简化处理，实际需要完整映射
        }
        
        // 特殊键
        match code {
            13 => Some(Key::Return),
            27 => Some(Key::Escape),
            32 => Some(Key::Space),
            8 => Some(Key::Backspace),
            9 => Some(Key::Tab),
            _ => None,
        }
    }
}
