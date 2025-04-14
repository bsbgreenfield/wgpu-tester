use super::model::{Model, ObjectTransform};
use gltf::accessor::DataType;
use gltf::buffer::View;
use gltf::{Accessor, Gltf, Mesh, Node};
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::Debug;
use std::ops::Deref;
use std::path::{self, PathBuf};
use std::rc::Rc;
use std::{env, fs};

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
    pub mesh: Option<GMesh<'a>>,
}
impl<'a> GNode<'a> {
    fn add_child(self: &Rc<Self>, child: Rc<GNode<'a>>) {
        self.children.borrow_mut().push(child);
    }
}

pub struct ModelInstanceData {
    vertices: Vec<u8>,
    indices: Vec<u8>,
    transforms: Vec<[[f32; 4]; 4]>,
    vertex_offsets: Vec<usize>,
    index_offsets: Vec<usize>,
}

impl ModelInstanceData {
    fn new<'a>(node: &'a GNode) -> Self {
        let vertices = Vec::<u8>::new();
        let indices = Vec::<u8>::new();
        let transforms = Vec::<[[f32; 4]; 4]>::new();
        let vertex_offsets = Vec::<usize>::new();
        let index_offsets = Vec::<usize>::new();
        let mut model_data = ModelInstanceData {
            vertices,
            indices,
            transforms,
            vertex_offsets,
            index_offsets,
        };
        model_data
    }
}

pub struct GMesh<'a> {
    index_slice: &'a [u8],
    vertex_slice: &'a [u8],
    normal_slice: &'a [u8],
}

impl<'a> GMesh<'a> {
    pub fn new(
        maybe_mesh: &Option<Mesh>,
        buffer_data: &'a Vec<u8>,
    ) -> Result<Option<Self>, GltfErrors> {
        if let Some(mesh) = maybe_mesh {
            // for now, only accept one primitive per mesh
            let primitive = mesh.primitives().nth(0).ok_or(GltfErrors::NoPrimitive)?;
            let indices_accessor = primitive.indices().ok_or(GltfErrors::NoIndices)?;
            let indices = Self::get_mesh_indices(buffer_data, &indices_accessor)?;
            let mut maybe_vertices: Option<&[u8]> = None;
            let mut maybe_normals: Option<&[u8]> = None;
            for (semantic, accessor) in primitive.attributes() {
                match semantic {
                    gltf::Semantic::Positions => {
                        let vertices_result = Self::get_mesh_vertices(buffer_data, &accessor)?;
                        let _ = maybe_vertices.insert(vertices_result);
                    }
                    gltf::Semantic::Normals => {
                        let normals_results = Self::get_mesh_normals(buffer_data, &accessor)?;
                        let _ = maybe_normals.insert(normals_results);
                    }
                    _ => {}
                }
            }
            let vertices = maybe_vertices.ok_or(GltfErrors::VericesError(String::from(
                "could not get vertices from the accessor",
            )))?;
            let normals = maybe_normals.ok_or(GltfErrors::NormalsError(String::from(
                "could not get normal from accessor",
            )))?;
            Ok(Some(Self {
                index_slice: indices,
                vertex_slice: vertices,
                normal_slice: normals,
            }))
        } else {
            return Ok(None);
        }
    }
    fn get_mesh_normals(
        buffer_data: &'a Vec<u8>,
        normals_accessor: &Accessor,
    ) -> Result<&'a [u8], GltfErrors> {
        if normals_accessor.data_type() != DataType::F32 {
            return Err(GltfErrors::NormalsError(String::from(
                "normal data type is somethings other than F32!",
            )));
        }
        let normals_buffer_view = normals_accessor.view().ok_or(GltfErrors::NoView)?;
        match normals_buffer_view.stride() {
            Some(_) => todo!("havent implemented stride for bufferview"),
            None => {
                return Ok(Self::get_byte_data_from_view(
                    buffer_data,
                    &normals_buffer_view,
                ));
            }
        }
    }

    fn get_mesh_vertices(
        buffer_data: &'a Vec<u8>,
        position_accessor: &Accessor,
    ) -> Result<&'a [u8], GltfErrors> {
        if position_accessor.data_type() != DataType::F32 {
            return Err(GltfErrors::VericesError(String::from(
                "the data type of the vertices is something other than an F32!!",
            )));
        }
        let vertices_buffer_view = position_accessor.view().ok_or(GltfErrors::NoView)?;
        match vertices_buffer_view.stride() {
            Some(_) => todo!("havent implemented stride for bufferview"),
            None => {
                return Ok(Self::get_byte_data_from_view(
                    buffer_data,
                    &vertices_buffer_view,
                ));
            }
        }
    }
    fn get_mesh_indices(
        buffer_data: &'a Vec<u8>,
        indices_accessor: &Accessor,
    ) -> Result<&'a [u8], GltfErrors> {
        if indices_accessor.data_type() != DataType::U16 {
            return Err(GltfErrors::IndicesError(String::from(
                "the data type of this meshes indices is something other than u16!",
            )));
        }
        let indices_buffer_view = indices_accessor.view().ok_or(GltfErrors::NoView)?;
        match indices_buffer_view.stride() {
            Some(_) => todo!("havent implemented strides for bufferview yet!!"),
            None => {
                return Ok(Self::get_byte_data_from_view(
                    buffer_data,
                    &indices_buffer_view,
                ));
            }
        }
    }

    fn get_byte_data_from_view(buffer_data: &'a Vec<u8>, view: &View) -> &'a [u8] {
        let start = view.offset();
        let end = start + view.length();
        &buffer_data[start..end]
    }
}

// each Model has exactly one node tree
// in order to draw itself, it traverses this tree
// creating the required buffers
pub struct GModel<'a> {
    pub byte_data: Rc<Vec<u8>>,
    pub node: Option<Rc<GNode<'a>>>,
}

// used to gather the models
pub struct GScene<'a> {
    models: Vec<GModel<'a>>,
}

impl<'a> GScene<'a> {
    pub fn new(
        nodes: gltf::iter::Nodes<'a>,
        root_nodes_ids: Vec<usize>,
        buffer_data: &'a Vec<u8>,
    ) -> Self {
        // step 1: get the nodes
        let g_nodes = get_nodes(nodes, root_nodes_ids, &buffer_data);
        let mut models = Vec::with_capacity(g_nodes.len());
        for node in g_nodes {
            models.push(GModel {
                byte_data: Rc::new(vec![]),
                node: Some(node),
            });
        }
        Self { models }
    }
}

fn get_nodes<'a>(
    nodes: gltf::iter::Nodes,
    root_nodes_ids: Vec<usize>,
    buffer_data: &'a Vec<u8>,
) -> Vec<Rc<GNode<'a>>> {
    let mut node_wrappers = Vec::<NodeWrapper>::with_capacity(nodes.len());
    let mut ret: Vec<Rc<GNode>> = Vec::with_capacity(root_nodes_ids.len());
    for node in nodes {
        let mesh = GMesh::new(&node.mesh(), &buffer_data);
        let transform = node.transform().matrix();
        let children: Vec<usize> = node.children().map(|c| c.index()).collect();
        // in this first pass process all node data besides the children to ensure that
        // everything actually exists before recursing.
        let node = GNode {
            mesh: mesh.unwrap(),
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
    }
    for i in &root_nodes_ids {
        let root_node = &node_wrappers[*i].node_ref.clone();
        let child_indices = &node_wrappers[*i].child_indices;
        build_node(&node_wrappers, root_node, child_indices);
        ret.push(root_node.clone());
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
        let child_child_indices = &child_wrapper.child_indices;
        let child = child_wrapper.node_ref.clone();
        build_node(g_nodes, &child, child_child_indices);
        root_node.add_child(child);
    }
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
    let root_nodes = scene.nodes().map(|n| n.index()).collect();
    let scene: GScene = GScene::new(gltf.nodes(), root_nodes, &buffer_data);
    Ok(vec![])
}

fn get_node_meshes(node: Node) -> Result<Vec<Mesh>, GltfErrors> {
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

fn get_indices_slice<'a>(mesh: &'a Mesh) -> Result<&'a [f32], GltfErrors> {
    // TODO: process more primitives?
    // use only the first primitive for now
    let primitive = mesh.primitives().next().unwrap();

    // if there are no indices specified for this primitive, thats an error
    let indices_accessor: Accessor = primitive.indices().ok_or(GltfErrors::NoIndices)?;
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
fn print_model<'a>(model: &GModel<'a>) {
    if let Some(root_node) = &model.node {
        print_node(root_node);
    }
}
fn print_node(node: &GNode) {
    println!("node with {} children", node.children.borrow().len());
    for (idx, child) in node.children.borrow().iter().enumerate() {
        println!("child {}", idx);
        print_node(child);
    }
}
