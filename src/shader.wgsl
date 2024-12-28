struct RenderUniform {
	inverse_projection: mat4x4<f32>,
	inverse_view: mat4x4<f32>,

	sky_color: vec3<f32>,
	time: f32,
	screen_dimensions: vec2<u32>,
	frames_accumulated: u32,
	accumulate: u32,
}

struct Sphere {
	position: vec4<f32>,
	radius: f32,
	material_index: u32,
}

struct Material {
	albedo: vec3<f32>,
	roughness: f32,
	emission_color: vec3<f32>,
	emission_strength: f32,
}

@group(0) @binding(0)
var<uniform> render_info: RenderUniform;

@group(1) @binding(0)
var<storage, read_write> accumulation: array<vec4<f32>>;

@group(2) @binding(0)
var<storage> spheres: array<Sphere>;

@group(2) @binding(1)
var<storage> materials: array<Material>;


struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
	@location(0) position: vec2<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, -1.0),
    );

    var out: VertexOutput;

    out.clip_position = vec4<f32>(positions[in_vertex_index], 0.0, 1.0);
    out.position = vec2<f32>(positions[in_vertex_index]);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = per_pixel(in.position.xy);

    if render_info.accumulate != 0 {
        let index = calculate_index(in.position.xy);
        accumulation[index] += color;

        return accumulation[index] / f32(render_info.frames_accumulated);
    }

    return color;
}

fn calculate_index(coord: vec2<f32>) -> u32 {
    let screen_dimensions = render_info.screen_dimensions;

		// convert to [0, 1)
    let normalized_coord = coord * 0.5 + 0.5;
    let pixel_coord = vec2<u32>(normalized_coord * vec2<f32>(screen_dimensions));
    return pixel_coord.y * screen_dimensions.x + pixel_coord.x;
}

struct Ray {
	origin: vec3<f32>,
	direction: vec3<f32>,
}

struct HitPayload {
	hit_distance: f32,
	position: vec3<f32>,
	normal: vec3<f32>,
	object_index: u32
}

fn per_pixel(coord: vec2<f32>) -> vec4<f32> {
    let inverse_view = render_info.inverse_view;

    let origin = inverse_view[3].xyz;
    let target_ = render_info.inverse_projection * vec4<f32>(coord, 1.0, 1.0);
		// cast into world space
    let direction = (inverse_view * vec4<f32>(normalize(target_.xyz / target_.w), 0.0)).xyz;

    var ray = Ray(origin, direction);
    let bounces = 5;

    var light = vec3<f32>(0.0);
    var contribution = vec3<f32>(1.0);

    var rng = initial_seed(coord);

    for (var i = 0; i < bounces; i++) {
        let hit = trace_ray(ray);

        if hit.hit_distance < 0.0 {
            light += render_info.sky_color * contribution;
						break;
        }

        let sphere = spheres[hit.object_index];
        let material = materials[sphere.material_index];

        contribution *= material.albedo;
        light += material.emission_color * material.emission_strength;

        let scatter_direction = next_random_vec3(&rng);

        ray.origin = hit.position + hit.normal * 0.0001;
        ray.direction = normalize(hit.normal + scatter_direction);
    }

    return vec4<f32>(light, 1.0);
}


fn trace_ray(ray: Ray) -> HitPayload {
    var closest_sphere = -1;
    var hit_distance = bitcast<f32>(0x7f800000);

    for (var i = 0; i < i32(arrayLength(&spheres)); i++) {
        let sphere = spheres[i];

        let origin = ray.origin - sphere.position.xyz;

        let a = dot(ray.direction, ray.direction);
        let b = 2.0 * dot(origin, ray.direction);
        let c = dot(origin, origin) - sphere.radius * sphere.radius;

        let discriminant = (b * b) - (4.0 * a * c);

        if discriminant <= 0 {
					continue;
        }

        let t = (-b - sqrt(discriminant)) / (2.0 * a);

        if t >= 0 && t < hit_distance {
            hit_distance = t;
            closest_sphere = i;
        }
    }

    if closest_sphere == -1 {
        return miss(ray);
    }

    return closest_hit(ray, hit_distance, u32(closest_sphere));
}

fn closest_hit(ray: Ray, hit_distance: f32, object_index: u32) -> HitPayload {
    var payload: HitPayload;

    payload.hit_distance = hit_distance;
    payload.object_index = object_index;

    let sphere = spheres[object_index];
    let origin = ray.origin - sphere.position.xyz;

    payload.position = origin + ray.direction * hit_distance;
    payload.normal = normalize(payload.position);
    payload.position += sphere.position.xyz;

    return payload;
}

fn miss(ray: Ray) -> HitPayload {
    var payload: HitPayload;
    payload.hit_distance = -1.0;

    return payload;
}

fn initial_seed(coord: vec2<f32>) -> u32 {
    let translated = coord * 0.5 + 0.5;
    let x = translated.x * 1000.0;
    let y = translated.y * 1000.0;
    let t = pow(render_info.time, 2.0) * 1000.0;

    return u32(x) ^ (u32(y) << 16u) ^ u32(t);
}

fn next_random(rng: ptr<function, u32>) -> f32 {
    (*rng) = (*rng) ^ ((*rng) >> 16u);
    (*rng) = (*rng) * 0x85ebca6bu;
    (*rng) = (*rng) ^ ((*rng) >> 13u);
    (*rng) = (*rng) * 0xc2b2ae35u;
    (*rng) = (*rng) ^ ((*rng) >> 16u);

    return f32((*rng) & 0x007fffffu) / f32(0x00800000u);
}

fn next_random_vec3(rng: ptr<function, u32>) -> vec3<f32> {
    let x = next_random(rng);
    let y = next_random(rng);
    let z = next_random(rng);

    return vec3<f32>(x, y, z);
}
