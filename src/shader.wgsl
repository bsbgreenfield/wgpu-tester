struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) tex_coords: vec2<f32>,
}

struct InstanceInput {
  @location(3) obj_matrix_0: vec4<f32>,
  @location(4) obj_matrix_1: vec4<f32>,
  @location(5) obj_matrix_2: vec4<f32>,
  @location(6) obj_matrix_3: vec4<f32>,
  @location(7) model_index: u32,
}


struct GlobalTransforms{
transforms: array<mat4x4<f32>>,
}
struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) tex_coords: vec2<f32>,
}

struct CameraUniform {
  transform: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera_uniform: CameraUniform;

@group(1) @binding(1)
var<storage, read> global_transforms: GlobalTransforms;

@group(2) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(2) @binding(1)
var s_diffuse: sampler;

@group(3)@binding(0)
var<uniform> base_color_factors: vec4<f32>;


@vertex
fn vs_main(obj: VertexInput, instance: InstanceInput) -> VertexOutput {
    let obj_matrix = mat4x4<f32>(
        instance.obj_matrix_0,
        instance.obj_matrix_1,
        instance.obj_matrix_2,
        instance.obj_matrix_3,
    );

	let global_t_matrix = global_transforms.transforms[instance.model_index];
    var out: VertexOutput;
    out.clip_position = camera_uniform.transform * global_t_matrix * obj_matrix  * vec4<f32>(obj.position, 1.0);
	out.tex_coords = obj.tex_coords;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let colors: vec4<f32> =  textureSample(t_diffuse, s_diffuse, in.tex_coords);
	return vec4<f32>(colors[0] * base_color_factors[0], colors[1] * base_color_factors[1], colors[2] * base_color_factors[2], colors[3] * base_color_factors[3]);
}
