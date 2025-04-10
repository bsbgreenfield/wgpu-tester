use super::model::Model;
use gltf::{Accessor, Gltf, Mesh, Node};
use std::cell::RefCell;
use std::collections::HashSet;
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;
use std::{env, fs};

pub struct NodeWrapper<'a> {
    node_ref: Rc<GNode<'a>>,
    child_indices: Vec<usize>,
}
// a Node has a mesh and a transform
pub struct GNode<'a> {
    children: RefCell<Vec<Rc<GNode<'a>>>>,
    transform: [[f32; 4]; 4],
    mesh: Option<GMesh<'a>>,
}
impl<'a> GNode<'a> {
    fn add_child(self: &Rc<Self>, child: Rc<GNode<'a>>) {
        self.children.borrow_mut().push(child);
    }
}
pub struct GMesh<'a> {
    index_slice: &'a [u8],
    vertex_slice: &'a [u8],
    normal_slice: &'a [u8],
}

impl<'a> GMesh<'a> {
    pub fn new(maybe_mesh: &Option<Mesh>, buffer_data: &Vec<u8>) -> Option<Self> {
        if let Some(mesh) = maybe_mesh {
            // for now, only accept one primitive per mesh
            let primitive = mesh.primitives().nth(1);
            Some(Self {
                index_slice: &[],
                vertex_slice: &[],
                normal_slice: &[],
            })
        } else {
            return None;
        }
    }
}

// each Model has exactly one node tree
// in order to draw itself, it traverses this tree
// creating the required buffers
pub struct GModel<'a> {
    byte_data: Rc<Vec<u8>>,
    node: Option<GNode<'a>>,
}

// used to gather the models
pub struct GScene<'a> {
    models: Vec<GModel<'a>>,
}

impl<'a> GScene<'a> {
    pub fn new(nodes: gltf::iter::Nodes, root_nodes_ids: Vec<usize>, buffer_data: Vec<u8>) -> Self {
        // step 1: get the nodes
        let g_nodes = get_nodes(nodes, root_nodes_ids, buffer_data);
        Self { models: vec![] }
    }
}

fn get_nodes(
    nodes: gltf::iter::Nodes,
    root_nodes_ids: Vec<usize>,
    buffer_data: Vec<u8>,
) -> Vec<Rc<GNode>> {
    let mut node_wrappers = Vec::<NodeWrapper>::with_capacity(nodes.len());
    let mut ret: Vec<Rc<GNode>> = Vec::with_capacity(root_nodes_ids.len());
    for node in nodes {
        let mesh = GMesh::new(&node.mesh(), &buffer_data);
        let transform = node.transform().matrix();
        let children: Vec<usize> = node.children().map(|c| c.index()).collect();
        // in this first pass process all node data besides the children to ensure that
        // everything actually exists before recursing.
        let node = GNode {
            mesh,
            transform,
            children: RefCell::new(Vec::with_capacity(children.len())),
        };
        // push a reference to the node to the g_nodes vec, along with the indices it will need
        node_wrappers.push(NodeWrapper {
            node_ref: Rc::new(node),
            child_indices: children,
        });
        // for each root node, loop through the children at the indices specified for this node
        // clone a new Rc from the Rc<child> node and add it to the children vec
        for i in &root_nodes_ids {
            let root_node = &node_wrappers[*i].node_ref.clone();
            let child_indices = &node_wrappers[*i].child_indices;
            build_node(&node_wrappers, root_node, child_indices);
            ret.push(root_node.clone());
        }
    }
    ret
}

fn build_node<'a>(
    g_nodes: &Vec<NodeWrapper<'a>>,
    root_node: &Rc<GNode<'a>>,
    child_indices: &Vec<usize>,
) {
    for child_id in child_indices {
        let child_wrapper = &g_nodes[*child_id];
        let child = child_wrapper.node_ref.clone();
        build_node(g_nodes, &child, child_indices);
        root_node.add_child(child);
    }
}

pub enum gltf_errors {
    NoIndices,
    NoView,
}

pub fn load_gltf(dirname: &str) -> Result<Vec<Model>, gltf::Error> {
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
    let models = Vec::<Model>::with_capacity(gltf.nodes().len());
    let root_nodes = scene.nodes().map(|n| n.index()).collect();
    let scene: GScene = GScene::new(gltf.nodes(), root_nodes, buffer_data);
    Ok(vec![])
}

fn get_node_meshes(node: Node) -> Result<Vec<Mesh>, gltf_errors> {
    let mut meshes = Vec::<Mesh>::new();
    if let Some(mesh) = node.mesh() {
        meshes.push(mesh);
    }
    for child_node in node.children() {
        let child_meshes = get_node_meshes(child_node)?;
        for mesh in &child_meshes {
            let indices: &[f32] = get_indices_slice(mesh)?;
        }
    }

    Ok(meshes)
}

fn get_indices_slice<'a>(mesh: &'a Mesh) -> Result<&'a [f32], gltf_errors> {
    // TODO: process more primitives?
    // use only the first primitive for now
    let primitive = mesh.primitives().next().unwrap();

    // if there are no indices specified for this primitive, thats an error
    let indices_accessor: Accessor = primitive.indices().ok_or(gltf_errors::NoIndices)?;
    Ok(&[])
}

//fn get_slice_from_accessor<'a>(accessor: &'a Accessor) -> Result<&'a [f32], gltf_errors> {
//    let view = accessor.view().ok_or(gltf_errors::NoView)?;
//    view.buffer().source()
//}

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
            match path.extension().ok_or(err)?.to_str().ok_or(err)? {
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
            }
        }
    }
    if gltf_file.is_none() || bin_file.is_none() {
        return Err(err.into());
    } else {
        return Ok((gltf_file.unwrap(), bin_file.unwrap()));
    }
}
