mod gfx_context;

use std::sync::Arc;

use gfx_context::GfxContext;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use anyhow::Result;

pub struct App {
    window: Arc<Window>,
    gfx_context: GfxContext,
}

pub enum AppHandler {
    Running(App),
    Initializing,
}

impl App {
    async fn new(window: Window) -> Result<Self> {
        let window = Arc::new(window);
        let gfx_context = GfxContext::new(Arc::clone(&window)).await?;

        Ok(Self {
            gfx_context,
            window,
        })
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        use WindowEvent::*;

        match event {
            RedrawRequested => {
                self.window.request_redraw();
                self.gfx_context.render().expect("failed rendering")
            }

            Resized(size) => self.gfx_context.resize(size),

            CloseRequested => event_loop.exit(),

            _ => {}
        }
    }
}

impl AppHandler {
    pub fn new() -> Self {
        Self::Initializing
    }
}

impl ApplicationHandler for AppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_inner_size(LogicalSize::new(1920, 1080))
                    .with_title("ray tracer"),
            )
            .expect("failed to create window");

        let app = pollster::block_on(App::new(window)).expect("failed to initialize app");

        *self = AppHandler::Running(app);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Self::Running(ref mut app) = self else {
            return;
        };

        app.window_event(event_loop, event);
    }
}

pub fn run() -> Result<()> {
    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut AppHandler::new())?;

    Ok(())
}
