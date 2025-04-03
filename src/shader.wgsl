struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @builtin(vertex_index) index: u32,
}

struct InstanceInput {
  @location(3) obj_matrix_0: vec4<f32>,
  @location(4) obj_matrix_1: vec4<f32>,
  @location(5) obj_matrix_2: vec4<f32>,
  @location(6) obj_matrix_3: vec4<f32>,
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


@vertex
fn vs_main(obj: VertexInput, instance: InstanceInput) -> VertexOutput {
    let obj_matrix = mat4x4<f32>(
        instance.obj_matrix_0,
        instance.obj_matrix_1,
        instance.obj_matrix_2,
        instance.obj_matrix_3,
    );
    var out: VertexOutput;
    out.clip_position = camera_uniform.transform * obj_matrix * vec4<f32>(obj.position, 1.0);
    var color: vec3<f32> = vec3<f32>(0.5, 0.2, 0.7);
    out.color = color;
    return out;
}

@ fragment
fn fs_main(in: VertexOutput) -> @ location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
