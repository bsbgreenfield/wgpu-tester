use crate::{model::model::GMesh, scene::scene::PrimitiveData};
use bytemuck::AnyBitPattern;
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
    IBMS,
}

pub fn copy_binary_data_from_gltf(
    accessor: &Accessor,
    accessor_type: gltf::Semantic,
    buffer_offsets: &Vec<u64>,
    binary_data: &Vec<u8>,
) -> Result<Vec<u8>, GltfErrors> {
    let view = accessor.view().ok_or(GltfErrors::NoView)?;
    let byte_offset =
        view.offset() + accessor.offset() + buffer_offsets[view.buffer().index()] as usize;
    let count = accessor.count();
    let byte_size = match accessor.data_type() {
        DataType::U8 => 1,
        DataType::U16 => 2,
        DataType::F32 => 4,
        _ => todo!(),
    };
    let num_elements = match accessor.dimensions() {
        Dimensions::Scalar => 1,
        Dimensions::Vec2 => 2,
        Dimensions::Vec3 => 3,
        Dimensions::Vec4 => 4,
        Dimensions::Mat4 => 16,
        _ => todo!(),
    };

    let stride = match view.stride() {
        Some(s) => s,
        None => 0,
    };

    println!(
        "{:?}: byte_offset: {}, byte_size: {}, count: {}, num_elements: {}, stride {}",
        accessor_type, byte_offset, byte_size, count, num_elements, stride
    );
    let mut copy_dest: Vec<u8> = Vec::with_capacity(byte_size * num_elements * count);
    let mut byte_loc = byte_offset;
    for _ in 0..count {
        for _ in 0..num_elements {
            for _ in 0..byte_size {
                copy_dest.push(binary_data[byte_loc]);
                byte_loc += 1;
            }
        }
        byte_loc += stride - (byte_size * num_elements); // if the stride is equal to the byte size
                                                         // of the component, then no need to adjust alignment
    }
    assert_eq!(copy_dest.len(), byte_size * num_elements * count);

    // for i in 0..count {
    //     let o = i * num_elements * byte_size;
    //     println!(
    //         "{:?}",
    //         bytemuck::cast_slice::<u8, f32>(&copy_dest[o..o + num_elements * byte_size])
    //     );
    // }
    Ok(copy_dest)
}

pub fn get_data_from_binary<'a, T: AnyBitPattern>(
    offset: u32,
    len: u32,
    binary_data: &'a Vec<u8>,
) -> &'a [T] {
    let data_bytes = &binary_data[offset as usize..(offset + len) as usize];
    let cast_slice = bytemuck::cast_slice::<u8, T>(data_bytes);
    cast_slice
}

pub(super) fn get_index_offset_len(
    maybe_accessor: Option<&Accessor>,
    buffer_offsets: &Vec<u64>,
) -> Result<Option<(usize, usize)>, GltfErrors> {
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
                Dimensions::Mat4 => 16,
                _ => todo!(),
            };
            let length = byte_size * num_elements * accessor.count();
            let buffer_view = accessor.view().ok_or(GltfErrors::NoView)?;
            let buffer_offset = buffer_offsets[buffer_view.buffer().index()];
            let offset = buffer_view.offset() + accessor.offset() + buffer_offset as usize;
            return Ok(Some((offset, length)));
        }
        None => Ok(None),
    }
}

pub(super) fn get_model_meshes(
    mesh_ids: &Vec<u32>,
    nodes: &Vec<gltf::Node>,
    buffer_offsets: &Vec<u64>,
    binary_data: &Vec<u8>,
) -> Result<(Vec<GMesh>, Vec<PrimitiveData>), GltfErrors> {
    let mut mesh_primitive_data: Vec<PrimitiveData> = Vec::new();
    let mut meshes = Vec::<GMesh>::new();
    for mesh_id in mesh_ids.iter() {
        let mesh = nodes
            .iter()
            .find(|n| n.mesh().is_some() && n.mesh().unwrap().index() as u32 == *mesh_id)
            .unwrap()
            .mesh()
            .unwrap();

        let g_mesh = GMesh::new(&mesh)?;
        let primitive_data = GMesh::get_primitive_data(&mesh, buffer_offsets, binary_data)?;
        meshes.push(g_mesh);
        mesh_primitive_data.extend(primitive_data);
    }

    Ok((meshes, mesh_primitive_data))
}
