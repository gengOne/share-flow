use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    /// Broadcast message to find other peers
    Discovery {
        id: String,
        name: String,
        port: u16,
    },
    /// Mouse movement delta
    MouseMove {
        x: i32,
        y: i32,
    },
    /// Mouse button state change
    MouseClick {
        button: u8, // 0: Left, 1: Right, 2: Middle, etc.
        state: bool, // true: Down, false: Up
    },
    /// Keyboard key state change
    KeyPress {
        key: u32, // Virtual key code
        state: bool, // true: Down, false: Up
    },
    /// Request to establish a control connection
    ConnectRequest,
    /// Response to connection request
    ConnectResponse {
        success: bool,
    },
    /// Notify peer that we are disconnecting
    Disconnect,
}
