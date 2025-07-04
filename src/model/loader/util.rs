use std::{
    collections::HashMap,
    fs::{self, ReadDir},
    path::PathBuf,
};

use cgmath::SquareMatrix;
use gltf::{animation::Channel, Gltf};

use crate::model::{
    animation::{animation_controller::SimpleAnimation, animation_node::AnimationNode},
    loader::loader::GltfFileLoadError,
    model::{GModel, LocalTransform},
    util::get_model_meshes,
};

struct ModelMeshData {
    mesh_ids: Vec<u32>,
    mesh_instances: Vec<u32>,
    mesh_transform_buckets: Vec<Vec<LocalTransform>>,
    node_to_lt_index_map: HashMap<usize, usize>,
}
impl ModelMeshData {
    fn new() -> Self {
        Self {
            node_to_lt_index_map: HashMap::new(),
            mesh_ids: Vec::new(),
            mesh_instances: Vec::new(),
            mesh_transform_buckets: Vec::new(),
        }
    }
}
#[allow(dead_code)]
pub(super) struct GltfBinaryExtras {
    animation: Option<PathBuf>,
    textures: Option<Vec<PathBuf>>,
}
pub(super) type GltfFiles = (PathBuf, PathBuf, Option<GltfBinaryExtras>);

fn build_animation_node_tree(node: &gltf::Node) -> AnimationNode {
    let children: Vec<AnimationNode> = node
        .children()
        .map(|child| build_animation_node_tree(&child))
        .collect();
    AnimationNode::new(node, children)
}

pub(super) fn load_models_from_gltf<'a>(
    root_nodes_ids: Vec<usize>,
    nodes: gltf::iter::Nodes<'a>,
    animations: &gltf::iter::Animations,
) -> (Vec<GModel>, Vec<LocalTransform>, Vec<SimpleAnimation>) {
    let nodes: Vec<_> = nodes.collect(); // collect the data into a vec so it can be indexed
    let mut models = Vec::<GModel>::with_capacity(root_nodes_ids.len());
    let mut local_transform_data = Vec::<LocalTransform>::new();
    let mut simple_animations: Vec<SimpleAnimation> = Vec::new();
    for rid in root_nodes_ids.iter() {
        let mut model_mesh_data = ModelMeshData::new();
        let root_node: &gltf::Node<'a> = &nodes[*rid];
        if root_node.camera().is_some() {
            continue;
        }

        // mesh data
        model_mesh_data = find_model_meshes(
            root_node,
            cgmath::Matrix4::<f32>::identity(),
            model_mesh_data,
        );

        // get a animation node trees
        let maybe_animation_node = load_animations(&root_node, animations);
        // create a new SimpleAnimation
        // models are indexed by the order in which they are aded to the scenes
        // [models] field.
        if let Some(animation_node) = maybe_animation_node {
            simple_animations.push(SimpleAnimation::new(
                animation_node,
                models.len(),
                model_mesh_data.node_to_lt_index_map,
            ));
        }

        // instantiate meshes, instantiate model
        let meshes =
            get_model_meshes(&model_mesh_data.mesh_ids, &nodes).expect("meshes for this model");
        let mi_len = model_mesh_data.mesh_instances.len().clone();
        let g_model = GModel::new(meshes, model_mesh_data.mesh_instances);

        assert_eq!(model_mesh_data.mesh_ids.len(), mi_len,);
        assert_eq!(
            model_mesh_data.mesh_transform_buckets.len(),
            model_mesh_data.mesh_ids.len()
        );
        // add the local transformations to the running vec
        for i in 0..model_mesh_data.mesh_ids.len() {
            // TODO: avoid copying the data
            local_transform_data.extend(model_mesh_data.mesh_transform_buckets[i].clone());
        }

        models.push(g_model);
    }
    (models, local_transform_data, simple_animations)
}

// for each distinct animation on a single model
// create an [AniamtionNode] tree, and populate each node with 0 or more
// sets of samplers that appy to the that node
fn load_animations(
    root_node: &gltf::Node,
    animations: &gltf::iter::Animations,
) -> Option<AnimationNode> {
    let mut animation_node = build_animation_node_tree(root_node);
    let mut is_animated = false;
    for animation in animations.clone().into_iter() {
        let channels: Vec<Channel> = animation.channels().into_iter().collect();
        animation_node.attach_sampler_sets(&channels, &mut is_animated);
    }
    // the model represented by the root node is considered animated if,
    // for any of the gltf animations, at least one of the nodes in its tree
    // has an associated channel
    // if not animated, the AnimationNode can be discarded
    if is_animated {
        return Some(animation_node);
    } else {
        return None;
    }
}

fn find_model_meshes(
    root_node: &gltf::Node,
    base_translation: cgmath::Matrix4<f32>,
    mut model_mesh_data: ModelMeshData,
) -> ModelMeshData {
    let cg_trans = cgmath::Matrix4::<f32>::from(root_node.transform().matrix());
    let new_trans = base_translation * cg_trans;
    if let Some(mesh) = root_node.mesh() {
        let mut transform_index = 0;
        'block: {
            // create the transform
            let local_transform: LocalTransform = LocalTransform {
                model_index: 0,
                transform_matrix: new_trans.into(),
            };
            // check if this mesh has already been added, if so
            // add this mesh transform to the end of the bucket at index
            for (idx, m) in model_mesh_data.mesh_ids.iter().enumerate() {
                transform_index += model_mesh_data.mesh_transform_buckets[idx].len();

                if *m == mesh.index() as u32 {
                    model_mesh_data.mesh_instances[idx] += 1;
                    model_mesh_data.mesh_transform_buckets[idx].push(local_transform);
                    break 'block;
                }
            }
            // if we get here, then the mesh is totally new
            model_mesh_data.mesh_ids.push(mesh.index() as u32);
            model_mesh_data.mesh_instances.push(1);
            model_mesh_data
                .mesh_transform_buckets
                .push(vec![local_transform]);

            //
        }
        let unique_kv = model_mesh_data
            .node_to_lt_index_map
            .insert(root_node.index(), transform_index);
        assert!(unique_kv.is_none()); // each node should be unique
    }
    for child_node in root_node.children() {
        model_mesh_data = find_model_meshes(&child_node, new_trans, model_mesh_data);
    }
    model_mesh_data
}

pub(super) fn get_root_nodes(gltf: &Gltf) -> Result<Vec<usize>, gltf::Error> {
    println!("here");
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
