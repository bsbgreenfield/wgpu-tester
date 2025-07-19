use crate::{model::model::GMesh, scene::scene::PrimitiveData};
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

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum AttributeType {
    Position,
    Normal,
    Index,
    Joints,
    Weights,
    IBMS,
    Times,
    RotationT,
    TranslationT,
    ScaleT,
}
impl AttributeType {
    pub fn from_animation_channel(channel: &gltf::animation::Channel) -> Self {
        match channel.target().property() {
            gltf::animation::Property::Translation => AttributeType::TranslationT,
            gltf::animation::Property::Rotation => AttributeType::RotationT,
            gltf::animation::Property::Scale => AttributeType::ScaleT,
            _ => panic!(),
        }
    }
}

pub fn copy_binary_data_from_gltf(
    accessor: &Accessor,
    accessor_type: AttributeType,
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

    let mut copy_dest: Vec<u8> = Vec::with_capacity(byte_size * num_elements * count);
    let mut byte_loc = byte_offset;
    let extra_stride = if let Some(stride) = view.stride() {
        stride - (byte_size * num_elements)
    } else {
        0
    };

    //println!(
    //    "{:?}: element size {}, count: {}, offset: {}, stride: {:?}",
    //    accessor_type,
    //    (byte_size * num_elements),
    //    count,
    //    byte_offset,
    //    view.stride()
    //);
    for _ in 0..count {
        for _ in 0..num_elements {
            for _ in 0..byte_size {
                copy_dest.push(binary_data[byte_loc]);
                byte_loc += 1;
            }
        }

        byte_loc += extra_stride;
        // of the component, then no need to adjust alignment
    }
    assert_eq!(copy_dest.len(), byte_size * num_elements * count);

    Ok(copy_dest)
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
