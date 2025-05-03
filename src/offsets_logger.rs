use std::sync::Arc;

use softbuffer::Buffer;
use winit::window::Window;

use crate::cs_modules::{GameClientArea, OwnPlayer, Players};
use crate::game::{Cheat, GameMemory};
use crate::keyboard::Key;
use crate::keyboard::Key::F8;

#[derive(Default)]
pub struct OffsetsLogger {
    count: u64,
}

impl Cheat for OffsetsLogger {
    fn name(&self) -> &'static str {
        "Offsets Logger"
    }

    fn key(&self) -> &'static Key {
        &F8
    }

    fn update(
        &mut self,
        gm: &mut GameMemory,
        _gca: &mut GameClientArea,
        _buffer: &mut Buffer<Arc<Window>, Arc<Window>>,
    ) {
        self.count += 1;
        println!("===================================");
        println!("Debug game objects {}:", self.count);
        println!("{:?}", gm.get::<OwnPlayer>());
        println!("\n");
        println!("{:?}", gm.get::<Players>());
        println!("\n");
        println!("{:?}", gm.get::<GameClientArea>());
        println!("===================================");
    }
}
