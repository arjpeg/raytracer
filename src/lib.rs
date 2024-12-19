mod gfx_context;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use anyhow::Result;

use gfx_context::GfxContext;

pub struct App {
    /// The main surface being displayed onto.
    window: Arc<Window>,
    /// The state and context of the connection to the rendering device.
    gfx_context: GfxContext,

    /// The egui winit side state of the window to manage events.
    egui_state: egui_winit::State,
    /// The actual egui context to render ui.
    egui_ctx: egui::Context,

    /// Information about the frame counter (used to track performance).
    frame_timer: FrameTimer,
}

pub enum AppHandler {
    Running(App),
    Initializing,
}

struct FrameTimer {
    prev_frame_time: std::time::Instant,
    fps: Option<f64>,
    counter: u32,
}

impl FrameTimer {
    fn new() -> Self {
        Self {
            prev_frame_time: Instant::now(),
            fps: None,
            counter: 0,
        }
    }

    fn update(&mut self) {
        let current_time = Instant::now();
        let elapsed = current_time - self.prev_frame_time;

        self.counter += 1;

        if elapsed > Duration::from_secs(1) {
            let fps = self.counter as f64 / elapsed.as_secs_f64();

            self.counter = 0;
            self.prev_frame_time = current_time;
            self.fps = Some(fps);
        }
    }
}

impl App {
    async fn new(window: Window) -> Result<Self> {
        let window = Arc::new(window);
        let gfx_context = GfxContext::new(Arc::clone(&window)).await?;

        let (egui_ctx, egui_state) = Self::initialize_egui(&window);

        let frame_timer = FrameTimer::new();

        Ok(Self {
            gfx_context,
            window,
            egui_state,
            egui_ctx,
            frame_timer,
        })
    }

    fn initialize_egui(window: &Window) -> (egui::Context, egui_winit::State) {
        use egui::*;
        use egui_winit::State;

        let ctx = Context::default();
        let state = State::new(ctx.clone(), ctx.viewport_id(), window, None, None, None);

        (ctx, state)
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        use wgpu::SurfaceError as SE;
        use WindowEvent as WE;

        let response = self.egui_state.on_window_event(&self.window, &event);

        if response.consumed {
            return;
        }

        match event {
            WE::RedrawRequested => {
                self.window.request_redraw();

                let egui_output = self.ui();

                self.egui_state
                    .handle_platform_output(&self.window, egui_output.platform_output.clone());

                self.frame_timer.update();

                self.gfx_context.update_buffers();

                if let Err(e) = self.gfx_context.render(&self.egui_ctx, egui_output) {
                    match e {
                        SE::Timeout => (),
                        SE::OutOfMemory => panic!("out of memory!"),
                        SE::Lost | SE::Outdated => {
                            self.gfx_context.resize(self.window.inner_size())
                        }
                    }
                };
            }

            WE::Resized(size) => self.gfx_context.resize(size),

            WE::CloseRequested => event_loop.exit(),

            _ => {}
        }
    }

    fn ui(&mut self) -> egui::FullOutput {
        use egui::*;

        let raw_input = self.egui_state.take_egui_input(&self.window);

        self.egui_ctx.run(raw_input, |ctx| {
            Window::new("render info").show(ctx, |ui| {
                ui.label(&format!("fps: {:0.2}", self.frame_timer.fps.unwrap_or(0.0)));

                ui.add(Separator::default());

                ui.horizontal(|ui| {
                    let eye = &mut self.gfx_context.render_uniform.camera.eye;

                    ui.label("camera position: ");
                    ui.add(DragValue::new(&mut eye.x).speed(0.01));
                    ui.add(DragValue::new(&mut eye.y).speed(0.01));
                    ui.add(DragValue::new(&mut eye.z).speed(0.01));
                });

                ui.horizontal(|ui| {
                    let color = &mut self.gfx_context.render_uniform.sphere_color;
                    let mut color_array = color.truncate().to_array();

                    ui.label("sphere color: ");
                    ui.color_edit_button_rgb(&mut color_array);

                    color.x = color_array[0];
                    color.y = color_array[1];
                    color.z = color_array[2];
                });
            });
        })
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

        window.request_redraw();

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
