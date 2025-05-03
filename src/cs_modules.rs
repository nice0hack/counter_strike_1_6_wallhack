#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use crate::process_detective::{GameModule, GameObject, ProcessDetective};
use crate::vector3::Vector3;
use std::fmt;
use std::os::raw::c_long;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct PCSTR(pub *const u8);

impl PCSTR {
    pub fn null() -> Self {
        PCSTR(std::ptr::null())
    }
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone)]
struct RECT {
    pub left: c_long,
    pub top: c_long,
    pub right: c_long,
    pub bottom: c_long,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct POINT {
    pub x: c_long,
    pub y: c_long,
}

extern "system" {
    fn FindWindowA(lpClassName: PCSTR, lpWindowName: PCSTR) -> isize;
    fn GetClientRect(hWnd: isize, lpRect: *mut RECT) -> i32;
    fn ClientToScreen(hWnd: isize, lpPoint: *mut POINT) -> i32;
}

#[derive(Clone, Debug)]
pub enum CsModuleName {
    Hw,
}

#[derive(Clone, Debug)]
pub struct CsModule {
    pub name: CsModuleName,
}

impl GameModule for CsModule {
    fn name(&self) -> &'static str {
        match self.name.clone() {
            CsModuleName::Hw => "hw.dll",
        }
    }
}

pub struct OwnPlayer {
    hw_gm: CsModule,
    pub health: i32,
    pub pos: Vector3,
    pub matrix: [f32; 16],
}

impl OwnPlayer {
    pub fn world_to_screen(&self, screen_size: &Vector3, world_pos: &Vector3) -> Vector3 {
        // Matrix-vector Product, multiplying world(eye) coordinates by projection matrix = clipCoords
        let mut clip_coords = Vector3::default();
        clip_coords.x = world_pos.x * self.matrix[0]
            + world_pos.y * self.matrix[4]
            + world_pos.z * self.matrix[8]
            + self.matrix[12];
        clip_coords.y = world_pos.x * self.matrix[1]
            + world_pos.y * self.matrix[5]
            + world_pos.z * self.matrix[9]
            + self.matrix[13];
        clip_coords.z = world_pos.x * self.matrix[2]
            + world_pos.y * self.matrix[6]
            + world_pos.z * self.matrix[10]
            + self.matrix[14];
        let w = world_pos.x * self.matrix[3]
            + world_pos.y * self.matrix[7]
            + world_pos.z * self.matrix[11]
            + self.matrix[15];

        if w < 0.1f32 {
            return Vector3::new(-1.0, -1.0, -1.0);
        }

        // Perspective division, dividing by clip.W = Normalized Device Coordinates
        let mut ndc = Vector3::default();
        ndc.x = clip_coords.x / w;
        ndc.y = clip_coords.y / w;
        ndc.z = clip_coords.z / w;

        // Transform to window coordinates
        let mut screen = Vector3::default();
        screen.x = (screen_size.x / 2f32 * ndc.x) + (ndc.x + screen_size.x / 2f32);
        screen.y = -(screen_size.y / 2f32 * ndc.y) + (ndc.y + screen_size.y / 2f32);
        screen.z = (*world_pos - self.pos).length();
        return screen;
    }
}

impl GameObject for OwnPlayer {
    fn new() -> Self {
        Self {
            hw_gm: CsModule {
                name: CsModuleName::Hw,
            },
            health: 0,
            pos: Vector3::default(),
            matrix: [0f32; 16],
        }
    }

    fn read(&mut self, pd: &mut ProcessDetective) {
        self.health = pd.read_mem(&self.hw_gm, 0x145E790).unwrap_or_default();
        self.pos = pd.read_mem(&self.hw_gm, 0x0145E704).unwrap_or_default();
        if let Some(matrix) = pd.read_mem(&self.hw_gm, 0xdc6260) {
            self.matrix = matrix;
        }
    }
}

impl fmt::Debug for OwnPlayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OwnPlayer {{\n    health: {},\n    position: {:?},\n    matrix: {:?}\n}}",
            self.health, self.pos, self.matrix
        )
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Player {
    unknown_1: [u8; 0x130], // 0x0
    model_name: [u8; 0x8],  // 0x130
    unknown_2: [u8; 0x44],  // 0x138
    pub state: i32,         // 0x17c
    pub visible: f32,       // 0x180
    unknown_3: [u8; 0x4],   // 0x184
    pub pos: Vector3,       // 0x188,
    unknown_4: [u8; 0xbc],  // 0x194
}

impl Player {
    pub fn new() -> Self {
        Self {
            unknown_1: [0; 0x130],
            model_name: [0; 0x8],
            unknown_2: [0; 0x44],
            state: 0,
            visible: 0f32,
            unknown_3: [0; 0x4],
            pos: Vector3::default(),
            unknown_4: [0; 0xbc],
        }
    }

    pub fn get_model_name(&self) -> String {
        let model_name_bytes = self
            .model_name
            .iter()
            .take_while(|&c| *c != 0)
            .copied()
            .collect::<Vec<u8>>();
        String::from_utf8(model_name_bytes).unwrap_or_default()
    }

    pub fn get_player_color(&self) -> u32 {
        match self.get_model_name().as_str() {
            // Add new to correctly identify the player's team
            "terror" | "arctic" | "leet" | "guerilla" | "t_mod" => 0xffff0000,
            "gign" | "sas" | "gsg9" | "urban" | "ct_mod" => 0xff0000ff,
            _ => 0xffff00ff, // Unknown team
        }
    }

    pub fn is_alive(&self) -> bool {
        !(self.state == 0 || self.pos.x == 0.0 || self.pos.y == 0.0 || self.pos.z == 0.0)
    }
}

impl fmt::Debug for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Player {{\n    state: {},\n    visible: {},\n    position: {:?},\n    model_name: {:?}\n}}",
               self.state, self.visible, self.pos, self.get_model_name())
    }
}

#[derive(Debug)]
pub struct Players {
    hw_gm: CsModule,
    pub list: [Player; 128],
}

impl Default for Players {
    fn default() -> Self {
        Self {
            hw_gm: CsModule {
                name: CsModuleName::Hw,
            },
            list: [Player::new(); 128],
        }
    }
}

impl GameObject for Players {
    fn new() -> Self {
        Self::default()
    }

    fn read(&mut self, pd: &mut ProcessDetective) {
        self.list = pd
            .read_mem(&self.hw_gm, 0x013fda98)
            .unwrap_or([Player::new(); 128]);
    }
}

pub struct GameClientArea {
    pub pos_x: u32,
    pub pos_y: u32,
    pub width: u32,
    pub height: u32,
}

impl GameObject for GameClientArea {
    fn new() -> Self {
        Self {
            pos_x: 0,
            pos_y: 0,
            width: 640,
            height: 480,
        }
    }

    fn read(&mut self, _pd: &mut ProcessDetective) {
        const WINDOW_NAME: &[u8] = b"Counter-Strike\0";
        let h_wnd = unsafe { FindWindowA(PCSTR::null(), PCSTR(WINDOW_NAME.as_ptr())) };

        if h_wnd != 0 {
            let mut client_rect = RECT::default();
            unsafe {
                GetClientRect(h_wnd, &mut client_rect);
            }

            let mut top_left = POINT {
                x: client_rect.left,
                y: client_rect.top,
            };
            let mut bottom_right = POINT {
                x: client_rect.right,
                y: client_rect.bottom,
            };

            unsafe {
                ClientToScreen(h_wnd, &mut top_left);
                ClientToScreen(h_wnd, &mut bottom_right);
            }

            self.pos_x = top_left.x as u32;
            self.pos_y = top_left.y as u32;
            self.width = (bottom_right.x - top_left.x) as u32;
            self.height = (bottom_right.y - top_left.y) as u32;
        }
    }
}

impl fmt::Debug for GameClientArea {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GameClientArea {{\n    pos_x: {},\n    pos_y: {},\n    width: {},\n    height: {}\n}}",
            self.pos_x, self.pos_y, self.width, self.height
        )
    }
}
