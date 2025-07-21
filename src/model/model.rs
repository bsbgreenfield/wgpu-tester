use super::util::GltfErrors;
use crate::model::vertex::ModelVertex;
use crate::model::{animation::animation_node::AnimationNode, primitive::GPrimitive};
use crate::scene::scene::{GScene, PrimitiveData};
use gltf::Mesh;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{self, Range};
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccessorDataType {
    U8,
    Vec3F32,
    U16,
}

pub struct MeshAnimationData {
    pub mesh_animations: Vec<usize>,
    pub node_to_lt_index: HashMap<usize, usize>,
}
pub struct JointAnimationData {
    pub joint_to_joint_index: HashMap<usize, usize>,
    pub joint_count: usize,
    pub joint_indices: Vec<usize>,
}

pub struct ModelAnimationData {
    pub animation_node: Rc<AnimationNode>,
    pub model_index: usize,
    pub animation_count: usize,
    pub mesh_animation_data: MeshAnimationData,
    pub joint_animation_data: JointAnimationData,
    pub is_skeletal: bool,
}

impl Debug for ModelAnimationData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Animation Data")
            .field("model_index", &self.model_index)
            .field("animation_count", &self.animation_count)
            .field(
                "node_to_lt_index",
                &self.mesh_animation_data.node_to_lt_index,
            )
            .field(
                "joint_to_joint_index",
                &self.joint_animation_data.joint_to_joint_index,
            )
            .finish()
    }
}

// Maybe this entire folder should be moved inside of scene
// its annoying that these three functions are left as pub just
// so that scene can access them, but I may want to work with
// modesls independently later
pub struct GModel {
    pub model_id: usize,
    meshes: Vec<GMesh>,
    pub mesh_instances: Vec<u32>,
    pub animation_data: Option<ModelAnimationData>,
}

impl GModel {
    pub fn print_model(&self) {
        println!("Model: ");
        println!("       Meshes: {:?}", self.meshes);
        println!("       Animations: {:?}", self.animation_data);
        println!("---------------------------------------------");
    }
    pub(super) fn new(
        model_id: usize,
        meshes: Vec<GMesh>,
        mesh_instances: Vec<u32>,
        animation_data: Option<ModelAnimationData>,
    ) -> Self {
        Self {
            model_id,
            meshes,
            mesh_instances,
            animation_data,
        }
    }

    pub fn get_model_vertex_data(
        &mut self,
        primitive_data: &Vec<PrimitiveData>,
        buffer_offset_val: &mut u32,
    ) -> Vec<ModelVertex> {
        let mut vertex_buffer_data = Vec::<ModelVertex>::new();
        // for each piece of data associated with a primitive in this model
        // add data to the vertex buffer.
        for mesh in self.meshes.iter_mut() {
            let mesh_primitive_data_vec = primitive_data
                .iter()
                .filter(|primitive_data| primitive_data.mesh_id == mesh.mesh_id);
            for (primitive, data) in mesh.primitives.iter_mut().zip(mesh_primitive_data_vec) {
                let primitive_vertex_data = data.get_vertex_data();
                primitive.initialized_vertex_offset_len =
                    Some((*buffer_offset_val, primitive_vertex_data.len() as u32));
                *buffer_offset_val += primitive_vertex_data.len() as u32;
                vertex_buffer_data.extend(primitive_vertex_data);
            }
        }
        vertex_buffer_data
    }

    pub fn build_range_vec(
        &self,
        range_vec: &mut Vec<std::ops::Range<usize>>,
        primitive_data: &Vec<PrimitiveData>,
    ) {
        for data in primitive_data.iter() {
            let offset = data.indices_offset;
            let len = data.indices_len;
            let primitive_range = offset..offset + len;
            crate::model::range_splicer::define_index_ranges(range_vec, &primitive_range);
        }
    }

    pub fn get_model_index_data(
        main_buffer_data: &Vec<u8>,
        range_vec: &Vec<std::ops::Range<usize>>,
    ) -> Vec<u16> {
        GPrimitive::get_index_data(main_buffer_data, &range_vec)
    }
    pub fn set_model_primitive_offsets(
        &mut self,
        range_vec: &Vec<std::ops::Range<usize>>,
        primitive_data: &Vec<PrimitiveData>,
    ) {
        for mesh in self.meshes.iter_mut() {
            let mesh_primitive_data = primitive_data
                .iter()
                .filter(|data| data.mesh_id == mesh.mesh_id);
            for (primitive, data) in mesh.primitives.iter_mut().zip(mesh_primitive_data) {
                primitive
                    .set_relative_indices_offset(data, &range_vec)
                    .expect("set primitive indices offset");
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct GMesh {
    pub mesh_id: usize,
    primitives: Vec<GPrimitive>,
}

impl GMesh {
    pub fn get_primitive_data(
        mesh: &Mesh,
        buffer_offsets: &Vec<u64>,
        binary_data: &Vec<u8>,
    ) -> Result<Vec<PrimitiveData>, GltfErrors> {
        let mut primitive_data: Vec<PrimitiveData> = Vec::with_capacity(mesh.primitives().len());
        for primitive in mesh.primitives() {
            let data =
                PrimitiveData::from_data(mesh.index(), primitive, buffer_offsets, binary_data)?;
            primitive_data.push(data);
        }
        Ok(primitive_data)
    }
    pub(super) fn new(mesh: &Mesh) -> Result<Self, GltfErrors> {
        let mut g_primitives: Vec<GPrimitive> = Vec::with_capacity(mesh.primitives().len());
        for _ in mesh.primitives() {
            let p = GPrimitive::new();
            g_primitives.push(p);
        }
        Ok(Self {
            mesh_id: mesh.index(),
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
            let (vertices_offset, vertices_length) =
                primitive.initialized_vertex_offset_len.unwrap();
            if indices_length > 0 {
                self.draw_indexed(
                    indices_offset..(indices_length + indices_offset),
                    vertices_offset as i32,
                    instances.clone(),
                );
            } else {
                self.draw(
                    vertices_offset..(vertices_offset + vertices_length),
                    instances.clone(),
                );
            }
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
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
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
