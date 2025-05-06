use super::util::{get_primitive_index_data, get_primitive_vertex_data, GltfErrors};
use crate::scene::scene2::GScene;
use crate::scene::scene2::SceneBufferData;
use gltf::{Mesh, Primitive};
use std::ops::{self, Range};
use std::rc::Rc;

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

        let (indices_offset, indices_length) = get_primitive_index_data(
            &indices_accessor,
            &mut scene_buffer_data.index_buf,
            &scene_buffer_data.main_buffer_data,
        )?;

        Ok(Self {
            vertices_offset,
            vertices_length,
            indices_offset,
            indices_length,
        })
    }
}

#[derive(Debug, Clone)]
pub struct GMesh {
    pub index: u32,
    primitives: Vec<GPrimitive>,
}
impl GMesh {
    pub fn new(mesh: &Mesh, scene_buffer_data: &mut SceneBufferData) -> Result<Self, GltfErrors> {
        let mut g_primitives: Vec<GPrimitive> = Vec::with_capacity(mesh.primitives().len());
        for primitive in mesh.primitives() {
            // loop through the primitives and build out the vertex buffer and index buffer
            // side effects!! I know!!! Im sorry!!
            let p = GPrimitive::new(primitive, scene_buffer_data)?;
            g_primitives.push(p);
        }

        Ok(Self {
            index: mesh.index() as u32,
            primitives: g_primitives,
        })
    }
}

pub struct GModel {
    pub byte_data: Rc<Vec<u8>>,
    pub meshes: Vec<GMesh>,
    pub mesh_instances: Vec<u32>,
}

pub trait GDrawModel<'a> {
    fn draw_gmesh(&mut self, mesh: &'a GMesh);
    fn draw_gmesh_instanced(&mut self, mesh: &'a GMesh, scene: &GScene, instances: Range<u32>);
    fn draw_gmodel(&mut self, model: &'a GModel, scene: &GScene);
    fn draw_scene(&mut self, scene: &'a GScene);
}

impl<'a, 'b> GDrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_gmesh(&mut self, mesh: &'b GMesh) {}
    fn draw_gmesh_instanced(&mut self, mesh: &'b GMesh, scene: &GScene, instances: Range<u32>) {
        for primitive in mesh.primitives.iter() {
            let r: Range<u64> = Range {
                start: primitive.vertices_offset as u64,
                end: (primitive.vertices_length + primitive.vertices_offset) as u64,
            };

            let ri: Range<u64> = Range {
                start: (primitive.indices_offset as u64),
                end: ((primitive.indices_length * 2) as u64 + primitive.indices_offset as u64),
            };
            self.set_vertex_buffer(0, scene.vertex_buffer.slice(r));
            self.set_index_buffer(scene.index_buffer.slice(ri), wgpu::IndexFormat::Uint16);
            self.draw_indexed(0..primitive.indices_length, 0, instances.clone());
        }
    }
    fn draw_gmodel(&mut self, model: &'b GModel, scene: &GScene) {
        let mut offset: u32 = 0;
        for (idx, mesh) in model.meshes.iter().enumerate() {
            self.draw_gmesh_instanced(&mesh, scene, offset..offset + model.mesh_instances[idx]);
            offset += model.mesh_instances[idx];
        }
    }

    fn draw_scene(&mut self, scene: &'b GScene) {
        for model in scene.models.iter() {
            self.draw_gmodel(model, scene);
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
