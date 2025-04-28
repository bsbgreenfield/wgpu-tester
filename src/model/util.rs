use crate::scene;

use super::model2::SceneMeshData;
use super::model2::{GMesh, GModel, GScene, SceneBufferData};
use super::vertex::ModelVertex;
use cgmath::SquareMatrix;
use gltf::accessor::DataType;
use gltf::json::mesh;
use gltf::{Accessor, Gltf, Node};
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

// each Model has exactly one node tree
// in order to draw itself, it traverses this tree
// creating the required buffers

// used to gather the models

pub fn find_meshes(
    root_node: &Node,
    mut scene_mesh_data: SceneMeshData,
    mut base_translation: [[f32; 4]; 4],
) -> SceneMeshData {
    'block: {
        let cg_base = cgmath::Matrix4::<f32>::from(base_translation);
        let cg_trans = cgmath::Matrix4::<f32>::from(root_node.transform().matrix());
        base_translation = (cg_base * cg_trans).into();
        if let Some(mesh) = root_node.mesh() {
            // this is an instance of a mesh. Push the current base translation
            scene_mesh_data
                .transformation_matrices
                .push(base_translation);
            // check mesh_ids to see if this particular mesh has already been added, if so, the index
            // of the match is equal to the index within mesh_instances that we want to increment by 1
            for (idx, m) in scene_mesh_data.mesh_ids.iter().enumerate() {
                if *m == mesh.index() as u32 {
                    scene_mesh_data.mesh_instances[idx] += 1;
                    break 'block;
                }
            }
            // this mesh has not been added: append to both vecs
            scene_mesh_data.mesh_ids.push(mesh.index() as u32);
            scene_mesh_data.mesh_instances.push(1);
        }
    }
    for child_node in root_node.children() {
        scene_mesh_data = find_meshes(&child_node, scene_mesh_data, base_translation);
    }

    scene_mesh_data
}

pub fn get_meshes(
    mesh_ids: &Vec<u32>,
    nodes: &Vec<Node>,
    scene_buffer_data: &mut SceneBufferData,
) -> Result<Vec<GMesh>, GltfErrors> {
    let mut meshes = Vec::<GMesh>::new();
    println!("this is the meshes passed into get meshes {:?}", mesh_ids);
    for mesh_id in mesh_ids.iter() {
        let mesh = nodes
            .iter()
            .find(|n| n.mesh().is_some() && n.mesh().unwrap().index() as u32 == *mesh_id)
            .unwrap()
            .mesh()
            .unwrap();
        let g_mesh = GMesh::new(&mesh, scene_buffer_data)?;
        meshes.push(g_mesh);
    }
    Ok(meshes)
}

fn has_mesh(mesh_wrappers: &Vec<GMesh>, index: u32) -> bool {
    for wrapper in mesh_wrappers {
        if wrapper.index == index {
            return true;
        }
    }
    false
}

pub fn get_primitive_index_data(
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
    let indices_bytes = &byte_data[indices_buffer_view.offset()
        ..(indices_buffer_view.length() + indices_buffer_view.offset())];

    // get a [u16] slice from the u8 data
    let indices_u16 = bytemuck::cast_slice::<u8, u16>(indices_bytes);

    // the offset within our composed index buffer is equal to the current length of
    // of the buffer (of u16s) * 2 (2 bytes per u16 element);
    // however, we are NOT multiplying indices len by 2, becuase we acutally need that number
    // as is for render_pass.draw_indexed.
    let primitive_indices_offset = index_data.len() * 2;
    let primitive_indices_len = indices_u16.len();

    println!(
        "this primitive has slice {} to {}",
        primitive_indices_offset as u32,
        (primitive_indices_offset as u32 + primitive_indices_len as u32)
    );

    index_data.extend(indices_u16);

    Ok((
        primitive_indices_offset as u32,
        primitive_indices_len as u32,
    ))
}
/// *THIS FUNCTIONS MUTATES DATA*
/// expand the ModelVertex buffer to include the bytes specified by this primitive
/// by composing ModelVertex structs from bufferview data on the positions and normals
pub fn get_primitive_vertex_data(
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

    let position_bytes = &byte_data[position_buffer_view.offset()
        ..position_buffer_view.length() + position_buffer_view.offset()];
    let normal_bytes = &byte_data
        [normals_buffer_view.offset()..normals_buffer_view.length() + normals_buffer_view.offset()];

    let position_f32: &[f32] = bytemuck::cast_slice(position_bytes);
    let normals_f32: &[f32] = bytemuck::cast_slice(normal_bytes);

    assert_eq!(normals_f32.len(), position_f32.len()); // cannot zip if they arent the same size

    let vertex_vec: Vec<ModelVertex> = (0..(position_f32.len() / 3))
        .map(|i| ModelVertex {
            position: position_f32[i * 3..i * 3 + 3].try_into().unwrap(),
            normal: normals_f32[i * 3..i * 3 + 3].try_into().unwrap(),
        })
        .collect();

    // the offset for the composed vertex buffer is equal to the length of the elements in the
    // ModelVertex vec * 24. This is because each ModelVertex is 24 bytes long.
    // similarly, the length of the slice is the length of the f32 slice * 4, because an f32 is 4
    // bytes long.
    let vertex_offset = (vertex_data.len() as u32) * 24;
    let vertex_len = (position_f32.len() * 4) + (normals_f32.len() * 4);

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

pub fn load_gltf(dirname: &str, device: &wgpu::Device) -> Result<GScene, gltf::Error> {
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
    let root_node_ids: Vec<usize> = scene.nodes().map(|n| n.index()).collect();
    let scene = GScene::new(gltf.nodes(), root_node_ids, buffer_data_rc, device);
    match scene {
        Ok(scene) => return Ok(scene),
        Err(_) => panic!(),
    }
}
