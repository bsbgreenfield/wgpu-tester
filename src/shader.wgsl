struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) joints: vec4<u32>,
  @location(3) weights: vec4<u32>,
}

struct InstanceInput {
  @location(5) obj_matrix_0: vec4<f32>,
  @location(6) obj_matrix_1: vec4<f32>,
  @location(7) obj_matrix_2: vec4<f32>,
  @location(8) obj_matrix_3: vec4<f32>,
  @location(9) model_index: u32,
}


struct GlobalTransforms{
  transforms: array<mat4x4<f32>>,
}
struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) color: vec3<f32>,
}

struct CameraUniform {
  transform: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera_uniform: CameraUniform;

@group(1) @binding(1)
var<storage, read> global_transforms: GlobalTransforms;

@group(2) @binding(2)
var<storage, read> joint_transforms: array<mat4x4<f32>>;

struct SkinMatrix {
   skin_matrix_0: vec4<f32>,
   skin_matrix_1: vec4<f32>,
   skin_matrix_2: vec4<f32>,
   skin_matrix_3: vec4<f32>,
}

fn apply_bone_transform(obj: VertexInput) -> vec4<f32> {
	var result = vec4<f32>(0.0, 0.0, 0.0, 0.0);
	for (var i = 0; i < 4; i ++ ){
		let joint_index = obj.joints[i];
        let weight = f32(obj.weights[i]) / 255.0;

        let transform = joint_transforms[joint_index];
        let transformed_position = (transform * weight) * vec4<f32>(obj.position, 1.0) ;

        result += transformed_position;
	}
	     
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

	var position: vec4<f32> = apply_bone_transform(obj);
    out.clip_position = camera_uniform.transform * global_t_matrix * obj_matrix  * vec4<f32>(obj.position, 1.0);

    var color: vec3<f32> = vec3<f32>(0.5, 0.2, 0.7);
	out.color = color; 
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
