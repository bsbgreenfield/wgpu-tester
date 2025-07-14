use crate::model::model::GMesh;
use gltf::{
    accessor::{DataType, Dimensions},
    Accessor,
};
use std::fmt::Debug;

#[derive(Debug)]
pub enum GltfErrors {
    NoIndices,
    NoView,
    NoPrimitive,
    IndicesError(String),
    VericesError(String),
    NormalsError(String),
}

#[derive(Debug)]
pub enum InitializationError {
    InstanceDataInitializationError(Box<String>),
    SceneMergeError(Box<String>),
    SceneInitializationError,
}

#[derive(PartialEq)]
pub enum AttributeType {
    Position,
    Normal,
    Index,
    Joints,
    Weights,
}

pub(super) fn get_primitive_data(
    maybe_accessor: Option<&Accessor>,
    _attribute_type: AttributeType,
    buffer_offsets: &Vec<u64>,
) -> Result<Option<(u32, u32)>, GltfErrors> {
    match maybe_accessor {
        Some(accessor) => {
            let byte_size = match accessor.data_type() {
                DataType::U16 => 2,
                DataType::F32 => 4,
                DataType::U8 => 1,
                _ => todo!(),
            };
            let num_elements = match accessor.dimensions() {
                Dimensions::Scalar => 1,
                Dimensions::Vec2 => 2,
                Dimensions::Vec3 => 3,
                Dimensions::Vec4 => 4,
                _ => todo!(),
            };
            let length = byte_size * num_elements * accessor.count();
            let buffer_view = accessor.view().ok_or(GltfErrors::NoView)?;
            let buffer_offset = buffer_offsets[buffer_view.buffer().index()];
            let offset = buffer_view.offset() + accessor.offset() + buffer_offset as usize;
            if _attribute_type == AttributeType::Joints {
                println!("len: {:?}", length);
            }
            return Ok(Some((offset as u32, length as u32)));
        }
        None => Ok(None),
    }
}

pub(super) fn get_model_meshes(
    mesh_ids: &Vec<u32>,
    nodes: &Vec<gltf::Node>,
    buffer_offsets: &Vec<u64>,
) -> Result<Vec<GMesh>, GltfErrors> {
    let mut meshes = Vec::<GMesh>::new();
    for mesh_id in mesh_ids.iter() {
        let mesh = nodes
            .iter()
            .find(|n| n.mesh().is_some() && n.mesh().unwrap().index() as u32 == *mesh_id)
            .unwrap()
            .mesh()
            .unwrap();

        let g_mesh = GMesh::new(&mesh, buffer_offsets)?;
        meshes.push(g_mesh);
    }

    Ok(meshes)
}
