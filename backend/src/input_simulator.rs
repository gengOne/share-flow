use rdev::{simulate, EventType, Key};

#[cfg(not(windows))]
use rdev::Button;

pub struct InputSimulator;

impl InputSimulator {
    pub fn new() -> Self {
        Self
    }

    pub fn mouse_move(&self, dx: i32, dy: i32) {
        // 只在有实际移动时打印日志
        if dx != 0 || dy != 0 {
            println!("[InputSimulator] 鼠标移动: dx={}, dy={}", dx, dy);
        }
        
        // 使用 Windows API 进行鼠标移动
        #[cfg(windows)]
        {
            use std::mem;
            
            #[repr(C)]
            struct INPUT {
                type_: u32,
                union_: INPUT_UNION,
            }
            
            #[repr(C)]
            #[derive(Copy, Clone)]
            union INPUT_UNION {
                mi: MOUSEINPUT,
            }
            
            #[repr(C)]
            #[derive(Copy, Clone)]
            struct MOUSEINPUT {
                dx: i32,
                dy: i32,
                mouse_data: u32,
                dw_flags: u32,
                time: u32,
                dw_extra_info: usize,
            }
            
            const INPUT_MOUSE: u32 = 0;
            const MOUSEEVENTF_MOVE: u32 = 0x0001;
            
            extern "system" {
                fn SendInput(n_inputs: u32, p_inputs: *const INPUT, cb_size: i32) -> u32;
            }
            
            unsafe {
                // 使用 SendInput 进行相对移动（更高效）
                let input = INPUT {
                    type_: INPUT_MOUSE,
                    union_: INPUT_UNION {
                        mi: MOUSEINPUT {
                            dx,
                            dy,
                            mouse_data: 0,
                            dw_flags: MOUSEEVENTF_MOVE,
                            time: 0,
                            dw_extra_info: 0,
                        },
                    },
                };
                
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
        println!("[InputSimulator] 鼠标点击: button={}, down={}", button, is_down);
        
        #[cfg(windows)]
        {
            use std::mem;
            
            #[repr(C)]
            struct INPUT {
                type_: u32,
                union_: INPUT_UNION,
            }
            
            #[repr(C)]
            #[derive(Copy, Clone)]
            union INPUT_UNION {
                mi: MOUSEINPUT,
            }
            
            #[repr(C)]
            #[derive(Copy, Clone)]
            struct MOUSEINPUT {
                dx: i32,
                dy: i32,
                mouse_data: u32,
                dw_flags: u32,
                time: u32,
                dw_extra_info: usize,
            }
            
            const INPUT_MOUSE: u32 = 0;
            const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
            const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
            const MOUSEEVENTF_RIGHTDOWN: u32 = 0x0008;
            const MOUSEEVENTF_RIGHTUP: u32 = 0x0010;
            const MOUSEEVENTF_MIDDLEDOWN: u32 = 0x0020;
            const MOUSEEVENTF_MIDDLEUP: u32 = 0x0040;
            
            extern "system" {
                fn SendInput(n_inputs: u32, p_inputs: *const INPUT, cb_size: i32) -> u32;
            }
            
            let dw_flags = match (button, is_down) {
                (0, true) => MOUSEEVENTF_LEFTDOWN,
                (0, false) => MOUSEEVENTF_LEFTUP,
                (1, true) => MOUSEEVENTF_RIGHTDOWN,
                (1, false) => MOUSEEVENTF_RIGHTUP,
                (2, true) => MOUSEEVENTF_MIDDLEDOWN,
                (2, false) => MOUSEEVENTF_MIDDLEUP,
                _ => MOUSEEVENTF_LEFTDOWN,
            };
            
            unsafe {
                let input = INPUT {
                    type_: INPUT_MOUSE,
                    union_: INPUT_UNION {
                        mi: MOUSEINPUT {
                            dx: 0,
                            dy: 0,
                            mouse_data: 0,
                            dw_flags,
                            time: 0,
                            dw_extra_info: 0,
                        },
                    },
                };
                
                let result = SendInput(1, &input, mem::size_of::<INPUT>() as i32);
                if result == 1 {
                    println!("  ✓ 鼠标点击成功");
                } else {
                    eprintln!("  ✗ 鼠标点击失败");
                }
            }
        }
        
        #[cfg(not(windows))]
        {
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
            
            if let Err(e) = simulate(&event_type) {
                eprintln!("模拟鼠标点击失败: {:?}", e);
            } else {
                println!("  ✓ 鼠标点击成功");
            }
        }
    }

    pub fn key_press(&self, key_code: u32, is_down: bool) {
        // 将字符码转换为 rdev Key
        let key = self.map_key_code(key_code);
        
        if let Some(rdev_key) = key {
            let event_type = if is_down {
                EventType::KeyPress(rdev_key)
            } else {
                EventType::KeyRelease(rdev_key)
            };

            println!("[InputSimulator] 按键: code={} (char: '{}'), down={}", 
                     key_code, 
                     char::from_u32(key_code).unwrap_or('?'), 
                     is_down);
            
            if let Err(e) = simulate(&event_type) {
                eprintln!("模拟按键失败: {:?}", e);
            }
        } else {
            println!("[InputSimulator] 未知键码: {} (char: '{}')", 
                     key_code, 
                     char::from_u32(key_code).unwrap_or('?'));
        }
    }

    fn map_key_code(&self, code: u32) -> Option<Key> {
        // 键码映射 - 支持大小写字母
        match code {
            // 字母 A-Z (大写 ASCII 65-90)
            65 => Some(Key::KeyA), 66 => Some(Key::KeyB), 67 => Some(Key::KeyC),
            68 => Some(Key::KeyD), 69 => Some(Key::KeyE), 70 => Some(Key::KeyF),
            71 => Some(Key::KeyG), 72 => Some(Key::KeyH), 73 => Some(Key::KeyI),
            74 => Some(Key::KeyJ), 75 => Some(Key::KeyK), 76 => Some(Key::KeyL),
            77 => Some(Key::KeyM), 78 => Some(Key::KeyN), 79 => Some(Key::KeyO),
            80 => Some(Key::KeyP), 81 => Some(Key::KeyQ), 82 => Some(Key::KeyR),
            83 => Some(Key::KeyS), 84 => Some(Key::KeyT), 85 => Some(Key::KeyU),
            86 => Some(Key::KeyV), 87 => Some(Key::KeyW), 88 => Some(Key::KeyX),
            89 => Some(Key::KeyY), 90 => Some(Key::KeyZ),
            
            // 字母 a-z (小写 ASCII 97-122)
            97 => Some(Key::KeyA), 98 => Some(Key::KeyB), 99 => Some(Key::KeyC),
            100 => Some(Key::KeyD), 101 => Some(Key::KeyE), 102 => Some(Key::KeyF),
            103 => Some(Key::KeyG), 104 => Some(Key::KeyH), 105 => Some(Key::KeyI),
            106 => Some(Key::KeyJ), 107 => Some(Key::KeyK), 108 => Some(Key::KeyL),
            109 => Some(Key::KeyM), 110 => Some(Key::KeyN), 111 => Some(Key::KeyO),
            112 => Some(Key::KeyP), 113 => Some(Key::KeyQ), 114 => Some(Key::KeyR),
            115 => Some(Key::KeyS), 116 => Some(Key::KeyT), 117 => Some(Key::KeyU),
            118 => Some(Key::KeyV), 119 => Some(Key::KeyW), 120 => Some(Key::KeyX),
            121 => Some(Key::KeyY), 122 => Some(Key::KeyZ),
            
            // 数字 0-9
            48 => Some(Key::Num0), 49 => Some(Key::Num1), 50 => Some(Key::Num2),
            51 => Some(Key::Num3), 52 => Some(Key::Num4), 53 => Some(Key::Num5),
            54 => Some(Key::Num6), 55 => Some(Key::Num7), 56 => Some(Key::Num8),
            57 => Some(Key::Num9),
            
            // 特殊键
            13 => Some(Key::Return),
            10 => Some(Key::Return), // 换行符
            27 => Some(Key::Escape),
            32 => Some(Key::Space),
            8 => Some(Key::Backspace),
            9 => Some(Key::Tab),
            
            // 标点符号
            33 => Some(Key::Num1),      // !
            64 => Some(Key::Num2),      // @
            35 => Some(Key::Num3),      // #
            36 => Some(Key::Num4),      // $
            37 => Some(Key::Num5),      // %
            94 => Some(Key::Num6),      // ^
            38 => Some(Key::Num7),      // &
            42 => Some(Key::Num8),      // *
            40 => Some(Key::Num9),      // (
            41 => Some(Key::Num0),      // )
            45 => Some(Key::Minus),     // -
            95 => Some(Key::Minus),     // _
            61 => Some(Key::Equal),     // =
            43 => Some(Key::Equal),     // +
            91 => Some(Key::LeftBracket),   // [
            93 => Some(Key::RightBracket),  // ]
            92 => Some(Key::BackSlash),     // \
            59 => Some(Key::SemiColon),     // ;
            58 => Some(Key::SemiColon),     // :
            39 => Some(Key::Quote),         // '
            34 => Some(Key::Quote),         // "
            44 => Some(Key::Comma),         // ,
            60 => Some(Key::Comma),         // <
            46 => Some(Key::Dot),           // .
            62 => Some(Key::Dot),           // >
            47 => Some(Key::Slash),         // /
            63 => Some(Key::Slash),         // ?
            96 => Some(Key::BackQuote),     // `
            126 => Some(Key::BackQuote),    // ~
            
            _ => None,
        }
    }
}
