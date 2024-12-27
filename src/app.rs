use std::{sync::Arc, time::Instant};

use glam::Vec3;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{CursorGrabMode, Window, WindowId},
};

use anyhow::Result;

use crate::{
    camera::Camera,
    gfx_context::{GfxContext, Sphere},
};

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

    /// If the `window` currently has focus over the cursor.
    focused: bool,
}

pub enum AppHandler {
    Running(App),
    Initializing,
}

impl App {
    async fn new(window: Window) -> Result<Self> {
        use glam::vec3;

        let window = Arc::new(window);

        let camera = Camera::new_facing(vec3(0.0, 1.0, 4.0), Vec3::NEG_Z);
        let gfx_context = GfxContext::new(Arc::clone(&window), &camera).await?;

        let (egui_ctx, egui_state) = Self::initialize_egui(&window);

        Ok(Self {
            gfx_context,
            window,
            camera,
            egui_state,
            egui_ctx,
            dt: 0.0,
            last_frame: Instant::now(),
            focused: false,
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
        use WindowEvent as WE;

        let response = self.egui_state.on_window_event(&self.window, &event);

        if event != WE::RedrawRequested && response.consumed {
            return;
        }

        match event {
            WE::RedrawRequested => {
                self.update();
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

    fn update(&mut self) {
        use wgpu::SurfaceError as SE;

        let egui_output = self.ui();

        self.egui_state
            .handle_platform_output(&self.window, egui_output.platform_output.clone());

        self.dt = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = Instant::now();

        self.gfx_context.update_buffers(&mut self.camera);

        let hovering = self.egui_ctx.is_pointer_over_area();

        self.egui_ctx.input(|i| {
            if !hovering && i.pointer.primary_down() {
                self.window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
                self.window.set_cursor_visible(false);
                self.focused = true;
            } else {
                self.window.set_cursor_grab(CursorGrabMode::None).unwrap();
                self.window.set_cursor_visible(true);
                self.focused = false;
            }

            if self.focused {
                self.camera.handle_keyboard(i, self.dt);
            }
        });

        if let Err(e) = self.gfx_context.render(&self.egui_ctx, egui_output) {
            match e {
                SE::Timeout => (),
                SE::OutOfMemory => panic!("out of memory!"),
                SE::Lost | SE::Outdated => self.gfx_context.resize(self.window.inner_size()),
            }
        };

        self.window.request_redraw();
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
                    ui.add(DragValue::new(&mut self.camera.yaw).speed(0.1));
                    ui.add(DragValue::new(&mut self.camera.pitch).speed(0.1));
                });

                ui.horizontal(|ui| {
                    let direction = &mut self.gfx_context.render_uniform.light_direction;

                    ui.label("light direction: ");
                    ui.add(DragValue::new(&mut direction.x).speed(0.01));
                    ui.add(DragValue::new(&mut direction.y).speed(0.01));
                    ui.add(DragValue::new(&mut direction.z).speed(0.01));
                });

                ui.separator();

                ui.horizontal(|ui| {
                    let color = &mut self.gfx_context.render_uniform.sky_color;
                    let mut color_array = color.to_array();

                    ui.label("sky color: ");
                    ui.color_edit_button_rgb(&mut color_array);

                    color.x = color_array[0];
                    color.y = color_array[1];
                    color.z = color_array[2];
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("accumulate: ");

                    let accumulate = &mut self.gfx_context.render_uniform.accumulate;
                    let prev = *accumulate;
                    ui.checkbox(accumulate, "");

                    if *accumulate != prev {
                        self.gfx_context.reset_accumulation();
                    }
                });
            });

            Window::new("scene").show(ctx, |ui| {
                if ui.button("add sphere to scene").clicked() {
                    self.gfx_context.scene.add_sphere(Sphere::random());
                }

                ui.separator();

                for sphere in self.gfx_context.scene.spheres_mut() {
                    ui.horizontal(|ui| {
                        let position = &mut sphere.position;

                        ui.label("position: ");
                        ui.add(DragValue::new(&mut position.x).speed(0.01));
                        ui.add(DragValue::new(&mut position.y).speed(0.01));
                        ui.add(DragValue::new(&mut position.z).speed(0.01));
                    });

                    ui.horizontal(|ui| {
                        ui.label("radius: ");
                        ui.add(DragValue::new(&mut sphere.radius).speed(0.01));
                    });

                    ui.horizontal_top(|ui| {
                        ui.label("roughness: ");
                        ui.add(Slider::new(&mut sphere.roughness, 0.0..=1.0));
                    });

                    ui.horizontal(|ui| {
                        let color = &mut sphere.color;
                        let mut color_array = color.to_array();

                        ui.label("albedo: ");
                        ui.color_edit_button_rgb(&mut color_array);

                        color.x = color_array[0];
                        color.y = color_array[1];
                        color.z = color_array[2];
                    });

                    ui.separator();
                }
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
