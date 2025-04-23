use wgpu::util::DeviceExt;

use std::ops::{self, Range};

use crate::model::vertex::ModelVertex;

use super::model2::{GMesh, GModel};

#[derive(Clone)]
pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: usize,
}

#[derive(Clone)]
pub struct Model {
    pub meshes: Vec<Mesh>,
}

impl Model {
    pub fn from_vertices(vertices: &[ModelVertex], indices: &[u32], device: &wgpu::Device) -> Self {
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

pub trait DrawModel<'a> {
    fn draw_mesh(&mut self, mesh: &'a Mesh);
    fn draw_mesh_instanced(&mut self, mesh: &'a Mesh, instances: Range<u32>);
    fn draw_model(&mut self, object: &'a Model);
    fn draw_model_instanced(&mut self, model: &'a Model, instances: Range<u32>);
    fn draw_gmesh_instanced(&mut self, mesh: GMesh, instances: u32);
    fn draw_gmodel_instanced(&mut self, model: GModel) {
        for (idx, mesh) in model.meshes.iter().enumerate() {
            // the number stored at this index of mesh instances is the total number of instances
            // of meshes that need to be drawn
            let mesh_instances = model.mesh_instances[idx];
            self.draw_gmesh_instanced(*mesh, mesh_instances);
        }
    }
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_gmesh_instanced(&mut self, mesh: GMesh, instances: u32) {
        let indices = mesh.indices_offset..(mesh.indices_offset + mesh.indices_length);
        self.draw_indexed(indices, mesh.vertex_offset as i32, 0..instances);
    }
    fn draw_mesh(&mut self, mesh: &'b Mesh) {
        self.draw_mesh_instanced(mesh, 0..1);
    }

    fn draw_mesh_instanced(&mut self, mesh: &'b Mesh, instances: Range<u32>) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.draw_indexed(0..mesh.num_elements as u32, 0, instances);
    }

    fn draw_model(&mut self, model: &'b Model) {
        self.draw_model_instanced(model, 0..1);
    }

    fn draw_model_instanced(&mut self, model: &'b Model, instances: Range<u32>) {
        for mesh in &model.meshes {
            self.draw_mesh_instanced(mesh, instances.clone());
        }
    }
}

pub trait ToRawMatrix {
    fn as_raw_matrix(&self) -> [[f32; 4]; 4];
}

#[derive(Debug, Clone, Copy)]
pub struct ObjectTransform {
    pub transform_matrix: cgmath::Matrix4<f32>,
}
impl ops::Mul<ObjectTransform> for ObjectTransform {
    type Output = ObjectTransform;
    fn mul(self, rhs: ObjectTransform) -> Self::Output {
        ObjectTransform {
            transform_matrix: self.transform_matrix * rhs.transform_matrix,
        }
    }
}

impl ObjectTransform {
    pub const fn raw_matrix_from_vectors(
        x_vector: [f32; 4],
        y_vector: [f32; 4],
        z_vector: [f32; 4],
        w_vector: [f32; 4],
    ) -> [[f32; 4]; 4] {
        [x_vector, y_vector, z_vector, w_vector]
    }

    pub fn from_raw_matrix(matrix: [[f32; 4]; 4]) -> Self {
        Self {
            transform_matrix: matrix.into(),
        }
    }

    pub const fn identity() -> [[f32; 4]; 4] {
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
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
impl ToRawMatrix for ObjectTransform {
    fn as_raw_matrix(&self) -> [[f32; 4]; 4] {
        self.transform_matrix.into()
    }
}
