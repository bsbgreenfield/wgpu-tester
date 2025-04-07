use super::model::Model;
use gltf::{Accessor, Gltf, Mesh, Node};
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::{env, fs};

pub struct NodeWrapper<'a> {
    node_tree: Rc<RefCell<GNodeCell<'a>>>,
    child_indices: Vec<usize>,
}
pub struct GNodeCell<'a> {
    children: Vec<Rc<RefCell<GNodeCell<'a>>>>,
    transform: [[f32; 4]; 4],
    mesh: Option<GMesh<'a>>,
}
// a Node has a mesh and a transform
pub struct GNode<'a> {
    children: Vec<Rc<GNode<'a>>>,
    transform: [[f32; 4]; 4],
    mesh: Option<GMesh<'a>>,
}

impl<'a> GNode<'a> {
    fn add_child(&mut self, gnode_ref: &Rc<GNode<'a>>) {
        self.children.push(gnode_ref.clone());
    }
}

pub struct GMesh<'a> {
    index_slice: &'a [u8],
    vertex_slice: &'a [u8],
    normal_slice: &'a [u8],
}

impl<'a> GMesh<'a> {
    pub fn new(mesh: &Option<Mesh>) -> Option<Self> {
        if mesh.is_none() {
            return None;
        }
        Some(Self {
            index_slice: &[],
            vertex_slice: &[],
            normal_slice: &[],
        })
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
    fn get_root_nodes(g_nodes: &Vec<NodeWrapper>) -> Vec<usize> {
        let mut children = HashSet::<usize>::new();
        for node in g_nodes {
            for i in node.child_indices.iter() {
                children.insert(*i);
            }
        }
        println!("indices that are listed as children: {:?}", children);
        // root nodes are the indices from 0 to g_nodes.len() that aren't in this list
        let mut root_nodes_ids = Vec::<usize>::new();
        for i in 0..g_nodes.len() {
            if !children.contains(&i) {
                root_nodes_ids.push(i);
            }
        }
        root_nodes_ids
    }
    /// When building a scene, we dont know how many models we have
    /// the [new()] function builds out one or more trees of nodes, and
    /// each model is defined as something that owns a root node.
    pub fn new(nodes: gltf::iter::Nodes) -> Self {
        let mut g_nodes = Vec::<NodeWrapper>::with_capacity(nodes.len());

        // first pass
        for node in nodes {
            let mesh = GMesh::new(&node.mesh());
            let transform = node.transform().matrix();
            let children: Vec<usize> = node.children().map(|c| c.index()).collect();
            println!("This node has children: {:?}", children);
            // in this first pass process all node data besides the children to ensure that
            // everything actually exists before recursing.
            let node = GNodeCell {
                mesh,
                transform,
                children: Vec::with_capacity(children.len()),
            };
            g_nodes.push(NodeWrapper {
                node_tree: Rc::new(RefCell::new(node)),
                child_indices: children,
            });
        }
        let root_nodes_ids: Vec<usize> = Self::get_root_nodes(&g_nodes);
        let mut completed_roots = Vec::<NodeWrapper>::new();
        // next, go through the root nodes and recurse to build the trees
        for idx in root_nodes_ids {
            println!("Got root at index {}", idx);
            let root_wrapper = g_nodes.remove(idx);
            let child_indices = &root_wrapper.child_indices;
            get_node_tree(&g_nodes, root_wrapper.node_tree.clone(), child_indices);
            print_node(&root_wrapper.node_tree);
            completed_roots.push(root_wrapper);
        }
        Self { models: vec![] }
    }
}

fn print_node<'a>(node_wrapper: &Rc<RefCell<GNodeCell<'a>>>) {
    println!(
        "This node has {} children",
        node_wrapper.borrow().children.len()
    );
    for (idx, child) in node_wrapper.borrow().children.iter().enumerate() {
        println!("child # {}:", idx);
        print_node(child);
    }
}

fn get_node_tree<'a>(
    g_nodes: &Vec<NodeWrapper<'a>>,
    root_node: Rc<RefCell<GNodeCell<'a>>>,
    child_indices: &Vec<usize>,
) {
    // for each child of root node, recurse into the child to add its children
    for child_idx in child_indices {
        let child = Rc::clone(&g_nodes[*child_idx].node_tree);
        let child_ids = &g_nodes[*child_idx].child_indices;
        get_node_tree(&g_nodes, child.clone(), child_ids);

        root_node.borrow_mut().children.push(child);
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
    let scene: GScene = GScene::new(gltf.nodes());
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
