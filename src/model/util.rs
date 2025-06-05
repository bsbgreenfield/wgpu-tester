use super::range_splicer;
use super::vertex::ModelVertex;
use crate::model::model::AccessorDataType;
use crate::scene::scene::*;
use gltf::accessor::DataType;
use gltf::buffer::View;
use gltf::{Accessor, Gltf};
use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

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

pub(super) fn get_primitive_index_data(
    indices_accessor: &Accessor,
    scene_buffer_data: &mut SceneBufferData,
) -> Result<(u32, u32), GltfErrors> {
    if indices_accessor.data_type() != DataType::U16 {
        return Err(GltfErrors::IndicesError(String::from(
            "the data type of this meshes indices is something other than u16!",
        )));
    }
    let indices_buffer_view = indices_accessor.view().ok_or(GltfErrors::NoView)?;

    let total_byte_offset = indices_buffer_view.offset() + indices_accessor.offset();
    let primitive_indices_range = std::ops::Range::<usize> {
        start: indices_buffer_view.offset() + indices_accessor.offset(),
        end: total_byte_offset + (indices_accessor.count() * 2),
    };
    range_splicer::define_index_ranges(
        &mut scene_buffer_data.index_ranges,
        &primitive_indices_range,
    );
    Ok((
        primitive_indices_range.start as u32,
        (primitive_indices_range.end - primitive_indices_range.start) as u32,
    ))
}

fn get_bytes_from_view(
    view: &View,
    count: usize,
    size: usize,
    initial_offset: usize,
    byte_data: &Rc<Vec<u8>>,
) -> Vec<u8> {
    let mut bytes = Vec::<u8>::new();
    let mut offset = initial_offset + view.offset();
    if let Some(stride_len) = view.stride() {
        for _ in 0..count {
            bytes.extend(&byte_data[offset..offset + size]);
            offset += stride_len;
        }
    } else {
        bytes = byte_data[offset..view.length() + offset].to_vec();
    }

    bytes
}

pub(super) fn get_primitive_data(
    accessor: &Accessor,
    expected_data_type: AccessorDataType,
) -> Result<(u32, u32), GltfErrors> {
    match accessor.data_type() {
       DataType::F32 => {
        if expected_data_type != AccessorDataType::Vec3F32 {
            return Err(GltfErrors::VericesError(String::from("Data type given is not f32!")));
        }
       } ,
       DataType::U16 => {
        if expected_data_type != AccessorDataType::U16 {
            return Err(GltfErrors::IndicesError(String::from("Data type given is not u16!")));
        }
       }
       _ => panic!("unhandled data type")
    }
    let len = match expected_data_type {
       AccessorDataType::U16 => accessor.count() * 2,
       AccessorDataType::Vec3F32 => accessor.count() * 12, 
    };
    let buffer_view = accessor.view().ok_or(GltfErrors::NoView)?;
    let offset = buffer_view.offset() + accessor.offset();
    Ok((offset as u32, len as u32))
}

/// *THIS FUNCTIONS MUTATES DATA*
/// expand the ModelVertex buffer to include the bytes specified by this primitive
/// by composing ModelVertex structs from bufferview data on the positions and normals
pub(super) fn get_primitive_vertex_data(
    position_accessor: &Accessor,
    normals_accessor: &Accessor,
    vertex_data: &mut Vec<ModelVertex>,
    byte_data: &Rc<Vec<u8>>,
) -> Result<(u32, u32), GltfErrors> {
    if position_accessor.data_type() != DataType::F32
        && normals_accessor.data_type() != DataType::F32
    {
        return Err(GltfErrors::VericesError(String::from(
            "the data type of the vertices is something other than an F32!!",
        )));
    }
    let position_buffer_view = position_accessor.view().ok_or(GltfErrors::NoView)?;
    let normals_buffer_view = position_accessor.view().ok_or(GltfErrors::NoView)?;

    let position_bytes = &get_bytes_from_view(
        &position_buffer_view,
        position_accessor.count(),
        12, // vec3<f32>
        position_accessor.offset(),
        byte_data,
    );
    let normal_bytes = &get_bytes_from_view(
        &normals_buffer_view,
        normals_accessor.count(),
        12, // vec3<f32>
        normals_accessor.offset(),
        byte_data,
    );

    let position_f32: &[f32] = bytemuck::cast_slice(position_bytes);
    let normals_f32: &[f32] = bytemuck::cast_slice(normal_bytes);

    assert_eq!(normals_f32.len(), position_f32.len()); // cannot zip if they arent the same size

    let vertex_vec: Vec<ModelVertex> = (0..(position_f32.len() / 3))
        .map(|i| ModelVertex {
            position: position_f32[i * 3..i * 3 + 3].try_into().unwrap(),
            normal: normals_f32[i * 3..i * 3 + 3].try_into().unwrap(),
        })
        .collect();

    // these are the offset and length of the ModelVertex structs within the buffer, not to be
    // confused with the byte offsets.
    let vertex_offset = vertex_data.len() as u32;
    let vertex_len = vertex_vec.len();

    vertex_data.extend(vertex_vec);

    Ok((vertex_offset, vertex_len as u32))
}
