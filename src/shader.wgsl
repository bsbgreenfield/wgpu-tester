struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) tex_coords: vec2<f32>,
  @location(3) joints: vec4<u32>,
  @location(4) weights: vec4<f32>,
  @location(5) base_color_index: u32,
}

struct InstanceInput {
  @location(6) obj_matrix_0: vec4<f32>,
  @location(7) obj_matrix_1: vec4<f32>,
  @location(8) obj_matrix_2: vec4<f32>,
  @location(9) obj_matrix_3: vec4<f32>,
  @location(10) model_index: u32,
}


struct GlobalTransforms{
  transforms: array<mat4x4<f32>>,
}
struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(2) tex_coords: vec2<f32>,
  @location(5) base_color_index: u32,
}

struct CameraUniform {
  transform: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera_uniform: CameraUniform;
@group(0) @binding(1)
var<storage, read> base_colors: array<vec3<f32>>;

@group(1) @binding(1)
var<storage, read> global_transforms: GlobalTransforms;

@group(2) @binding(2)
var<storage, read> joint_transforms: array<mat4x4<f32>>;

@group(3) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(3) @binding(1)
var s_diffuse: sampler;


fn apply_bone_transform(joints: vec4<u32>, weights: vec4<f32>, position: vec3<f32>) -> vec4<f32> {
	let skin_mat: mat4x4<f32> = 
	                           weights[0] * joint_transforms[joints[0]] +
	                           weights[1] * joint_transforms[joints[1]] +
	                           weights[2] * joint_transforms[joints[2]] +
                               weights[3] * joint_transforms[joints[3]];
	let result: vec4<f32> = skin_mat * vec4<f32>(position, 1.0);
	return result;
} 

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
	let new_position: vec4<f32> = apply_bone_transform(obj.joints, obj.weights, obj.position);
    out.clip_position = camera_uniform.transform * global_t_matrix * obj_matrix * new_position;
	out.tex_coords = obj.tex_coords;
	out.base_color_index = obj.base_color_index;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let colors: vec4<f32> =  textureSample(t_diffuse, s_diffuse, in.tex_coords);
	let bc_factor = base_colors[in.base_color_index];
	return vec4<f32>(colors[0] * bc_factor[0], colors[1] * bc_factor[1], colors[2] * bc_factor[2], colors[3]);
}
