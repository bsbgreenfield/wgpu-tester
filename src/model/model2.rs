use std::primitive;
use std::rc::Rc;

use crate::model::util::find_meshes;

use super::util::{get_meshes, GltfErrors};
use super::vertex::ModelVertex;
use gltf::accessor::DataType;
use gltf::buffer::View;
use gltf::{Accessor, Mesh, Primitive};
use wgpu::util::DeviceExt;

struct GPrimitive {
    vertices_offset: u32,
    indices_offset: u32,
    indices_length: u32,
}

impl GPrimitive {
    fn new(
        primitive: Primitive,
        vertex_data: &mut Vec<ModelVertex>,
        index_data: &mut Vec<u16>,
        byte_data: &Rc<Vec<u8>>,
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
            vertex_data,
            &byte_data,
        )?;

        let (indices_offset, indices_length) =
            get_primitive_index_data(&indices_accessor, index_data, &byte_data)?;

        Ok(Self {
            vertices_offset,
            indices_offset,
            indices_length,
        })
    }
}

fn get_primitive_index_data(
    indices_accessor: &Accessor,
    index_data: &mut Vec<u16>,
    byte_data: &Rc<Vec<u8>>,
) -> Result<(u32, u32), GltfErrors> {
    if indices_accessor.data_type() != DataType::U16 {
        return Err(GltfErrors::IndicesError(String::from(
            "the data type of this meshes indices is something other than u16!",
        )));
    }
    let indices_buffer_view = indices_accessor.view().ok_or(GltfErrors::NoView)?;
    let indices_bytes = &byte_data[indices_buffer_view.offset()..indices_buffer_view.length()];

    let indices_u16 = bytemuck::cast_slice(indices_bytes);
    let primitive_indices_offset = index_data.len();
    let primitive_indices_len = indices_u16.len();

    index_data.extend(indices_u16);

    Ok((
        primitive_indices_offset as u32,
        primitive_indices_len as u32,
    ))
}
/// *THIS FUNCTIONS MUTATES DATA*
/// expand the ModelVertex buffer to include the bytes specified by this primitive
/// by composing ModelVertex structs from bufferview data on the positions and normals
fn get_primitive_vertex_data(
    position_accessor: &Accessor,
    normals_accessor: &Accessor,
    vertex_data: &mut Vec<ModelVertex>,
    byte_data: &Rc<Vec<u8>>,
) -> Result<u32, GltfErrors> {
    if position_accessor.data_type() != DataType::F32
        && normals_accessor.data_type() != DataType::F32
    {
        return Err(GltfErrors::VericesError(String::from(
            "the data type of the vertices is something other than an F32!!",
        )));
    }
    let position_buffer_view = position_accessor.view().ok_or(GltfErrors::NoView)?;
    let normals_buffer_view = position_accessor.view().ok_or(GltfErrors::NoView)?;

    let position_bytes = &byte_data[position_buffer_view.offset()..position_buffer_view.length()];
    let normal_bytes = &byte_data[normals_buffer_view.offset()..normals_buffer_view.length()];

    let position_f32: &[f32] = bytemuck::cast_slice(position_bytes);
    let normals_f32: &[f32] = bytemuck::cast_slice(normal_bytes);

    assert_eq!(normals_f32.len(), position_f32.len()); // cannot zip if they arent the same size

    let vertex_vec: Vec<ModelVertex> = (0..(position_f32.len() / 3))
        .map(|i| ModelVertex {
            position: position_f32[i * 3..i * 3 + 3].try_into().unwrap(),
            normal: normals_f32[i * 3..i * 3 + 3].try_into().unwrap(),
        })
        .collect();

    let vertex_offset = vertex_data.len() as u32;

    vertex_data.extend(vertex_vec);

    Ok(vertex_offset)
}

#[derive(Debug, Clone, Copy)]
pub struct GMesh {
    pub index: u32,
    pub vertex_offset: u32,
    pub indices_offset: u32,
    pub indices_length: u32,
}
impl GMesh {
    pub fn newF(
        mesh: &Mesh,
        buffer_data: &Rc<Vec<u8>>,
        vertex_data: &mut Vec<ModelVertex>,
        index_data: &mut Vec<u16>,
    ) -> Result<Self, GltfErrors> {
        let mut g_primitives: Vec<GPrimitive> = Vec::with_capacity(mesh.primitives().len());
        for primitive in mesh.primitives() {
            // loop through the primitives and build out the vertex buffer and index buffer
            // side effects!! I know!!! Im sorry!!
            g_primitives.push(GPrimitive::new(
                primitive,
                vertex_data,
                index_data,
                buffer_data,
            )?);
        }

        Err(GltfErrors::NoIndices)
    }
}

pub struct GModel {
    pub byte_data: Rc<Vec<u8>>,
    pub meshes: Vec<GMesh>,
    pub mesh_instances: Vec<u32>,
}

pub struct GScene {
    models: Vec<GModel>,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}
impl GScene {
    pub fn new<'a>(
        mut nodes: gltf::iter::Nodes<'a>,
        root_nodes_ids: Vec<usize>,
        buffer_data: Rc<Vec<u8>>,
        device: &wgpu::Device,
    ) -> Result<Self, GltfErrors> {
        let mut vertex_data: Vec<ModelVertex> = Vec::new();
        let mut index_data: Vec<u16> = Vec::new();
        let meshes: Vec<GMesh> = get_meshes(
            nodes.clone(),
            &buffer_data,
            &mut vertex_data,
            &mut index_data,
        )?;

        let mut models = Vec::with_capacity(root_nodes_ids.len());

        for id in root_nodes_ids.iter() {
            let root_node = nodes.nth(*id).ok_or(GltfErrors::VericesError(String::from(
                "could not identify any root nodes",
            )))?;

            let mut mesh_ids = Vec::<u32>::new();
            let mut mesh_instances = Vec::<u32>::new();

            (mesh_ids, mesh_instances) = find_meshes(&root_node, mesh_ids, mesh_instances);

            assert_eq!(mesh_ids.len(), mesh_instances.len());

            let child_meshes: Vec<GMesh> = meshes
                .iter()
                .filter_map(|m| {
                    if mesh_ids.contains(&m.index) {
                        return Some(*m);
                    }
                    None
                })
                .collect();
            models.push(GModel {
                byte_data: buffer_data.clone(),
                meshes: child_meshes,
                mesh_instances,
            });
        }
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Index Buffer"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        Ok(Self {
            models,
            vertex_buffer,
            index_buffer,
        })
    }
}
