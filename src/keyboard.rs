#![allow(non_snake_case)]

use std::fmt;

extern "system" {
    fn GetKeyState(n_virt_key: i32) -> i16;
}

#[allow(dead_code)]
pub enum Key {
    CapsLock,
    FiveNumKey,
    F3,
    F5,
    F8,
}

impl Key {
    pub fn enabled(&self) -> bool {
        let key_num = match self {
            Key::CapsLock => 0x14,
            Key::FiveNumKey => 0x35,
            Key::F3 => 0x72,
            Key::F5 => 0x74,
            Key::F8 => 0x77,
        };

        unsafe { GetKeyState(key_num) & 1 == 1 }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Key::CapsLock => write!(f, "Caps Lock"),
            Key::FiveNumKey => write!(f, "5 num key"),
            Key::F3 => write!(f, "F3"),
            Key::F5 => write!(f, "F5"),
            Key::F8 => write!(f, "F8"),
        }
    }
}
