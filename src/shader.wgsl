struct RenderUniform {
	inverse_projection: mat4x4<f32>,
	inverse_view: mat4x4<f32>,
	sphere_color: vec4<f32>,
	aspect_ratio: f32,
}


@group(0) @binding(0)
var<uniform> render_info: RenderUniform;

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
    let coord = in.position.xy;

    return vec4<f32>(per_pixel(coord), 1.0);
}

fn per_pixel(coord: vec2<f32>) -> vec3<f32> {
    let ray_origin = render_info.inverse_view[3].xyz;

    let ray_target = render_info.inverse_projection * vec4<f32>(coord, 1.0, 1.0);
    let ray_direction = (render_info.inverse_view * vec4<f32>(normalize(ray_target.xyz / ray_target.w), 0.0)).xyz;

    let sphere_color = render_info.sphere_color.xyz;
    let sphere_origin = vec3<f32>(0.0);

    let light_direction = normalize(vec3<f32>(-1.0, -1.0, -1.0));

    let radius = 0.5;

    let a = dot(ray_direction, ray_direction);
    let b = 2.0 * dot(ray_origin, ray_direction);
    let c = dot(ray_origin, ray_origin) - radius * radius;

    let discriminant = (b * b) - (4.0 * a * c);

    if discriminant <= 0 {
        return vec3<f32>(0.0);
    }

    let t = (-b - sqrt(discriminant)) / (2.0 * a);
    let hit_postion = ray_origin + ray_direction * t;
    let normal = normalize(hit_postion - sphere_origin);

    return max(dot(normal, -light_direction), 0.0) * sphere_color;
}
