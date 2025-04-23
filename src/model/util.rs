use gltf::{Gltf, Node};
use std::cell::RefCell;
use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use super::model2::{GMesh, GModel, GScene};
use super::vertex::ModelVertex;

#[derive(Debug)]
pub enum GltfErrors {
    NoIndices,
    NoView,
    NoPrimitive,
    IndicesError(String),
    VericesError(String),
    NormalsError(String),
}

pub struct NodeWrapper<'a> {
    node_ref: Rc<GNode<'a>>,
    child_indices: Vec<usize>,
}
// a Node has a mesh and a transform
pub struct GNode<'a> {
    pub children: RefCell<Vec<Rc<GNode<'a>>>>,
    pub transform: [[f32; 4]; 4],
    pub mesh: Option<GMesh>,
}
impl<'a> GNode<'a> {
    fn add_child(self: &Rc<Self>, child: Rc<GNode<'a>>) {
        self.children.borrow_mut().push(child);
    }
}

// each Model has exactly one node tree
// in order to draw itself, it traverses this tree
// creating the required buffers

// used to gather the models

pub fn find_meshes(
    root_node: &Node,
    mut mesh_ids: Vec<u32>,
    mut mesh_instances: Vec<u32>,
) -> (Vec<u32>, Vec<u32>) {
    'block: {
        if let Some(mesh) = root_node.mesh() {
            // check mesh_ids to see if this particular mesh has already been added, if so, the index
            // of the match is equal to the index within mesh_instances that we want to increment by 1
            for (idx, m) in mesh_ids.iter().enumerate() {
                if *m == mesh.index() as u32 {
                    mesh_instances[idx] += 1;
                    break 'block;
                }
            }
            // this mesh has not been added: append to both vecs
            mesh_ids.push(mesh.index() as u32);
            mesh_instances.push(1);
        }
    }
    for child_node in root_node.children() {
        (mesh_ids, mesh_instances) = find_meshes(&child_node, mesh_ids, mesh_instances);
    }

    (mesh_ids, mesh_instances)
}

pub fn get_meshes(
    nodes: gltf::iter::Nodes,
    buffer_data: &Rc<Vec<u8>>,
    vertex_data: &mut Vec<ModelVertex>,
    index_data: &mut Vec<u16>,
) -> Result<Vec<GMesh>, GltfErrors> {
    let mut meshes = Vec::<GMesh>::new();
    for node in nodes {
        if let Some(mesh) = node.mesh() {
            if !has_mesh(&meshes, mesh.index() as u32) {
                let g_mesh = GMesh::newF(&mesh, buffer_data, vertex_data, index_data)?;
                meshes.push(g_mesh);
            }
        }
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

/// get exactly one gltf file and one bin file from the provided directory
/// TODO: make these errors more usefull
fn get_gltf_file(dir_path: PathBuf) -> Result<(PathBuf, PathBuf), std::io::Error> {
    let mut gltf_file: Option<PathBuf> = None;
    let mut bin_file: Option<PathBuf> = None;
    let err = std::io::ErrorKind::InvalidData;
    for entry in fs::read_dir(&dir_path)? {
        if gltf_file.is_some() && bin_file.is_some() {
            break;
        }
        if let Ok(e) = entry {
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
fn print_node(node: &GNode) {
    println!("node with {} children", node.children.borrow().len());
    for (idx, child) in node.children.borrow().iter().enumerate() {
        println!("child {}", idx);
        print_node(child);
    }
}

pub fn load_gltf(dirname: &str, device: &wgpu::Device) -> Result<Vec<GModel>, gltf::Error> {
    let dir_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("res")
        .join(dirname);
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

    println!("Sucess!");
    Ok(vec![])
}
