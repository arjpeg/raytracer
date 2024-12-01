struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
	@location(0) position: vec3<f32>,
}

@vertex
fn vs_main(
	@builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
	// full screen quad
	var positions: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
		vec2<f32>(-1.0, -1.0),
		vec2<f32>(-1.0, 1.0),
		vec2<f32>(1.0, 1.0),

		vec2<f32>(1.0, 1.0),
		vec2<f32>(1.0, -1.0),
		vec2<f32>(-1.0, -1.0),
	);

	var out: VertexOutput;

	out.clip_position = vec4<f32>(positions[in_vertex_index], 0.0, 1.0);
	out.position = vec3<f32>(positions[in_vertex_index], 0.0);

	return out;
}

struct PushConstants {
	resolution: vec2<f32>,
	zoom_center: vec2<f32>,
	zoom_factor: f32,
}

var<push_constant> c: PushConstants;
 
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let uv = (in.position.xy + 1.0) * 0.5;

	//let aspect_ratio = c.resolution.x / c.resolution.y;
	let aspect_ratio = c.zoom_center.x / c.zoom_center.y;

	let c = vec2<f32>(
			lerp(-2.5, 1.0, uv.x * aspect_ratio) - 1.0,
			lerp(-1.5, 1.5, uv.y)
	);

	let color = gradient(f32(mandelbrot(c)) / 100.0);

	return vec4<f32>(color, 1.0);
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
	return a * (1.0 - t) + b * t;
}

fn mandelbrot(c: vec2<f32>) -> u32 {
	var z: vec2<f32> = vec2<f32>(0.0, 0.0);
	var n: u32 = 0u;

	while (n < 100u) {
		z = vec2<f32>(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y) + c;

		if (length(z) > 2.0) {
			break;
		}

		n = n + 1u;
	}

	return n;
}

fn gradient(t: f32) -> vec3<f32> {
	let stop1 = vec3<f32>(0.0);
	let stop2 = vec3<f32>(0.03, 0.03, 0.47);
	let stop3 = vec3<f32>(0.9);

	if (t < 0.35) {
		let factor = t / 0.35;
		return mix(stop1, stop2, factor);
	} else {
		let factor = (t - 0.35) / 0.35;
		return mix(stop2, stop3, factor);
	}
}
