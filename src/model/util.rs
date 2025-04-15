use gltf::accessor::DataType;
use gltf::buffer::View;
use gltf::{Accessor, Gltf, Mesh, Node};
use std::cell::RefCell;
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

#[derive(Debug, Clone, Copy)]
pub struct GMesh {
    pub index: u32,
    vertex_offset: u32,
    indices_offset: u32,
    indices_length: u32,
}

impl GMesh {
    pub fn new<'a>(mesh: &Mesh, buffer_data: &'a Vec<u8>) -> Result<Self, GltfErrors> {
        let primitive = mesh.primitives().nth(0).ok_or(GltfErrors::NoPrimitive)?;
        let indices_accessor = primitive.indices().ok_or(GltfErrors::NoIndices)?;
        let mut maybe_vertex_offset: Option<u32> = None;
        let (indices_offset, indices_length) =
            Self::get_mesh_indices(buffer_data, &indices_accessor)?;
        for (semantic, accessor) in primitive.attributes() {
            match semantic {
                gltf::Semantic::Positions => {
                    let vertex_offset = Self::get_mesh_vertices(&accessor)?;
                    let _ = maybe_vertex_offset.insert(vertex_offset);
                }
                // gltf::Semantic::Normals => {
                //     let normals_results = Self::get_mesh_normals(buffer_data, &accessor)?;
                //     let _ = maybe_normals.insert(normals_results);
                // }
                _ => {}
            }
        }

        if maybe_vertex_offset.is_some() {
            return Ok(GMesh {
                index: mesh.index() as u32,
                vertex_offset: maybe_vertex_offset.unwrap(),
                indices_offset,
                indices_length,
            });
        }

        Err(GltfErrors::VericesError(String::from(
            "could not compose mesh",
        )))
    }
    fn get_mesh_indices<'a>(
        buffer_data: &'a Vec<u8>,
        indices_accessor: &Accessor,
    ) -> Result<(u32, u32), GltfErrors> {
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
    fn get_mesh_vertices<'a>(position_accessor: &Accessor) -> Result<u32, GltfErrors> {
        if position_accessor.data_type() != DataType::F32 {
            return Err(GltfErrors::VericesError(String::from(
                "the data type of the vertices is something other than an F32!!",
            )));
        }
        let vertices_buffer_view = position_accessor.view().ok_or(GltfErrors::NoView)?;
        match vertices_buffer_view.stride() {
            Some(_) => todo!("havent implemented stride for bufferview"),
            None => {
                return Ok(position_accessor.offset() as u32);
            }
        }
    }

    fn get_byte_data_from_view<'a>(buffer_data: &'a Vec<u8>, view: &View) -> (u32, u32) {
        (view.offset() as u32, view.length() as u32)
    }

    fn get_mesh_normals<'a>(
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
                todo!()
            }
        }
    }
}

// each Model has exactly one node tree
// in order to draw itself, it traverses this tree
// creating the required buffers
pub struct GModel {
    pub byte_data: Rc<Vec<u8>>,
    pub meshes: Vec<GMesh>,
    pub mesh_instances: Vec<u32>,
}

// used to gather the models
pub struct GScene {
    models: Vec<GModel>,
}

impl GScene {
    pub fn new<'a>(
        mut nodes: gltf::iter::Nodes<'a>,
        root_nodes_ids: Vec<usize>,
        buffer_data: Rc<Vec<u8>>,
    ) -> Result<Self, GltfErrors> {
        let meshes: Vec<GMesh> = get_meshes(nodes.clone(), &buffer_data)?;
        let mut models = Vec::with_capacity(root_nodes_ids.len());
        for id in root_nodes_ids.iter() {
            let root_node = nodes.nth(*id).ok_or(GltfErrors::VericesError(String::from(
                "could not identify any root nodes",
            )))?;
            let mut mesh_ids = Vec::<u32>::new();
            let mut mesh_instances = Vec::<u32>::new();
            (mesh_ids, mesh_instances) = find_meshes(&root_node, mesh_ids, mesh_instances);
            assert_eq!(mesh_ids.len(), mesh_instances.len());
            let child_meshes: Vec<GMesh> = meshes
                .iter()
                .filter_map(|m| {
                    if mesh_ids.contains(&m.index) {
                        return Some(*m);
                    }
                    None
                })
                .collect();
            models.push(GModel {
                byte_data: buffer_data.clone(),
                meshes: child_meshes,
                mesh_instances,
            });
        }
        Ok(Self { models })
    }
}

fn find_meshes(
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

fn get_meshes<'a>(
    nodes: gltf::iter::Nodes,
    buffer_data: &'a Vec<u8>,
) -> Result<Vec<GMesh>, GltfErrors> {
    let mut meshes = Vec::<GMesh>::new();
    for node in nodes {
        if let Some(mesh) = node.mesh() {
            if !has_mesh(&meshes, mesh.index() as u32) {
                let g_mesh = GMesh::new(&mesh, buffer_data)?;
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

pub fn load_gltf(dirname: &str) -> Result<Vec<GModel>, gltf::Error> {
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
    let buffer_data_rc = Rc::new(buffer_data);
    let scene: GScene = GScene::new(gltf.nodes(), root_nodes, buffer_data_rc)
        .map_err(|_| gltf::Error::UnsupportedScheme)?;

    Ok(vec![])
}
