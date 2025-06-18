use super::util::GltfErrors;
use crate::model::animation::AnimationNode;
use crate::model::vertex::ModelVertex;
use crate::model::{animation::Animation, primitive::GPrimitive};
use crate::scene::scene::GScene;
use gltf::Mesh;
use std::ops::{self, Range};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccessorDataType {
    Vec3F32,
    U16,
}

// Maybe this entire folder should be moved inside of scene
// its annoying that these three functions are left as pub just
// so that scene can access them, but I may want to work with
// modesls independently later
pub struct GModel {
    pub animation_nodes: Option<Vec<AnimationNode>>,
    meshes: Vec<GMesh>,
    pub mesh_instances: Vec<u32>,
}

impl GModel {
    pub(super) fn new(
        animation_nodes: Option<Vec<AnimationNode>>,
        meshes: Vec<GMesh>,
        mesh_instances: Vec<u32>,
    ) -> Self {
        Self {
            animation_nodes,
            meshes,
            mesh_instances,
        }
    }

    pub fn get_model_vertex_data(
        &mut self,
        main_buffer_data: &Vec<u8>,
        buffer_offset_val: &mut u32,
    ) -> Vec<ModelVertex> {
        let mut vertex_buffer_data = Vec::<ModelVertex>::new();
        for mesh in self.meshes.iter_mut() {
            for primitive in mesh.primitives.iter_mut() {
                let primitive_vertex_data = primitive.get_vertex_data(main_buffer_data);
                primitive.initialized_vertex_offset_len =
                    Some((*buffer_offset_val, primitive_vertex_data.len() as u32));
                *buffer_offset_val += primitive_vertex_data.len() as u32;
                vertex_buffer_data.extend(primitive_vertex_data);
            }
        }
        vertex_buffer_data
    }

    pub fn build_range_vec(&self, range_vec: &mut Vec<std::ops::Range<usize>>) {
        for mesh in self.meshes.iter() {
            for primitive in mesh.primitives.iter() {
                let primitive_range = primitive.indices_offset as usize
                    ..(primitive.indices_offset + primitive.indices_length) as usize;
                crate::model::range_splicer::define_index_ranges(range_vec, &primitive_range);
            }
        }
    }

    pub fn get_model_index_data(
        main_buffer_data: &Vec<u8>,
        range_vec: &Vec<std::ops::Range<usize>>,
    ) -> Vec<u16> {
        GPrimitive::get_index_data(main_buffer_data, &range_vec)
    }
    pub fn set_model_primitive_offsets(&mut self, range_vec: &Vec<std::ops::Range<usize>>) {
        for mesh in self.meshes.iter_mut() {
            for primitive in mesh.primitives.iter_mut() {
                primitive
                    .set_primitive_offset(&range_vec)
                    .expect("set primitive indices offset");
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct GMesh {
    primitives: Vec<GPrimitive>,
}
impl GMesh {
    pub(super) fn new(mesh: &Mesh) -> Result<Self, GltfErrors> {
        let mut g_primitives: Vec<GPrimitive> = Vec::with_capacity(mesh.primitives().len());
        for primitive in mesh.primitives() {
            let p = GPrimitive::new(primitive)?;
            g_primitives.push(p);
        }
        Ok(Self {
            primitives: g_primitives,
        })
    }
}

pub trait GDrawModel<'a> {
    fn draw_scene(&mut self, scene: &'a GScene);
}
trait RenderPassUtil<'a> {
    fn draw_gmesh_instanced(&mut self, mesh: &'a GMesh, instances: Range<u32>);
    fn draw_gmodel(&mut self, model: &'a GModel, instances: u32, num_mesh_instances: u32) -> u32;
}

impl<'a> RenderPassUtil<'a> for wgpu::RenderPass<'a> {
    fn draw_gmesh_instanced(&mut self, mesh: &'a GMesh, instances: Range<u32>) {
        for primitive in mesh.primitives.iter() {
            let (indices_offset, indices_length) = primitive.initialized_index_offset_len.unwrap();
            let (vertices_offset, _) = primitive.initialized_vertex_offset_len.unwrap();
            self.draw_indexed(
                indices_offset..(indices_length + indices_offset),
                vertices_offset as i32,
                instances.clone(),
            );
        }
    }
    fn draw_gmodel(
        &mut self,
        model: &'a GModel,
        model_offset: u32,
        model_instance_count: u32,
    ) -> u32 {
        let mut mesh_offset = model_offset;
        for (idx, mesh) in model.meshes.iter().enumerate() {
            let num_mesh_instances = model.mesh_instances[idx] * model_instance_count;
            self.draw_gmesh_instanced(mesh, mesh_offset..mesh_offset + num_mesh_instances);
            mesh_offset += num_mesh_instances;
        }
        mesh_offset
    }
}

impl<'a, 'b> GDrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_scene(&mut self, scene: &'b GScene) {
        let mut offset: u32 = 0;
        for (idx, model) in scene.models.iter().enumerate() {
            offset += self.draw_gmodel(model, offset, scene.get_model_instances()[idx] as u32);
        }
    }
}
pub trait ToRawMatrix {
    fn as_raw_matrix(&self) -> [[f32; 4]; 4];
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct LocalTransform {
    pub transform_matrix: [[f32; 4]; 4],
    pub model_index: u32,
}

impl LocalTransform {
    pub const fn raw_matrix_from_vectors(
        x_vector: [f32; 4],
        y_vector: [f32; 4],
        z_vector: [f32; 4],
        w_vector: [f32; 4],
    ) -> [[f32; 4]; 4] {
        [x_vector, y_vector, z_vector, w_vector]
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
            array_stride: (mem::size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress)
                + (mem::size_of::<u32>() as wgpu::BufferAddress),
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
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}
impl ToRawMatrix for LocalTransform {
    fn as_raw_matrix(&self) -> [[f32; 4]; 4] {
        self.transform_matrix.into()
    }
}

pub struct GlobalTransform {
    pub transform_matrix: cgmath::Matrix4<f32>,
}
impl ops::Mul<[[f32; 4]; 4]> for GlobalTransform {
    type Output = [[f32; 4]; 4];
    fn mul(self, rhs: [[f32; 4]; 4]) -> Self::Output {
        let a = self.transform_matrix * cgmath::Matrix4::<f32>::from(rhs);
        a.into()
    }
}
