use std::sync::Arc;

use egui_wgpu::ScreenDescriptor;

use glam::*;
use wgpu::{util::*, *};
use winit::{dpi::PhysicalSize, window::Window};

use anyhow::Result;

use crate::camera::Camera;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::NoUninit)]
pub struct RenderUniform {
    pub inverse_projection: glam::Mat4,
    pub inverse_view: glam::Mat4,
    pub light_direction: glam::Vec3,
    pub aspect_ratio: f32,
    pub sky_color: glam::Vec3,
    pub time: f32,
    pub dimensions: glam::UVec2,
    pub frames_accumulated: u32,
    pub accumulate: bool,
    pub padding: [u8; 3],
}

#[derive(Debug, Clone)]
pub struct Scene {
    spheres: Vec<Sphere>,
    size_changed: bool,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Sphere {
    pub position: glam::Vec4,
    pub color: glam::Vec3,
    pub radius: f32,
    pub roughness: f32,

    pub padding: [u8; 12],
}

#[derive(Debug)]
pub struct AccumulationBuffer {
    pub bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
}

pub struct GfxContext {
    /// The actual physical device responsible for rendering things (most likely the GPU).
    device: wgpu::Device,
    /// The queue of commands being staged to be sent to the `device`.
    queue: wgpu::Queue,
    /// The series of steps that data takes while moving through the rendering process.
    pipeline: wgpu::RenderPipeline,

    /// The actual window, being targeted by the `surface`
    window: Arc<Window>,
    /// A reference to the surface being rendered onto.
    surface: wgpu::Surface<'static>,
    /// The configuration of the `surface`.
    surface_config: wgpu::SurfaceConfiguration,

    /// The main egui renderer.
    egui_renderer: egui_wgpu::Renderer,

    render_data_bind_group: wgpu::BindGroup,

    pub render_uniform: RenderUniform,
    render_uniform_buffer: wgpu::Buffer,

    /// A description of the scene to be rendered.
    pub scene: Scene,
    scene_storage_buffer: wgpu::Buffer,

    accumulation_buffer: AccumulationBuffer,
}

impl GfxContext {
    /// Creates a new renderer given a window as the surface.
    pub async fn new(window: Arc<Window>, camera: &Camera) -> Result<Self> {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::PRIMARY,
            flags: InstanceFlags::empty(),
            ..Default::default()
        });

        let surface = instance.create_surface(Arc::clone(&window))?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default(), None)
            .await?;

        let surface_config = Self::get_surface_config(&adapter, &surface, window.inner_size());
        surface.configure(&device, &surface_config);

        let render_uniform = RenderUniform::new(window.inner_size(), camera);
        let render_uniform_buffer = render_uniform.create_buffer(&device);

        let scene = Scene {
            spheres: vec![
                Sphere {
                    position: vec4(0.0, -12.0, 0.0, 0.0),
                    color: vec3(0.0, 0.0, 1.0),
                    radius: 12.0,
                    roughness: 0.3,
                    padding: [0; 12],
                },
                Sphere {
                    position: vec4(0.0, 0.6, 0.0, 0.0),
                    color: vec3(1.0, 1.0, 1.0),
                    radius: 0.5,
                    roughness: 0.7,
                    padding: [0; 12],
                },
                Sphere {
                    position: vec4(-2.61, 0.0, 3.91, 0.0),
                    color: vec3(1.0, 0.0, 0.0),
                    radius: 2.75,
                    roughness: 0.7,
                    padding: [0; 12],
                },
            ],
            size_changed: false,
        };

        let scene_storage_buffer = scene.create_buffer(&device);

        let (render_data_bind_group, render_data_bind_group_layout) =
            Self::create_render_data_bind_group(
                &device,
                &render_uniform_buffer,
                &scene_storage_buffer,
            );

        let accumulation_buffer = AccumulationBuffer::new(&device, window.inner_size());

        let pipeline = Self::create_pipeline(
            &device,
            &surface_config,
            device.create_shader_module(include_wgsl!("shader.wgsl")),
            &[
                &render_data_bind_group_layout,
                &accumulation_buffer.bind_group_layout,
            ],
        );

        let egui_renderer =
            egui_wgpu::Renderer::new(&device, surface_config.format, None, 1, false);

        Ok(Self {
            device,
            queue,
            pipeline,
            window,
            surface,
            surface_config,
            egui_renderer,
            render_data_bind_group,
            render_uniform,
            render_uniform_buffer,
            scene,
            scene_storage_buffer,
            accumulation_buffer,
        })
    }

    /// Creates the rendering pipeline.
    fn create_pipeline(
        device: &Device,
        surface_config: &SurfaceConfiguration,
        shader: ShaderModule,
        bind_group_layouts: &[&BindGroupLayout],
    ) -> RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            push_constant_ranges: &[],
            bind_group_layouts,
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: surface_config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Cw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        })
    }

    /// Creates a surface configuration given an adapter, surface, and surface size.
    /// Does not apply the created config to the surface
    fn get_surface_config(
        adapter: &Adapter,
        surface: &Surface,
        size: PhysicalSize<u32>,
    ) -> SurfaceConfiguration {
        let PhysicalSize { width, height } = size;
        let surface_caps = surface.get_capabilities(adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .cloned()
            .find(TextureFormat::is_srgb)
            .unwrap();

        SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        }
    }

    /// Resizes the renderer's `config` to match the new given size.
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        let PhysicalSize { width, height } = size;

        assert!(width > 0, "cannot resize to zero width");
        assert!(height > 0, "cannot resize to zero height");

        self.surface_config.width = width;
        self.surface_config.height = height;

        self.surface.configure(&self.device, &self.surface_config);

        self.render_uniform.aspect_ratio = width as f32 / height as f32;
        self.render_uniform.dimensions = uvec2(width, height);

        self.reset_accumulation();
    }

    pub fn update_buffers(&mut self, camera: &mut Camera) {
        let projection = camera.calculate_projection(self.render_uniform.aspect_ratio);
        let view = camera.calculate_view();

        self.render_uniform.inverse_projection = projection.inverse();
        self.render_uniform.inverse_view = view.inverse();

        self.render_uniform.time += 0.01;
        self.render_uniform.frames_accumulated += 1;

        if camera.moved {
            camera.moved = false;

            self.reset_accumulation();
        }

        if self.scene.size_changed {
            self.scene.size_changed = false;
            self.scene_storage_buffer = self.scene.create_buffer(&self.device);

            self.render_data_bind_group = Self::create_render_data_bind_group(
                &self.device,
                &self.render_uniform_buffer,
                &self.scene_storage_buffer,
            )
            .0;
        }

        self.queue.write_buffer(
            &self.scene_storage_buffer,
            0,
            bytemuck::cast_slice(&self.scene.spheres),
        );

        self.queue.write_buffer(
            &self.render_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.render_uniform]),
        );
    }

    /// Renders the currently bound vertex buffer onto the `surface`.
    pub fn render(
        &mut self,
        egui_ctx: &egui::Context,
        egui_output: egui::FullOutput,
    ) -> Result<(), SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&TextureViewDescriptor {
            label: Some("Render View"),
            ..Default::default()
        });

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.main_render_pass(&mut encoder, &view);
        self.egui_render_pass(&mut encoder, &view, egui_ctx, egui_output);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn main_render_pass(&self, encoder: &mut CommandEncoder, view: &TextureView) {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.01,
                        g: 0.01,
                        b: 0.01,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        render_pass.set_pipeline(&self.pipeline);

        render_pass.set_bind_group(0, &self.render_data_bind_group, &[]);
        render_pass.set_bind_group(1, &self.accumulation_buffer.bind_group, &[]);

        render_pass.draw(0..6, 0..1);
    }

    fn egui_render_pass(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        ctx: &egui::Context,
        full_output: egui::FullOutput,
    ) {
        let tris = ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.surface_config.width, self.surface_config.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, &image_delta);
        }

        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            encoder,
            &tris,
            &screen_descriptor,
        );

        let mut render_pass = encoder
            .begin_render_pass(&RenderPassDescriptor {
                color_attachments: &[Some(RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                label: Some("Egui Main Render Pass"),
                timestamp_writes: None,
                occlusion_query_set: None,
            })
            .forget_lifetime();

        self.egui_renderer
            .render(&mut render_pass, &tris, &screen_descriptor);

        drop(render_pass);

        for x in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(x)
        }
    }

    fn create_render_data_bind_group(
        device: &Device,
        uniform_buffer: &Buffer,
        storage_buffer: &Buffer,
    ) -> (BindGroup, BindGroupLayout) {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Render Information Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Render Information Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: storage_buffer.as_entire_binding(),
                },
            ],
        });

        (bind_group, bind_group_layout)
    }

    pub fn reset_accumulation(&mut self) {
        self.accumulation_buffer
            .reset(&self.device, self.window.inner_size());

        self.render_uniform.frames_accumulated = 1;
    }
}

impl RenderUniform {
    fn new(size: PhysicalSize<u32>, camera: &Camera) -> Self {
        let aspect_ratio = size.width as f32 / size.height as f32;

        Self {
            inverse_projection: camera.calculate_projection(aspect_ratio).inverse(),
            inverse_view: camera.calculate_view().inverse(),
            light_direction: vec3(-0.25, -0.23, 0.12).normalize(),
            aspect_ratio,
            sky_color: vec3(0.01, 0.01, 0.01),
            time: 0.0,
            dimensions: uvec2(size.width, size.height),
            frames_accumulated: 0,
            accumulate: true,
            padding: [0; 3],
        }
    }

    fn create_buffer(&self, device: &Device) -> Buffer {
        device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Render Uniform Buffer"),
            contents: bytemuck::cast_slice(&[*self]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        })
    }
}

#[allow(dead_code)]
impl Scene {
    fn create_buffer(&self, device: &Device) -> Buffer {
        device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Scene Storage Buffer"),
            contents: bytemuck::cast_slice(&self.spheres),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        })
    }

    pub fn add_sphere(&mut self, sphere: Sphere) {
        self.spheres.push(sphere);
        self.size_changed = true;
    }

    pub fn spheres(&self) -> &[Sphere] {
        &self.spheres
    }

    pub fn spheres_mut(&mut self) -> &mut [Sphere] {
        &mut self.spheres
    }
}

impl Sphere {
    pub fn random() -> Self {
        use glam::{vec3, vec4};
        use rand::Rng;

        let mut rng = rand::thread_rng();

        let position = vec4(
            rng.gen_range(-5.0..5.0),
            rng.gen_range(-5.0..5.0),
            rng.gen_range(-5.0..5.0),
            0.0,
        );

        let color = vec3(rng.gen(), rng.gen(), rng.gen());

        let radius = rng.gen_range(0.3..1.2);

        let roughness = rng.gen();

        Self {
            position,
            color,
            radius,
            roughness,
            padding: [0; 12],
        }
    }
}

impl AccumulationBuffer {
    fn new(device: &Device, size: PhysicalSize<u32>) -> Self {
        let buffer_size = Self::calculate_bytes(size);

        let buffer = Self::create_buffer(device, buffer_size);

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Accumulation Buffer Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = Self::create_bind_group(device, &bind_group_layout, &buffer);

        Self {
            bind_group,
            bind_group_layout,
            buffer,
        }
    }

    fn calculate_bytes(size: PhysicalSize<u32>) -> u64 {
        size.width as u64 * size.height as u64 * size_of::<Vec4>() as u64
    }

    fn create_buffer(device: &Device, bytes: u64) -> Buffer {
        device.create_buffer(&BufferDescriptor {
            label: Some("Accumulation Storage Buffer"),
            size: bytes,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        })
    }

    fn create_bind_group(device: &Device, layout: &BindGroupLayout, buffer: &Buffer) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("Accumulation Buffer Bind Group"),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            layout,
        })
    }

    fn reset(&mut self, device: &Device, size: PhysicalSize<u32>) {
        self.buffer = Self::create_buffer(device, Self::calculate_bytes(size));
        self.bind_group = Self::create_bind_group(device, &self.bind_group_layout, &self.buffer);
    }
}
