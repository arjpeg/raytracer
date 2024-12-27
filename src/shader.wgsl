struct RenderUniform {
	inverse_projection: mat4x4<f32>,
	inverse_view: mat4x4<f32>,
	light_direction: vec3<f32>,
	aspect_ratio: f32,
}

struct Scene {
	spheres: array<Sphere>,
}

struct Sphere {
	position: vec4<f32>,
	albedo: vec3<f32>,
	radius: f32,
}

@group(0) @binding(0)
var<uniform> render_info: RenderUniform;

@group(0) @binding(1)
var<storage> scene: Scene;

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
    return per_pixel(in.position.xy);
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
    let direction = (inverse_view * vec4<f32>(normalize(target_.xyz / target_.w), 0.0)).xyz; // cast into world space

    var ray = Ray(origin, direction);
    let bounces = 2;

    var color = vec3<f32>(0.0);
    var multiplier = 1.0;

    for (var i = 0; i < bounces; i++) {
        let hit = trace_ray(ray);

        if hit.hit_distance < 0.0 {
            let sky_color = vec3<f32>(0.0);
            color += sky_color * multiplier;
						break;
        }

        let sphere = scene.spheres[hit.object_index];

        let light_intensity = max(dot(hit.normal, -normalize(render_info.light_direction)), 0.01);

        color += sphere.albedo * light_intensity;

        ray.origin = hit.position + hit.normal * 0.001;
        ray.direction = hit.normal;

        multiplier *= 0.7;
    }

    return vec4<f32>(color, 1.0);
}

fn trace_ray(ray: Ray) -> HitPayload {
    var closest_sphere = -1;
    var hit_distance = bitcast<f32>(0x7f800000);

    for (var i = 0; i < i32(arrayLength(&scene.spheres)); i++) {
        let sphere = scene.spheres[i];

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

    let sphere = scene.spheres[object_index];
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
