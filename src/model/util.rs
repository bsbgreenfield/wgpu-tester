use super::range_splicer;
use super::vertex::ModelVertex;
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
/// get exactly one gltf file and one bin file from the provided directory
/// TODO: make these errors more usefull
fn get_gltf_file(dir_path: PathBuf) -> Result<(PathBuf, PathBuf), std::io::Error> {
    let mut gltf_file: Option<PathBuf> = None;
    let mut bin_file: Option<PathBuf> = None;
    let err = std::io::ErrorKind::InvalidData;
    println!("{:?}", dir_path);
    for entry in fs::read_dir(&dir_path)? {
        if gltf_file.is_some() && bin_file.is_some() {
            break;
        }
        if let Ok(e) = entry {
            println!("{:?}", e);
            let path = e.path();
            match path.extension() {
                Some(path_ext) => match path_ext.to_str().ok_or(err)? {
                    "gltf" => {
                        if gltf_file.is_some() {
                            return Err(err.into());
                        }
                        gltf_file = Some(path);
                    }
                    "bin" => {
                        if bin_file.is_some() {
                            return Err(err.into());
                        }
                        bin_file = Some(path);
                    }
                    _ => {}
                },
                None => {}
            }
        }
    }
    if gltf_file.is_none() || bin_file.is_none() {
        return Err(err.into());
    } else {
        return Ok((gltf_file.unwrap(), bin_file.unwrap()));
    }
}

pub fn load_gltf(
    dirname: &str,
    device: &wgpu::Device,
    aspect_ratio: f32,
) -> Result<GScene, gltf::Error> {
    let dir_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("res")
        .join(dirname);
    println!("{:?}", dir_path);
    if !dir_path.is_dir() {
        return Err(gltf::Error::Io(std::io::ErrorKind::NotFound.into()));
    }
    let (gltf_file, bin) = get_gltf_file(dir_path)?;
    let gltf = Gltf::open(gltf_file)?;
    let buffer_data = std::fs::read(bin)?;
    // only use the first scene for now
    let scene = gltf.scenes().next().ok_or(gltf::Error::UnsupportedScheme)?;
    let buffer_data_rc = Rc::new(buffer_data);
    let mesh_node_iter = scene
        .nodes()
        .filter(|n| n.mesh().is_some() || n.children().len() != 0);
    let root_node_ids: Vec<usize> = mesh_node_iter.map(|n| n.index()).collect();
    let scene = GScene::new(
        gltf.nodes(),
        root_node_ids,
        buffer_data_rc,
        device,
        aspect_ratio,
    );
    match scene {
        Ok(scene) => return Ok(scene),
        Err(err) => panic!("{:?}", err),
    }
}
