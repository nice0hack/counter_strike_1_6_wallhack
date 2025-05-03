use std::sync::Arc;

use softbuffer::Buffer;
use winit::window::Window;

use crate::cs_modules::{GameClientArea, OwnPlayer, Players};
use crate::game::{Cheat, GameMemory};
use crate::keyboard::Key;
use crate::keyboard::Key::F5;
use crate::vector3::Vector3;

fn draw_rectangle(
    buffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    sub_x: usize,
    sub_y: usize,
    sub_width: usize,
    sub_height: usize,
    rect_x: usize,
    rect_y: usize,
    rect_width: usize,
    rect_height: usize,
    color: u32,
) {
    let sub_x_end = (sub_x + sub_width).min(buffer_width);
    let sub_y_end = (sub_y + sub_height).min(buffer_height);

    let x_start = (sub_x + rect_x).max(sub_x);
    let y_start = (sub_y + rect_y).max(sub_y);
    let x_end = (x_start + rect_width).min(sub_x_end);
    let y_end = (y_start + rect_height).min(sub_y_end);

    for col in x_start..x_end {
        if rect_y < sub_height {
            let top_index = y_start * buffer_width + col;
            buffer[top_index] = color;
        }
        if rect_y + rect_height - 1 < sub_height {
            let bottom_index = (y_end - 1) * buffer_width + col;
            buffer[bottom_index] = color;
        }
    }

    for row in y_start..y_end {
        if rect_x < sub_width {
            let left_index = row * buffer_width + x_start;
            buffer[left_index] = color;
        }
        if rect_x + rect_width - 1 < sub_width {
            let right_index = row * buffer_width + (x_end - 1);
            buffer[right_index] = color;
        }
    }
}

pub struct WallHack {
    players: Players,
    visible: [u32; 128],
}

impl Default for WallHack {
    fn default() -> Self {
        Self {
            players: Players::default(),
            visible: [0; 128],
        }
    }
}

impl Cheat for WallHack {
    fn name(&self) -> &'static str {
        "WallHack"
    }

    fn key(&self) -> &'static Key {
        &F5
    }

    fn update(
        &mut self,
        gm: &mut GameMemory,
        gca: &mut GameClientArea,
        buffer: &mut Buffer<Arc<Window>, Arc<Window>>,
    ) {
        let own_player = gm.get::<OwnPlayer>();
        let players = gm.get::<Players>();

        for i in 0..players.list.len() {
            let old_player = &mut self.players.list[i];
            let player = &players.list[i];

            if player.visible < 0.0
                || player.visible > 90.0
                || player.state >= 90
                || player.state <= 0
            {
                continue;
            }

            // Every time a player is visible and alive, this number will change
            // Yes, it's a crutch, but working
            if old_player.visible == player.visible {
                if self.visible[i] == 5 {
                    continue;
                } else {
                    self.visible[i] += 1
                }
            } else {
                self.visible[i] = 0;
            }
            old_player.visible = player.visible;

            let rect = own_player.world_to_screen(
                &Vector3::new(gca.width as f32, gca.height as f32, 0.0),
                &player.pos,
            );
            if rect.x < 0.0 || !player.is_alive() {
                continue;
            }

            let color = player.get_player_color();

            let rect_2 = own_player.world_to_screen(
                &Vector3::new(gca.width as f32, gca.height as f32, 0.0),
                &Vector3::new(player.pos.x, player.pos.y, player.pos.z - 50.0),
            );
            let box_height = (rect_2.y - rect.y) * 1.25;

            let rect_up_left_corner_x = rect_2.x - box_height / 4.0;
            let rect_up_left_corner_y = rect.y - box_height * 0.45;
            let rect_down_right_corner_x = rect_2.x + box_height / 3.2;
            let rect_down_right_corner_y = rect.y + box_height * 0.85;

            draw_rectangle(
                buffer.as_mut(),
                1920,
                1080,
                gca.pos_x as usize,
                gca.pos_y as usize,
                gca.width as usize,
                gca.height as usize,
                rect_up_left_corner_x as usize,
                rect_up_left_corner_y as usize,
                // Distance from our player to target player affects the size
                (rect_down_right_corner_x - rect_up_left_corner_x) as usize,
                (rect_down_right_corner_y - rect_up_left_corner_y) as usize,
                color,
            );
        }
    }
}
