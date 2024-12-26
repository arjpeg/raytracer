use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use egui::{InputState, Modifiers};
use glam::{vec3, Mat4, Vec3};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{CursorGrabMode, Window, WindowId},
};

use crate::gfx_context::GfxContext;
use anyhow::Result;

pub struct App {
    /// The main surface being displayed onto.
    window: Arc<Window>,
    /// The state and context of the connection to the rendering device.
    gfx_context: GfxContext,
    /// The current camera from which the world is being drawn.
    camera: Camera,

    /// The egui winit side state of the window to manage events.
    egui_state: egui_winit::State,
    /// The actual egui context to render ui.
    egui_ctx: egui::Context,

    /// The time in seconds since the last frame, also known as delta time.
    dt: f32,
    /// The time of the last frame.
    last_frame: Instant,
}

pub enum AppHandler {
    Running(App),
    Initializing,
}

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub eye: glam::Vec3,
    pub forward: glam::Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

impl App {
    async fn new(window: Window) -> Result<Self> {
        let window = Arc::new(window);
        let gfx_context = GfxContext::new(Arc::clone(&window)).await?;

        let (egui_ctx, egui_state) = Self::initialize_egui(&window);

        let camera = Camera {
            eye: vec3(0.0, 0.0, 2.0),
            forward: vec3(0.0, 0.0, -1.0),
            yaw: 270.0,
            pitch: 0.0,
        };

        Ok(Self {
            gfx_context,
            window,
            camera,
            egui_state,
            egui_ctx,
            dt: 0.0,
            last_frame: Instant::now(),
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

                self.dt = self.last_frame.elapsed().as_secs_f32();
                self.last_frame = Instant::now();

                self.gfx_context.update_buffers(&self.camera);

                let hovering = self.egui_ctx.is_pointer_over_area();

                self.egui_ctx.input(|i| {
                    self.camera.handle_keyboard(i, self.dt);

                    if !hovering && i.pointer.primary_down() {
                        self.window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
                        self.window.set_cursor_visible(false);
                    } else {
                        self.window.set_cursor_grab(CursorGrabMode::None).unwrap();
                        self.window.set_cursor_visible(true);
                    }
                });

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

    fn device_event(&mut self, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.egui_ctx.is_pointer_over_area() {
                return;
            }

            self.egui_ctx.input(|i| self.camera.handle_mouse(i, delta));
        }
    }

    fn ui(&mut self) -> egui::FullOutput {
        use egui::*;

        let raw_input = self.egui_state.take_egui_input(&self.window);

        self.egui_ctx.run(raw_input, |ctx| {
            Window::new("render info").show(ctx, |ui| {
                ui.label(&format!("frame time: {:0.3}", self.dt * 1000.0));

                ui.separator();

                ui.horizontal(|ui| {
                    let eye = &mut self.camera.eye;

                    ui.label("camera position: ");
                    ui.add(DragValue::new(&mut eye.x).speed(0.01));
                    ui.add(DragValue::new(&mut eye.y).speed(0.01));
                    ui.add(DragValue::new(&mut eye.z).speed(0.01));
                });

                ui.horizontal(|ui| {
                    ui.label("camera forward: ");
                    ui.add(DragValue::new(&mut self.camera.yaw).speed(0.01));
                    ui.add(DragValue::new(&mut self.camera.pitch).speed(0.01));
                });

                ui.separator();

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

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        let Self::Running(ref mut app) = self else {
            return;
        };

        app.device_event(event);
    }
}

impl Camera {
    pub fn calculate_projection(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh(45.0f32.to_radians(), aspect_ratio, 0.1, 1000.0)
    }

    pub fn calculate_view(&self) -> Mat4 {
        Mat4::look_to_rh(self.eye, self.forward.normalize(), Vec3::Y)
    }

    fn handle_keyboard(&mut self, input: &InputState, dt: f32) {
        use egui::Key;

        let forward = self.forward;
        let right = forward.cross(Vec3::Y);

        let mut delta_pos = Vec3::ZERO;

        if input.key_down(Key::W) {
            delta_pos += forward;
        }
        if input.key_down(Key::S) {
            delta_pos -= forward;
        }
        if input.key_down(Key::D) {
            delta_pos += right;
        }
        if input.key_down(Key::A) {
            delta_pos -= right;
        }

        if input.key_down(Key::Space) {
            delta_pos += Vec3::Y;
        }

        if input.modifiers.contains(Modifiers::SHIFT) {
            delta_pos -= Vec3::Y;
        }

        delta_pos = delta_pos.normalize_or_zero();
        let speed = 5.0;
        self.eye += speed * dt * delta_pos;
    }

    fn handle_mouse(&mut self, input: &InputState, delta: (f64, f64)) {
        if input.pointer.primary_down() {
            let mouse_sensitivity = 0.1;

            let dx = delta.0 as f32 * mouse_sensitivity;
            let dy = delta.1 as f32 * mouse_sensitivity;

            self.yaw += dx;
            self.pitch -= dy;

            self.pitch = self.pitch.clamp(-89.0, 89.0);
        }

        self.forward = vec3(
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos(),
        )
        .normalize();
    }
}
