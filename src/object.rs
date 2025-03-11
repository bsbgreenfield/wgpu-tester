use wgpu::util::DeviceExt;

use crate::vertex::Vertex;
use std::ops::Range;
pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: usize,
}

pub struct Object {
    pub meshes: Vec<Mesh>,
}

impl Object {
    pub fn from_vertices(vertices: &[Vertex], indices: &[u32], device: &wgpu::Device) -> Self {
        let mesh = Mesh {
            vertex_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            index_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            }),
            num_elements: indices.len(),
        };

        Self { meshes: vec![mesh] }
    }
}

pub trait DrawObject<'a> {
    fn draw_mesh(&mut self, mesh: &'a Mesh);
    fn draw_mesh_instanced(&mut self, mesh: &'a Mesh, instances: Range<u32>);
    fn draw_object(&mut self, object: &'a Object);
    fn draw_object_instanced(&mut self, object: &'a Object, instances: Range<u32>);
}

impl<'a, 'b> DrawObject<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(&mut self, mesh: &'b Mesh) {
        self.draw_mesh_instanced(mesh, 0..1);
    }

    fn draw_mesh_instanced(&mut self, mesh: &'b Mesh, instances: Range<u32>) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.draw_indexed(0..mesh.num_elements as u32, 0, instances);
    }

    fn draw_object(&mut self, object: &'b Object) {
        self.draw_object_instanced(object, 0..1);
    }

    fn draw_object_instanced(&mut self, object: &'b Object, instances: Range<u32>) {
        for mesh in &object.meshes {
            self.draw_mesh_instanced(mesh, instances.clone());
        }
    }
}

pub trait ToRawMatrix {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
    fn as_raw_matrix(&self) -> [[f32; 4]; 4];
}

pub struct ObjectTransform {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
}

impl ToRawMatrix for ObjectTransform {
    fn as_raw_matrix(&self) -> [[f32; 4]; 4] {
        (cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation))
            .into()
    }

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
