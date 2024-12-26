mod app;
mod camera;
mod gfx_context;

use anyhow::Result;
use app::AppHandler;

use log::LevelFilter;
use winit::event_loop::{ControlFlow, EventLoop};

pub fn run() -> Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Warn)
        .filter_module("raytracer", LevelFilter::Debug)
        .init();

    log::debug!("hello??");

    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut AppHandler::new())?;

    Ok(())
}
