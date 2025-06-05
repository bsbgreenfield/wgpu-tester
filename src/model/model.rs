use super::util::InitializationError;
use super::util::{get_primitive_index_data, get_primitive_vertex_data, GltfErrors};
use crate::model::util::get_primitive_data;
use crate::model::vertex::ModelVertex;
use crate::scene::scene::GScene;
use crate::scene::scene::SceneBufferData;
use gltf::accessor::DataType;
use gltf::json::serialize::to_vec;
use gltf::{Mesh, Primitive};
use std::ops::{self, Range};
use std::primitive;
use std::rc::Rc;

#[derive(Debug, Clone, Copy)]
pub struct GPrimitive2 {
    position_offset: u32,
    position_length: u32,
    normal_offset: u32,
    normal_length: u32,
    pub indices_offset: u32,
    pub indices_length: u32,
    pub initialized_vertex_offset_len: Option<(u32, u32)>,
    pub initialized_index_offset_len: Option<(u32, u32)>,
}

impl GPrimitive2 {
    fn is_initialized(&self) -> bool {
        if self.initialized_index_offset_len.is_some()
            && self.initialized_vertex_offset_len.is_some()
        {
            return true;
        }
        false
    }

    fn new(primitive: Primitive) -> Result<Self, GltfErrors> {
        let (_, position_accessor) = primitive
            .attributes()
            .find(|a| a.0 == gltf::Semantic::Positions)
            .unwrap();

        let (_, normals_accessor) = primitive
            .attributes()
            .find(|a| a.0 == gltf::Semantic::Normals)
            .unwrap();

        let indices_accessor = primitive.indices().unwrap();

        let (position_offset, position_length) =
            get_primitive_data(&position_accessor, DataType::F32)?;
        let (normal_offset, normal_length) = get_primitive_data(&normals_accessor, DataType::F32)?;
        let (indices_offset, indices_length) =
            get_primitive_data(&indices_accessor, DataType::U16)?;
        Ok(Self {
            position_offset,
            position_length,
            normal_offset,
            normal_length,
            indices_offset,
            indices_length,
            initialized_vertex_offset_len: None,
            initialized_index_offset_len: None,
        })
    }
    pub fn get_vertex_data(&self, main_buffer_data: &Vec<u8>) -> Vec<ModelVertex> {
        let position_bytes = &main_buffer_data
            [self.position_offset as usize..(self.position_offset + self.position_length) as usize];
        let normal_bytes = &main_buffer_data
            [self.normal_offset as usize..(self.normal_offset + self.normal_length) as usize];
        let position_f32: &[f32] = bytemuck::cast_slice(position_bytes);
        let normals_f32: &[f32] = bytemuck::cast_slice(normal_bytes);
        assert_eq!(normals_f32.len(), position_f32.len());
        let vertex_vec: Vec<ModelVertex> = (0..(position_f32.len() / 3))
            .map(|i| ModelVertex {
                position: position_f32[i * 3..i * 3 + 3].try_into().unwrap(),
                normal: normals_f32[i * 3..i * 3 + 3].try_into().unwrap(),
            })
            .collect();
        vertex_vec
    }
    pub fn get_index_data(
        main_buffer_data: &Vec<u8>,
        indices_ranges: &Vec<std::ops::Range<usize>>,
    ) -> Vec<u16> {
        let mut index_vec: Vec<u16> = Vec::new();
        for range in indices_ranges.iter() {
            let indices_bytes: &[u8] = &main_buffer_data[range.start..range.end];
            let indices_u16: &[u16] = bytemuck::cast_slice::<u8, u16>(indices_bytes);
            index_vec.extend(indices_u16.to_vec());
        }
        index_vec
    }
    pub fn set_primitive_offset(
        &mut self,
        index_ranges: &Vec<Range<usize>>,
    ) -> Result<(), InitializationError> {
        // upon creation, this primitive will have stored its offset and length relative to the
        // main byte buffer. Also at this stage, scene_buffer_data has stored a list of ranges that
        // need to be composed into the final index buffer. We need to translate the indices
        // relative to the main buffer to indices relative to a buffer which would contain only the
        // ranges specified in scene_buffer_data.
        let mut relative_buffer_offset = 0;
        for index_range in index_ranges.iter() {
            if self.indices_offset as usize > index_range.end {
                relative_buffer_offset += index_range.len();
            } else {
                relative_buffer_offset += self.indices_offset as usize - index_range.start;
                // paranoid?
                if (self.indices_offset + self.indices_length) as usize > index_range.end {
                    return Err(InitializationError::SceneInitializationError);
                }
                break;
            }
        }

        self.initialized_index_offset_len =
            Some(((relative_buffer_offset / 2) as u32, self.indices_length / 2));
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct GPrimitive {
    vertices_offset: u32,
    vertices_length: u32,
    indices_offset: u32,
    indices_length: u32,
}

impl GPrimitive {
    fn new(
        primitive: Primitive,
        scene_buffer_data: &mut SceneBufferData,
    ) -> Result<Self, GltfErrors> {
        let (_, position_accessor) = primitive
            .attributes()
            .find(|a| a.0 == gltf::Semantic::Positions)
            .unwrap();

        let (_, normals_accessor) = primitive
            .attributes()
            .find(|a| a.0 == gltf::Semantic::Normals)
            .unwrap();

        let indices_accessor = primitive.indices().unwrap();

        let (vertices_offset, vertices_length) = get_primitive_vertex_data(
            &position_accessor,
            &normals_accessor,
            &mut scene_buffer_data.vertex_buf,
            &scene_buffer_data.main_buffer_data,
        )?;

        let (indices_offset, indices_length) =
            get_primitive_index_data(&indices_accessor, scene_buffer_data)?;

        Ok(Self {
            vertices_offset,
            vertices_length,
            indices_offset,
            indices_length,
        })
    }

    pub(super) fn set_primitive_offset(
        &mut self,
        index_ranges: &Vec<Range<usize>>,
    ) -> Result<(), InitializationError> {
        // upon creation, this primitive will have stored its offset and length relative to the
        // main byte buffer. Also at this stage, scene_buffer_data has stored a list of ranges that
        // need to be composed into the final index buffer. We need to translate the indices
        // relative to the main buffer to indices relative to a buffer which would contain only the
        // ranges specified in scene_buffer_data.
        let mut relative_buffer_offset = 0;
        for index_range in index_ranges.iter() {
            if self.indices_offset as usize > index_range.end {
                relative_buffer_offset += index_range.len();
            } else {
                relative_buffer_offset += self.indices_offset as usize - index_range.start;
                // paranoid?
                if (self.indices_offset + self.indices_length) as usize > index_range.end {
                    return Err(InitializationError::SceneInitializationError);
                }
                break;
            }
        }

        self.indices_offset = (relative_buffer_offset / 2) as u32;
        self.indices_length = self.indices_length / 2;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct GMesh {
    pub index: u32,
    primitives: Vec<GPrimitive>,
}
#[derive(Debug, Clone)]
pub struct GMesh2 {
    pub index: u32,
    pub primitives: Vec<GPrimitive2>,
}
impl GMesh2 {
    pub fn new(mesh: &Mesh) -> Result<Self, GltfErrors> {
        let mut g_primitives: Vec<GPrimitive2> = Vec::with_capacity(mesh.primitives().len());
        for primitive in mesh.primitives() {
            let p = GPrimitive2::new(primitive)?;
            g_primitives.push(p);
        }
        Ok(Self {
            index: mesh.index() as u32,
            primitives: g_primitives,
        })
    }
}
impl GMesh {
    pub fn new(mesh: &Mesh, scene_buffer_data: &mut SceneBufferData) -> Result<Self, GltfErrors> {
        let mut g_primitives: Vec<GPrimitive> = Vec::with_capacity(mesh.primitives().len());
        for primitive in mesh.primitives() {
            // loop through the primitives and build out the vertex buffer and index buffer
            // side effects!
            let p = GPrimitive::new(primitive, scene_buffer_data)?;
            g_primitives.push(p);
        }

        Ok(Self {
            index: mesh.index() as u32,
            primitives: g_primitives,
        })
    }

    pub fn get_total_vertex_len(&self) -> u32 {
        let mut vertex_count = 0;
        for primitive in self.primitives.iter() {
            vertex_count += primitive.vertices_length;
        }
        return vertex_count;
    }
    pub fn get_total_index_len(&self) -> u32 {
        let mut index_count = 0;
        for primitive in self.primitives.iter() {
            index_count += primitive.indices_length;
        }
        return index_count;
    }
    /// this function increases the offset of all primitive vertex and index data in the mesh.
    /// This is needed for gltf file merging, as the scene to which this mesh belongs is being
    /// appended to a list of vertices and indices
    pub fn update_primitive_offsets_during_merge(&mut self, vertex_count: u32, index_count: u32) {
        for primitive in self.primitives.iter_mut() {
            primitive.vertices_offset += vertex_count;
            primitive.indices_offset += index_count;
        }
    }

    pub fn set_primitive_offsets(&mut self, index_ranges: &Vec<Range<usize>>) {
        for primitive in self.primitives.iter_mut() {
            let result = primitive.set_primitive_offset(index_ranges);
            if result.is_err() {
                panic!("error initializing primitives");
            }
        }
    }
}

pub struct GModel {
    pub byte_data: Rc<Vec<u8>>,
    pub meshes: Vec<GMesh>,
    pub mesh_instances: Vec<u32>,
}

pub trait GDrawModel<'a> {
    fn draw_gmesh(&mut self, mesh: &'a GMesh);
    fn draw_gmesh_instanced(&mut self, mesh: &'a GMesh, instances: Range<u32>);
    fn draw_gmodel(&mut self, model: &'a GModel, instances: u32, num_mesh_instances: u32) -> u32;
    fn draw_scene(&mut self, scene: &'a GScene);
}

impl<'a, 'b> GDrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_gmesh(&mut self, mesh: &'b GMesh) {}
    fn draw_gmesh_instanced(&mut self, mesh: &'b GMesh, instances: Range<u32>) {
        for primitive in mesh.primitives.iter() {
            self.draw_indexed(
                primitive.indices_offset..(primitive.indices_length + primitive.indices_offset),
                primitive.vertices_offset as i32,
                instances.clone(),
            );
        }
    }
    fn draw_gmodel(
        &mut self,
        model: &'b GModel,
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
