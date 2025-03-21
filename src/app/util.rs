use wgpu::util::DeviceExt;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

pub fn create_vertex_bind_group<B>(
    buffer_data: B,
    device: &wgpu::Device,
    label: Option<&str>,
    buf_label: Option<&str>,
    buffer_usage: wgpu::BufferUsages,
    binding_type: wgpu::BindingType,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup)
where
    B: Copy + Clone + bytemuck::Zeroable + bytemuck::Pod,
{
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: buf_label,
        contents: bytemuck::cast_slice(&[buffer_data]),
        usage: buffer_usage,
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: binding_type,
            count: None,
        }],
        label,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
        label,
    });

    (bind_group_layout, bind_group)
}
