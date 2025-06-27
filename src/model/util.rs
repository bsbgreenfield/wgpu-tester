use crate::model::model::{AccessorDataType, GMesh};
use gltf::accessor::DataType;
use gltf::Accessor;
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
pub enum AttributeType {
    Position,
    Normal,
    Index,
}

pub(super) fn get_primitive_data2(
    maybe_accessor: Option<&Accessor>,
    attribute_type: AttributeType,
) -> Result<(u32, u32), GltfErrors> {
    match maybe_accessor {
        Some(accessor) => {
            let len = accessor.size();
            let buffer_view = accessor.view().ok_or(GltfErrors::NoView)?;
            let offset = buffer_view.offset() + accessor.offset();
            return Ok((offset as u32, (accessor.count() * len) as u32));
        }
        None => Ok((0, 0)),
    }
}

pub(super) fn get_primitive_data(
    accessor: &Accessor,
    expected_data_type: AccessorDataType,
) -> Result<(u32, u32), GltfErrors> {
    match accessor.data_type() {
        DataType::F32 => {
            if expected_data_type != AccessorDataType::Vec3F32 {
                return Err(GltfErrors::VericesError(String::from(
                    "Data type given is not f32!",
                )));
            }
        }
        DataType::U16 => {
            if expected_data_type != AccessorDataType::U16 {
                return Err(GltfErrors::IndicesError(String::from(
                    "Data type given is not u16!",
                )));
            }
        }
        DataType::U8 => {
            println!("expected {:?}", expected_data_type);
            println!("actual {:?}", accessor.data_type());
            if expected_data_type != AccessorDataType::U8 {
                return Err(GltfErrors::IndicesError(String::from(
                    "Data type given is not u16!",
                )));
            }
        }

        _ => {
            println!("{:?}", accessor.data_type());
            panic!();
        }
    }
    let len = match expected_data_type {
        AccessorDataType::U8 => accessor.count(),
        AccessorDataType::U16 => accessor.count() * 2,
        AccessorDataType::Vec3F32 => accessor.count() * 12,
    };
    let buffer_view = accessor.view().ok_or(GltfErrors::NoView)?;
    let offset = buffer_view.offset() + accessor.offset();
    Ok((offset as u32, len as u32))
}
pub(super) fn get_model_meshes(
    mesh_ids: &Vec<u32>,
    nodes: &Vec<gltf::Node>,
) -> Result<Vec<GMesh>, GltfErrors> {
    let mut meshes = Vec::<GMesh>::new();
    for mesh_id in mesh_ids.iter() {
        let mesh = nodes
            .iter()
            .find(|n| n.mesh().is_some() && n.mesh().unwrap().index() as u32 == *mesh_id)
            .unwrap()
            .mesh()
            .unwrap();
        let g_mesh = GMesh::new(&mesh)?;
        meshes.push(g_mesh);
    }

    Ok(meshes)
}
