use wgpu::VertexBufferLayout;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
    pub joints: [u8; 4],
    pub weights: [u8; 4],
    pub base_color_index: u32,
}

pub trait Vertex {
    fn desc() -> VertexBufferLayout<'static>;
}
const ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
    0 => Float32x3,
    1 => Float32x3,
    2 => Float32x2,
    3 => Uint8x4,
    4 => Unorm8x4,
    5 => Uint32,
];
impl Vertex for ModelVertex {
    fn desc() -> VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}
