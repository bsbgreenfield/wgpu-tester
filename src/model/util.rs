use crate::model::model::AccessorDataType;
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
pub enum InitializationError<'a> {
    InstanceDataInitializationError(&'a str),
    SceneMergeError(&'a str),
    SceneInitializationError,
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
        _ => panic!("unhandled data type"),
    }
    let len = match expected_data_type {
        AccessorDataType::U16 => accessor.count() * 2,
        AccessorDataType::Vec3F32 => accessor.count() * 12,
    };
    let buffer_view = accessor.view().ok_or(GltfErrors::NoView)?;
    let offset = buffer_view.offset() + accessor.offset();
    Ok((offset as u32, len as u32))
}
