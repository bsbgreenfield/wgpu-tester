use std::rc::Rc;

use crate::model::util::find_meshes;

use super::util::{get_meshes, get_primitive_index_data, get_primitive_vertex_data, GltfErrors};
use super::vertex::ModelVertex;
use cgmath::{Matrix4, SquareMatrix};
use gltf::accessor::DataType;
use gltf::{Accessor, Mesh, Primitive, Scene};
use wgpu::util::DeviceExt;

#[derive(Debug, Clone, Copy)]
struct GPrimitive {
    vertices_offset: u32,
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

        let vertices_offset = get_primitive_vertex_data(
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
            indices_offset,
            indices_length,
        })
    }
}

#[derive(Debug, Clone)]
pub struct GMesh {
    pub index: u32,
    pub primitives: Vec<GPrimitive>,
}
impl GMesh {
    pub fn new(mesh: &Mesh, scene_buffer_data: &mut SceneBufferData) -> Result<Self, GltfErrors> {
        let mut g_primitives: Vec<GPrimitive> = Vec::with_capacity(mesh.primitives().len());
        for primitive in mesh.primitives() {
            // loop through the primitives and build out the vertex buffer and index buffer
            // side effects!! I know!!! Im sorry!!
            g_primitives.push(GPrimitive::new(primitive, scene_buffer_data)?);
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

pub struct SceneBufferData {
    main_buffer_data: Rc<Vec<u8>>,
    vertex_buf: Vec<ModelVertex>,
    index_buf: Vec<u16>,
}
impl SceneBufferData {
    fn new(main_buffer_data: Rc<Vec<u8>>) -> Self {
        Self {
            main_buffer_data,
            vertex_buf: Vec::new(),
            index_buf: Vec::new(),
        }
    }
}

pub struct SceneMeshData {
    pub mesh_ids: Vec<u32>,
    pub mesh_instances: Vec<u32>,
    pub transformation_matrices: Vec<[[f32; 4]; 4]>,
}
impl SceneMeshData {
    fn new() -> Self {
        Self {
            mesh_ids: Vec::new(),
            mesh_instances: Vec::new(),
            transformation_matrices: Vec::new(),
        }
    }
}
pub struct GScene {
    models: Vec<GModel>,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    local_transformation_buffer: wgpu::Buffer,
}

impl GScene {
    pub fn new<'a>(
        mut nodes: gltf::iter::Nodes<'a>,
        root_nodes_ids: Vec<usize>,
        buffer_data: Rc<Vec<u8>>,
        device: &wgpu::Device,
    ) -> Result<Self, GltfErrors> {
        let mut models = Vec::with_capacity(root_nodes_ids.len());
        let mut scene_buffer_data: SceneBufferData = SceneBufferData::new(buffer_data.clone());
        let mut scene_mesh_data = SceneMeshData::new();
        println!("hello {:?}", root_nodes_ids);
        // get a ref to the root node
        for id in root_nodes_ids.iter() {
            let root_node = nodes.nth(*id).ok_or(GltfErrors::VericesError(String::from(
                "could not identify any root nodes",
            )))?;

            // get a list of meshes by index and frequency, as well as a vec of
            // translation_matrices: one per mesh instance
            scene_mesh_data = find_meshes(
                &root_node,
                scene_mesh_data,
                cgmath::Matrix4::identity().into(),
            );
            assert_eq!(
                scene_mesh_data.mesh_ids.len(),
                scene_mesh_data.mesh_instances.len()
            );

            let meshes: Vec<GMesh> = get_meshes(nodes.clone(), &mut scene_buffer_data)?;

            let child_meshes: Vec<GMesh> = meshes
                .iter()
                .filter_map(|m| {
                    if scene_mesh_data.mesh_ids.contains(&m.index) {
                        return Some(m.clone());
                    }
                    None
                })
                .collect();
            models.push(GModel {
                byte_data: buffer_data.clone(),
                meshes: child_meshes,
                mesh_instances: scene_mesh_data.mesh_instances.clone(),
            });
        }
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Vertex Buffer"),
            contents: bytemuck::cast_slice(&scene_buffer_data.vertex_buf),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Index Buffer"),
            contents: bytemuck::cast_slice(&scene_buffer_data.index_buf),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        let local_transformation_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Local transform buffer"),
                contents: bytemuck::cast_slice(&scene_mesh_data.transformation_matrices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        println!("transformations: ");
        for t in scene_mesh_data.transformation_matrices.iter() {
            println!("{:?}", t);
        }

        Ok(Self {
            models,
            vertex_buffer,
            index_buffer,
            local_transformation_buffer,
        })
    }
}
