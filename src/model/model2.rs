use std::ops::Range;
use std::rc::Rc;

use crate::model::util::find_meshes;
use crate::scene::scene;

use super::util::{get_meshes, get_primitive_index_data, get_primitive_vertex_data, GltfErrors};
use super::vertex::ModelVertex;
use cgmath::{Matrix4, SquareMatrix};
use gltf::accessor::DataType;
use gltf::{Accessor, Mesh, Node, Primitive, Scene};
use wgpu::util::DeviceExt;
use wgpu::BufferSlice;

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
    pub models: Vec<GModel>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub local_transformation_buffer: wgpu::Buffer,
}

impl GScene {
    pub fn new<'a>(
        nodes: gltf::iter::Nodes<'a>,
        root_nodes_ids: Vec<usize>,
        buffer_data: Rc<Vec<u8>>,
        device: &wgpu::Device,
    ) -> Result<Self, GltfErrors> {
        let nodes: Vec<_> = nodes.collect();
        let mut models = Vec::with_capacity(root_nodes_ids.len());
        let mut scene_buffer_data: SceneBufferData = SceneBufferData::new(buffer_data.clone());
        let mut scene_mesh_data = SceneMeshData::new();

        for id in root_nodes_ids.iter() {
            // get a ref to the root node

            let root_node: &Node<'a> = &nodes[*id];

            // find mesh id's and instances associated with this root node
            scene_mesh_data = find_meshes(
                root_node,
                scene_mesh_data,
                cgmath::Matrix4::identity().into(),
            );
            assert_eq!(
                scene_mesh_data.mesh_ids.len(),
                scene_mesh_data.mesh_instances.len()
            );

            // given the meshes that are included in this model, generate GMeshes
            // this also appends the vertex and index buffer with the data for these meshes
            let meshes: Vec<GMesh> =
                get_meshes(&scene_mesh_data.mesh_ids, &nodes, &mut scene_buffer_data)?;

            models.push(GModel {
                byte_data: buffer_data.clone(),
                meshes,
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
            usage: wgpu::BufferUsages::INDEX,
        });

        let local_transformation_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Local transform buffer"),
                contents: bytemuck::cast_slice(&scene_mesh_data.transformation_matrices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        //   println!("transformations: ");
        //   for t in scene_mesh_data.transformation_matrices.iter() {
        //       println!("{:?}", t);
        //   }

        Ok(Self {
            models,
            vertex_buffer,
            index_buffer,
            local_transformation_buffer,
        })
    }
}

pub trait GDrawModel<'a> {
    fn draw_gmesh(&mut self, mesh: &'a GMesh);
    fn draw_gmesh_instanced(&mut self, mesh: &'a GMesh, scene: &GScene, instances: Range<u32>);
    fn draw_gmodel(&mut self, model: &'a GModel, scene: &GScene);
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
        for (idx, mesh) in model.meshes.iter().enumerate() {
            self.draw_gmesh_instanced(&mesh, scene, 0..model.mesh_instances[idx]);
        }
    }
}
