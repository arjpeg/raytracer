mod app;
mod camera;
mod gfx_context;

use anyhow::Result;
use app::AppHandler;

use winit::event_loop::{ControlFlow, EventLoop};

pub fn run() -> Result<()> {
    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut AppHandler::new())?;

    Ok(())
}
