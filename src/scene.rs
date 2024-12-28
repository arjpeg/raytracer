use std::sync::OnceLock;

use wgpu::{util::*, *};

use crate::gfx_context::GfxContext;

/// A description of all the primitives and materials currently being rendered.
#[derive(Debug)]
pub struct Scene {
    /// The spheres currently in the scene.
    spheres: Vec<Sphere>,
    /// The materials loaded in the scene.
    materials: Vec<Material>,

    /// A handle to the uploaded sphere data in the GPU.
    spheres_buffer: wgpu::Buffer,
    /// A handle to the uploaded material data in the GPU.
    materials_buffer: wgpu::Buffer,

    /// The bind group referencing both the buffers.
    bind_group: wgpu::BindGroup,

    /// If the size of `self.spheres` changed in the last frame (need to allocate a new buffer).
    spheres_size_changed: bool,
    /// If the size of `self.materials` changed in the last frame (need to allocate a new buffer).
    materials_size_changed: bool,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Sphere {
    /// The position of the sphere in 3d space.
    pub position: glam::Vec4,
    /// The radius of the sphere.
    pub radius: f32,

    /// The index of the material of the sphere.
    pub material_index: u32,

    padding: [u32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Material {
    /// The unlit, diffuse component of the material.
    pub albedo: glam::Vec3,
    /// How much light gets scattered when hitting this material.
    /// A value of zero means no light is scattered (perfectly smooth), while one means
    /// lightly is fully randomly scattered.
    pub roughness: f32,

    /// The color that this material emits.
    pub emission_color: glam::Vec3,
    /// The strength at which this material emits emission.
    pub emission_strength: f32,
}

impl Scene {
    /// Creates a new [`Scene`].
    pub fn new(gfx_context: &GfxContext) -> Self {
        use glam::{vec3, vec4};

        let spheres = vec![Sphere {
            position: vec4(0.0, 0.0, 0.0, 0.0),
            radius: 0.5,
            material_index: 0,
            padding: [0; 2],
        }];
        let materials = vec![Material {
            albedo: vec3(0.6, 0.2, 0.7),
            roughness: 0.2,
            emission_color: vec3(0.0, 0.0, 0.0),
            emission_strength: 0.0,
        }];

        let spheres_buffer = Self::create_spheres_buffer(gfx_context, &spheres);
        let materials_buffer = Self::create_materials_buffer(gfx_context, &materials);

        let bind_group = Self::create_bind_group(&gfx_context, &spheres_buffer, &materials_buffer);

        Self {
            spheres,
            materials,
            bind_group,
            spheres_buffer,
            materials_buffer,
            spheres_size_changed: false,
            materials_size_changed: false,
        }
    }

    pub fn add_sphere(&mut self, sphere: Sphere) {
        self.spheres.push(sphere);
        self.spheres_size_changed = true;
    }

    pub fn spheres_mut(&mut self) -> &mut [Sphere] {
        &mut self.spheres
    }

    pub fn update_buffers(&mut self, gfx_context: &GfxContext) {
        let recreate_bind_group = self.spheres_size_changed || self.materials_size_changed;

        let spheres_bytes = bytemuck::cast_slice(&self.spheres);
        let materials_bytes = bytemuck::cast_slice(&self.materials);

        if self.spheres_size_changed {
            self.spheres_size_changed = false;
            self.spheres_buffer = Self::create_spheres_buffer(gfx_context, &self.spheres);
        }

        if self.materials_size_changed {
            self.materials_size_changed = false;
            self.materials_buffer = Self::create_materials_buffer(gfx_context, &self.materials);
        }

        if recreate_bind_group {
            self.bind_group =
                Self::create_bind_group(gfx_context, &self.spheres_buffer, &self.materials_buffer);
        }

        gfx_context
            .queue
            .write_buffer(&self.spheres_buffer, 0, spheres_bytes);

        gfx_context
            .queue
            .write_buffer(&self.materials_buffer, 0, materials_bytes);
    }

    pub fn create_bind_group_layout(device: &Device) -> &'static BindGroupLayout {
        static LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();

        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Scene Bind Group Layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
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
            })
        })
    }

    fn create_bind_group(
        gfx_context: &GfxContext,
        sphere_buffer: &Buffer,
        material_buffer: &Buffer,
    ) -> BindGroup {
        gfx_context.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Scene Bind Group"),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: sphere_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: material_buffer.as_entire_binding(),
                },
            ],
            layout: Self::create_bind_group_layout(&gfx_context.device),
        })
    }

    pub fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }

    /// Utility function to create a new buffer, and upload all the given data to the GPU.
    fn create_buffer(gfx_context: &GfxContext, label: &str, data: &[u8]) -> Buffer {
        gfx_context
            .device
            .create_buffer_init(&BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(data),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            })
    }

    fn create_spheres_buffer(gfx_context: &GfxContext, spheres: &[Sphere]) -> Buffer {
        Self::create_buffer(
            gfx_context,
            "Scene Spheres Storage Buffer",
            bytemuck::cast_slice(spheres),
        )
    }

    fn create_materials_buffer(gfx_context: &GfxContext, materials: &[Material]) -> Buffer {
        Self::create_buffer(
            gfx_context,
            "Scene Materials Storage Buffer",
            bytemuck::cast_slice(materials),
        )
    }
}

impl Sphere {
    /// Creates a new [`Sphere`], with a random position and radius and a material referencing the
    /// first material in the [`Scene`].
    pub fn random() -> Sphere {
        use glam::vec4;
        use rand::Rng;

        let mut rng = rand::thread_rng();

        let position = vec4(
            rng.gen_range(-5.0..5.0),
            rng.gen_range(-5.0..5.0),
            rng.gen_range(-5.0..5.0),
            1.0,
        );
        let radius = rng.gen_range(0.3..1.2);

        Sphere {
            position,
            radius,
            material_index: 0,
            padding: [0; 2],
        }
    }
}
