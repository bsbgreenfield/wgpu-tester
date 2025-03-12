struct VertexInput {
  @location(0) position: vec3<f32>,
}

struct InstanceInput {
  @location(3) obj_matrix_0: vec4<f32>,
  @location(4) obj_matrix_1: vec4<f32>,
  @location(5) obj_matrix_2: vec4<f32>,
  @location(6) obj_matrix_3: vec4<f32>,
}

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
}

struct CTRUniform {
  transform: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> ctr_uniform: CTRUniform; 

@vertex
fn vs_main(obj: VertexInput, instance: InstanceInput) -> VertexOutput {
    let obj_matrix = mat4x4<f32>(
        instance.obj_matrix_0,
        instance.obj_matrix_1,
        instance.obj_matrix_2,
        instance.obj_matrix_3,
    );
    var out: VertexOutput;
    out.clip_position = ctr_uniform.transform * obj_matrix * vec4<f32>(obj.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.9, 0.1, 0.1, 1.0);
}
