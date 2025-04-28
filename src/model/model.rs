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
    fn draw_gmodel_instanced(&mut self, model: GModel) {
        for (idx, mesh) in model.meshes.iter().enumerate() {
            // the number stored at this index of mesh instances is the total number of instances
            // of meshes that need to be drawn
            let mesh_instances = model.mesh_instances[idx];
            // self.draw_gmesh_instanced(*mesh, mesh_instances);
        }
    }
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
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

    fn draw_model(&mut self, model: &'b Model) {
        self.draw_model_instanced(model, 0..1);
    }

    fn draw_model_instanced(&mut self, model: &'b Model, instances: Range<u32>) {
        for mesh in &model.meshes {
            self.draw_mesh_instanced(mesh, instances.clone());
        }
    }
}
