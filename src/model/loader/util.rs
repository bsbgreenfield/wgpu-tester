use std::{
    fs::{self, ReadDir},
    path::PathBuf,
};

use cgmath::SquareMatrix;
use gltf::{accessor::DataType, animation::Channel, Gltf, Node};

use crate::model::{
    animation::{GltfAnimationComponentData, GltfAnimationData, Interpolation},
    loader::loader::GltfFileLoadError,
    model::{GModel, LocalTransform},
    util::get_model_meshes,
};

#[derive(Debug)]
enum NodeType {
    Node,
    Mesh,
}
struct GNode {
    children: Vec<GNode>,
    node_type: NodeType,
    node_id: usize,
    mesh_id: Option<usize>,
}
impl GNode {
    fn new(node: &Node, children: Vec<GNode>) -> Self {
        match node.mesh() {
            Some(mesh) => GNode {
                children,
                node_type: NodeType::Mesh,
                node_id: node.index(),
                mesh_id: Some(mesh.index()),
            },
            None => GNode {
                children,
                node_type: NodeType::Node,
                node_id: node.index(),
                mesh_id: None,
            },
        }
    }
}
struct ModelMeshData {
    mesh_ids: Vec<u32>,
    mesh_instances: Vec<u32>,
    transformation_matrices: Vec<LocalTransform>,
    gnode: Option<GNode>,
}
impl ModelMeshData {
    fn new() -> Self {
        Self {
            mesh_ids: Vec::new(),
            mesh_instances: Vec::new(),
            transformation_matrices: Vec::new(),
            gnode: None,
        }
    }
}

#[allow(dead_code)]
pub(super) struct GltfBinaryExtras {
    animation: Option<PathBuf>,
    textures: Option<Vec<PathBuf>>,
}
pub(super) type GltfFiles = (PathBuf, PathBuf, Option<GltfBinaryExtras>);

fn build_node_tree(node: &gltf::Node) -> GNode {
    let children: Vec<GNode> = node
        .children()
        .map(|child| build_node_tree(&child))
        .collect();
    GNode::new(node, children)
}

pub(super) fn load_models_from_gltf<'a>(
    root_nodes_ids: Vec<usize>,
    nodes: gltf::iter::Nodes<'a>,
    animations: &gltf::iter::Animations,
) -> (Vec<GModel>, Vec<LocalTransform>) {
    let nodes: Vec<_> = nodes.collect(); // collect the data into a vec so it can be indexed
    let mut models = Vec::<GModel>::with_capacity(root_nodes_ids.len());
    let mut local_transform_data = Vec::<LocalTransform>::new();
    for rid in root_nodes_ids.iter() {
        let mut model_mesh_data = ModelMeshData::new();
        let root_node: &gltf::Node<'a> = &nodes[*rid];
        let gnode = build_node_tree(root_node); // for processing animations

        // animations
        let animation_data: Option<Vec<GltfAnimationData>> = load_animations(&gnode, animations);
        match animation_data {
            Some(a) => println!("{:?}", a),
            None => println!("no animations forund for rn {}", rid),
        }

        // mesh data
        model_mesh_data = find_model_meshes(
            root_node,
            cgmath::Matrix4::<f32>::identity(),
            model_mesh_data,
        );
        model_mesh_data.gnode = Some(gnode);

        // instantiate meshes, instantiate model
        let meshes =
            get_model_meshes(&model_mesh_data.mesh_ids, &nodes).expect("meshes for this model");
        let g_model = GModel::new(None, meshes, model_mesh_data.mesh_instances);

        // add the local transformations to the running vec
        local_transform_data.extend(model_mesh_data.transformation_matrices);

        models.push(g_model);
    }
    (models, local_transform_data)
}

pub(super) fn load_animations(
    gnode: &GNode,
    animations: &gltf::iter::Animations,
) -> Option<Vec<GltfAnimationData>> {
    let mut animations_data: Vec<GltfAnimationData> = Vec::new();
    for animation in animations.clone().into_iter() {
        let mut gltf_animation_components = Vec::<GltfAnimationComponentData>::new();
        // i dont understand how the gltf crate expects me to use the normal channels iter
        let channels: Vec<Channel> = animation.channels().into_iter().collect();
        for channel in channels.iter() {
            let mut mesh_ids = Vec::<usize>::new();
            let node_id: usize = channel.target().node().index();
            println!("Searching for meshes that correspond to node {}", node_id);
            find_meshes_for_animation(&gnode, node_id, &mut mesh_ids);
            if mesh_ids.is_empty() {
                return None;
            }
            gltf_animation_components.push(GltfAnimationComponentData {
                mesh_ids: mesh_ids,
                times_data: get_animation_times(&channel.sampler().input()),
                transforms_data: get_animation_transforms(&channel.sampler().output()),
                interpolation: Interpolation::from(channel.sampler().interpolation()),
            });
        }
        animations_data.push(GltfAnimationData {
            animation_components: gltf_animation_components,
        });
    }
    Some(animations_data)
}

fn get_animation_times(times_accessor: &gltf::Accessor) -> (usize, usize) {
    assert_eq!(times_accessor.data_type(), DataType::F32);
    let length = times_accessor.count() * 4;
    let offset = times_accessor.offset() + (times_accessor.view().unwrap().offset());
    (offset, length)
}

fn get_animation_transforms(transforms_accessor: &gltf::Accessor) -> (usize, usize) {
    assert_eq!(transforms_accessor.data_type(), DataType::F32);
    let length = transforms_accessor.count() * 16;
    let offset = transforms_accessor.offset() + (transforms_accessor.view().unwrap().offset());
    (offset, length)
}

fn find_meshes_for_animation(gnode: &GNode, node_id: usize, mesh_ids: &mut Vec<usize>) {
    if let Some(parent_node) = find_node(gnode, node_id) {
        println!("found node {}", parent_node.node_id);
        find_meshes_under_node(parent_node, mesh_ids);
    } else {
        return;
    }
}
fn find_node(gnode: &GNode, node_id: usize) -> Option<&GNode> {
    println!("visiting node {}", gnode.node_id);
    if gnode.node_id == node_id {
        return Some(gnode);
    }
    for child in gnode.children.iter() {
        return find_node(child, node_id);
    }
    None
}

fn find_meshes_under_node(gnode: &GNode, mesh_ids: &mut Vec<usize>) {
    match gnode.node_type {
        NodeType::Mesh => mesh_ids.push(gnode.mesh_id.unwrap()),
        NodeType::Node => {
            for child in gnode.children.iter() {
                find_meshes_under_node(child, mesh_ids);
            }
        }
    }
}

/// recurse through the root node to get data on transformations, mesh indices, and mesh
/// instances
fn find_model_meshes(
    root_node: &gltf::Node,
    mut base_translation: cgmath::Matrix4<f32>,
    mut model_mesh_data: ModelMeshData,
) -> ModelMeshData {
    'block: {
        let cg_trans = cgmath::Matrix4::<f32>::from(root_node.transform().matrix());
        base_translation = base_translation * cg_trans;
        if let Some(mesh) = root_node.mesh() {
            // this is an instance of a mesh. Push the current base translation
            let local_transform: LocalTransform = LocalTransform {
                model_index: 0,
                transform_matrix: base_translation.into(),
            };
            model_mesh_data
                .transformation_matrices
                .push(local_transform);
            // check mesh_ids to see if this particular mesh has already been added, if so, the index
            // of the match is equal to the index within mesh_instances that we want to increment by 1
            for (idx, m) in model_mesh_data.mesh_ids.iter().enumerate() {
                if *m == mesh.index() as u32 {
                    model_mesh_data.mesh_instances[idx] += 1;
                    break 'block;
                }
            }
            // this mesh has not been added: append to both vecs
            model_mesh_data.mesh_ids.push(mesh.index() as u32);
            model_mesh_data.mesh_instances.push(1);
        }
    }
    for child_node in root_node.children() {
        model_mesh_data = find_model_meshes(&child_node, base_translation, model_mesh_data);
    }
    model_mesh_data
}

pub(super) fn get_root_nodes(gltf: &Gltf) -> Result<Vec<usize>, gltf::Error> {
    let scene = gltf.scenes().next().ok_or(gltf::Error::UnsupportedScheme)?;
    let mesh_node_iter = scene
        .nodes()
        .filter(|n| n.mesh().is_some() || n.children().len() != 0);
    Ok(mesh_node_iter.map(|n| n.index()).collect())
}
pub(super) fn get_data_files(dir_path: PathBuf) -> Result<GltfFiles, GltfFileLoadError> {
    let gltf_file: PathBuf;
    let mut bin_file: Option<PathBuf> = None;
    let mut entries: ReadDir = fs::read_dir(&dir_path).map_err(|e| GltfFileLoadError::IoErr(e))?;

    // step 1: grab the main gltf file
    let gltf_entry = entries
        .find(|entry| {
            entry.as_ref().is_ok_and(|dir_entry| {
                dir_entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.to_str().is_some_and(|ext_str| ext_str == "gltf"))
            })
        })
        .ok_or(GltfFileLoadError::NoGltfFile)? // if find return none, return this err
        .map_err(|_| GltfFileLoadError::BadFile)?; // if find returns an Err, map it to BadFile
    gltf_file = gltf_entry.path();

    // step 2: assert that there is only a single binary file and grab it
    let entries: ReadDir = fs::read_dir(&dir_path).map_err(|e| GltfFileLoadError::IoErr(e))?;
    'outer: for entry in entries {
        if bin_file.is_some() {
            return Err(GltfFileLoadError::MultipleBinaryFiles);
        }
        if let Ok(dir_entry) = entry {
            if dir_entry
                .path()
                .extension()
                .is_some_and(|ext| ext.to_str().is_some_and(|ext_str| ext_str == "bin"))
            {
                bin_file = Some(dir_entry.path());
                break 'outer;
            }
        } else {
            return Err(GltfFileLoadError::BadFile);
        }
    }
    let bin = bin_file.expect("bin");
    Ok((gltf_file, bin, None))
}
