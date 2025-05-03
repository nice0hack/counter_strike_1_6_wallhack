#![allow(non_snake_case)]

use std::ffi::c_void;
use std::num::NonZeroU32;
use std::sync::Arc;

use softbuffer::{Buffer, Context, Surface};
use winit::application::ApplicationHandler;
use winit::event_loop::ActiveEventLoop;
use winit::platform::windows::{WindowAttributesExtWindows, HWND};
use winit::raw_window_handle_05::{HasRawWindowHandle, RawWindowHandle};
use winit::window::{Fullscreen, Window, WindowId, WindowLevel};
use winit::{
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
};

use crate::cs_modules::GameClientArea;
use crate::keyboard::Key;
use crate::process_detective::{GameObject, ProcessDetective};

const DARK_GRAY_CLEAR_COLOR: u32 = 0xff181818;

extern "system" {
    fn GetWindowLongPtrW(hWnd: *mut c_void, nIndex: i32) -> isize;

    fn SetWindowLongPtrW(hWnd: *mut c_void, nIndex: i32, dwNewLong: isize) -> isize;

    fn SetLayeredWindowAttributes(hWnd: *mut c_void, crKey: u32, bAlpha: u8, dwFlags: u32) -> u32;

    fn SetWindowPos(
        hWnd: *mut c_void,
        hWndInsertAfter: HWND,
        x: u32,
        y: u32,
        cx: u32,
        cy: u32,
        uFlags: u32,
    ) -> i32;
}

fn create_window(
    event_loop: &ActiveEventLoop,
) -> Result<
    (
        Arc<Window>,
        Context<Arc<Window>>,
        Surface<Arc<Window>, Arc<Window>>,
    ),
    String,
> {
    let window = event_loop
        .create_window(
            Window::default_attributes()
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_transparent(true)
                .with_decorations(false)
                .with_resizable(false)
                .with_fullscreen(Some(Fullscreen::Borderless(None)))
                .with_drag_and_drop(false),
        )
        .map_err(|e| format!("Create window error: {}", e))?;

    let window_size = window.inner_size();

    // Make super window
    if let RawWindowHandle::Win32(handle) = window.raw_window_handle() {
        unsafe {
            // GWL_EXSTYLE: -20,
            // WS_EX_LAYERED: 0x00080000, WS_EX_TRANSPARENT: 0x00000020
            // WS_EX_TOPMOST 0x00000008, WS_EX_NOACTIVATE: 0x08000000
            let ex_style = GetWindowLongPtrW(handle.hwnd, -20);
            SetWindowLongPtrW(
                handle.hwnd,
                -20,
                ex_style | 0x00080000 | 0x00000020 | 0x00000008 | 0x08000000,
            );

            // LWA_COLORKEY: 0x00000001, LWA_ALPHA: 0x00000002
            SetLayeredWindowAttributes(handle.hwnd, DARK_GRAY_CLEAR_COLOR, 0x0, 0x00000001);

            // HWND_TOPMOST: -1, SWP_NOMOVE: 0x0002, SWP_NOSIZE: 0x0001
            SetWindowPos(handle.hwnd, -1, 0, 0, 0, 0, 0x0002 | 0x0001);
        }
    }

    let window = Arc::new(window);

    let context =
        Context::new(window.clone()).map_err(|e| format!("Create draw context error: {}", e))?;

    let mut surface = Surface::new(&context, window.clone())
        .map_err(|e| format!("Create surface for draw error: {}", e))?;

    let (Some(width), Some(height)) = (
        NonZeroU32::new(window_size.width),
        NonZeroU32::new(window_size.height),
    ) else {
        return Err("Get NonZeroU32 width and height failed".to_string());
    };
    surface
        .resize(width, height)
        .map_err(|e| format!("Set surface size failed: {}", e))?;

    Ok((window.clone(), context, surface))
}

#[derive(Clone)]
pub struct GameMemory {
    pd: ProcessDetective,
}

impl GameMemory {
    pub fn new() -> Self {
        Self {
            pd: ProcessDetective::new(),
        }
    }

    pub fn get<T: GameObject>(&mut self) -> T {
        let mut game_object = T::new();
        game_object.read(&mut self.pd);
        game_object
    }
}

pub struct GameInfo {
    pub gm: GameMemory,
    pub window: Arc<Window>,
    pub surface: Surface<Arc<Window>, Arc<Window>>,
}

impl GameInfo {
    pub fn new(window: Arc<Window>, surface: Surface<Arc<Window>, Arc<Window>>) -> Self {
        Self {
            gm: GameMemory::new(),
            window,
            surface,
        }
    }
}

pub trait Cheat {
    fn name(&self) -> &'static str;
    fn key(&self) -> &'static Key;
    fn update(
        &mut self,
        gm: &mut GameMemory,
        gca: &mut GameClientArea,
        buffer: &mut Buffer<Arc<Window>, Arc<Window>>,
    );
}

pub struct GameWindow {
    gi: Option<GameInfo>,
    cheats: Vec<Box<dyn Cheat>>,
    game_client_area: GameClientArea,
    old_update: bool,
    need_update: bool,
}

impl GameWindow {
    pub fn update_cheats(&mut self) -> Result<(), String> {
        if let Some(gi) = self.gi.as_mut() {
            if !gi.gm.pd.is_active() {
                println!("\nTry to find the game");
                if let Err(e) = gi.gm.pd.find_process("hl.exe") {
                    println!("Waiting for the game to start...");
                    std::thread::sleep(std::time::Duration::from_millis(1000));

                    return Err(e.to_string());
                }
                println!("Game found");

                for cheat in &self.cheats {
                    println!("\nCheat name: {}", cheat.name());
                    println!("Key activation: {}", cheat.key());
                }
            }

            if self.old_update != self.need_update {
                // Maybe update some cheat data
                self.game_client_area = gi.gm.get::<GameClientArea>();

                self.old_update = self.need_update;
            }

            let mut buffer = gi
                .surface
                .buffer_mut()
                .map_err(|e| format!("Get buffer for draw failed: {}", e))?;

            // Fill a buffer with a solid color
            buffer.fill(DARK_GRAY_CLEAR_COLOR);

            for cheat in &mut self.cheats {
                if cheat.key().enabled() {
                    cheat.update(&mut gi.gm, &mut self.game_client_area, &mut buffer);
                    self.need_update = true;
                } else {
                    self.need_update = false;
                }
            }

            buffer
                .present()
                .map_err(|e| format!("Render draw to window failed: {}", e))?;
        }

        Ok(())
    }
}

impl ApplicationHandler for GameWindow {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Ok((window, _context, surface)) = create_window(event_loop) {
            self.gi = Some(GameInfo::new(window, surface));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.update_cheats() {
                    eprintln!("RedrawRequested error: {}", e);
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.gi.as_ref().map(|gi| gi.window.request_redraw());
    }
}

impl Default for GameWindow {
    fn default() -> Self {
        Self {
            gi: None,
            cheats: Vec::new(),
            game_client_area: GameClientArea::new(),
            old_update: false,
            need_update: true,
        }
    }
}

pub struct Game {
    game_window: GameWindow,
}

impl Game {
    pub fn new() -> Self {
        Self {
            game_window: GameWindow::default(),
        }
    }

    pub fn register<T: Cheat + Default + 'static>(&mut self) {
        self.game_window.cheats.push(Box::new(T::default()));
    }

    pub fn run(&mut self) -> Result<(), String> {
        let mut builder = EventLoop::builder();
        let event_loop = builder
            .build()
            .map_err(|e| format!("Create event loop window error: {}", e))?;

        event_loop.set_control_flow(ControlFlow::Poll);

        event_loop
            .run_app(&mut self.game_window)
            .map_err(|e| format!("Run window app error: {}", e))?;

        Ok(())
    }
}
