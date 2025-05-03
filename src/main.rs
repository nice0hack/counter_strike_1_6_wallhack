use crate::game::Game;
use crate::offsets_logger::OffsetsLogger;
use crate::wallhack::WallHack;

mod cs_modules;
mod game;
mod keyboard;
mod offsets_logger;
mod process_detective;
mod vector3;
mod wallhack;

fn main() {
    println!("COUNTER-STRIKE WALLHACK IS RUNNING");

    let mut game = Game::new();

    game.register::<OffsetsLogger>();
    game.register::<WallHack>();

    if let Err(e) = game.run() {
        eprintln!("{}", e);
    }
}
