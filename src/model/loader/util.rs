use std::{
    collections::HashMap,
    fs::{self, ReadDir},
    path::PathBuf,
    rc::Rc,
};

use base64::Engine;
use cgmath::SquareMatrix;
use gltf::{animation::Channel, Gltf};

use crate::model::{
    animation::{
        animation::SimpleAnimation,
        animation_node::{AnimationNode, NodeType},
    },
    loader::loader::GltfFileLoadError,
    model::{GModel, LocalTransform, ModelAnimationData},
    util::get_model_meshes,
};

struct NodeData {
    mesh_data: Option<ModelMeshData>,
    joint_data: Option<Vec<u32>>,
}
impl NodeData {
    fn new() -> Self {
        Self {
            mesh_data: Some(ModelMeshData::new()),
            joint_data: None,
        }
    }
}

struct ModelMeshData {
    joint_ids: Vec<usize>,
    joint_pose_transforms: Vec<[[f32; 4]; 4]>,
    mesh_ids: Vec<u32>,
    mesh_instances: Vec<u32>,
    mesh_transform_buckets: Vec<Vec<LocalTransform>>,
    node_to_lt_index_map: HashMap<usize, usize>,
    joint_to_joint_index_map: HashMap<usize, usize>,
}
impl ModelMeshData {
    fn new() -> Self {
        Self {
            node_to_lt_index_map: HashMap::new(),
            joint_to_joint_index_map: HashMap::new(),
            mesh_ids: Vec::new(),
            mesh_instances: Vec::new(),
            mesh_transform_buckets: Vec::new(),
            joint_ids: Vec::new(),
            joint_pose_transforms: Vec::new(),
        }
    }
}
#[allow(dead_code)]
pub(super) struct GltfBinaryExtras {
    animation: Option<PathBuf>,
    textures: Option<Vec<PathBuf>>,
}
pub(super) struct GltfFiles {
    pub(super) gltf: PathBuf,
    pub(super) bin: Option<PathBuf>,
}

use std::error::Error;

pub(super) fn decode_gltf_data_uri(uri: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    // Step 1: Check prefix
    const PREFIX: &str = "data:application/gltf-buffer;";
    if !uri.starts_with(PREFIX) {
        return Err("URI does not start with expected prefix".into());
    }

    // Step 2: Split metadata and encoded data
    let comma_index = uri.find(',').ok_or("No comma found in URI")?;
    let (meta, encoded_data) = uri[PREFIX.len()..].split_at(comma_index - PREFIX.len());
    let encoded_data = &encoded_data[1..]; // Skip the comma

    // Step 3: Match encoding and decode
    let decoded = match meta.trim() {
        "base64" => base64_decode(encoded_data)?,
        other => return Err(format!("Unsupported encoding: {}", other).into()),
    };

    Ok(decoded)
}

fn base64_decode(input: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    use base64::prelude::BASE64_STANDARD;
    // Uses standard lib base64 via experimental feature or stable crate if you choose
    let decoded = BASE64_STANDARD.decode(input)?; // Requires base64 crate
    Ok(decoded)
}

fn build_animation_node_tree(
    node: &gltf::Node,
    joint_ids: &Vec<usize>,
    has_mesh: &mut bool,
) -> AnimationNode {
    let children: Vec<AnimationNode> = node
        .children()
        .map(|child| build_animation_node_tree(&child, joint_ids, has_mesh))
        .collect();
    let node = AnimationNode::new(node, children, joint_ids);
    if node.node_type == NodeType::Mesh {
        *has_mesh = true;
    }
    node
}

pub(super) fn load_models_from_gltf<'a>(
    root_nodes_ids: Vec<usize>,
    joint_ids: &Vec<usize>,
    nodes: gltf::iter::Nodes<'a>,
    animations: &gltf::iter::Animations,
    buffers: &gltf::iter::Buffers,
) -> (Vec<GModel>, Vec<LocalTransform>) {
    let nodes: Vec<_> = nodes.collect(); // collect the data into a vec so it can be indexed
    let mut models = Vec::<GModel>::with_capacity(root_nodes_ids.len());
    let mut local_transform_data = Vec::<LocalTransform>::new();
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
            joint_ids,
        );
        println!("JOINTS: {:?}", model_mesh_data.joint_to_joint_index_map);

        let buffer_offsets: Vec<u64> = get_buffer_offsets(buffers);
        // get a animation node trees
        let (maybe_animation_node, animation_count, mesh_animations) = load_animations(
            &root_node,
            animations,
            &model_mesh_data.joint_ids,
            &buffer_offsets,
        );

        // instantiate meshes, instantiate model
        let meshes = get_model_meshes(&model_mesh_data.mesh_ids, &nodes, &buffer_offsets)
            .expect("meshes for this model");
        let mi_len = model_mesh_data.mesh_instances.len().clone();
        let gmodel_animation_data: Option<ModelAnimationData> = match maybe_animation_node {
            Some(animation_node) => {
                let joint_count = model_mesh_data.joint_to_joint_index_map.len().clone();
                Some(ModelAnimationData {
                    mesh_animations,
                    animation_count,
                    model_index: models.len(),
                    animation_node: Rc::new(animation_node),
                    node_to_lt_index: model_mesh_data.node_to_lt_index_map,
                    joint_to_joint_index: model_mesh_data.joint_to_joint_index_map,
                    joint_count,
                })
            }

            None => None,
        };
        println!(
            "ANIMATION DATA FOR MODEL {}: {:?}",
            models.len(),
            gmodel_animation_data,
        );
        let g_model = GModel::new(
            meshes,
            model_mesh_data.mesh_instances,
            gmodel_animation_data,
        );

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
    (models, local_transform_data)
}

fn get_buffer_offsets(buffers: &gltf::iter::Buffers) -> Vec<u64> {
    let mut buffer_offsets = Vec::<u64>::new();
    let mut last_buffer_size = 0;
    for buffer in buffers.clone().into_iter() {
        buffer_offsets.push(last_buffer_size);
        last_buffer_size += buffer.length() as u64;
    }
    buffer_offsets
}

// for each distinct animation on a single model
// create an [AniamtionNode] tree, and populate each node with 0 or more
// sets of samplers that appy to the that node
fn load_animations(
    root_node: &gltf::Node,
    animations: &gltf::iter::Animations,
    joint_ids: &Vec<usize>,
    buffer_offsets: &Vec<u64>,
) -> (Option<AnimationNode>, usize, Vec<usize>) {
    let mut animation_count = 0;
    let mut has_mesh = false;
    let mut mesh_animations: Vec<usize> = Vec::new();
    let mut animation_node = build_animation_node_tree(root_node, joint_ids, &mut has_mesh);
    let mut is_animated = false;
    for animation in animations.clone().into_iter() {
        let channels: Vec<Channel> = animation.channels().into_iter().collect();
        if animation_node.attach_sampler_sets(&channels, &mut is_animated, buffer_offsets) {
            animation_count += 1;
        }
        if has_mesh && is_animated {
            mesh_animations.push(animation_count - 1);
            has_mesh = false;
        }
    }
    // the model represented by the root node is considered animated if,
    // for any of the gltf animations, at least one of the nodes in its tree
    // has an associated channel
    // if not animated, the AnimationNode can be discarded
    if is_animated {
        return (Some(animation_node), animation_count, mesh_animations);
    } else {
        return (None, 0, mesh_animations);
    }
}

fn find_model_meshes(
    root_node: &gltf::Node,
    base_translation: cgmath::Matrix4<f32>,
    mut model_mesh_data: ModelMeshData,
    joint_ids: &Vec<usize>,
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
    } else {
        match joint_ids.binary_search(&root_node.index()) {
            Ok(_) => {
                model_mesh_data
                    .joint_to_joint_index_map
                    .insert(root_node.index(), model_mesh_data.joint_ids.len());
                model_mesh_data.joint_ids.push(root_node.index());
                model_mesh_data.joint_pose_transforms.push(new_trans.into());
            }
            Err(_) => {}
        }
    }

    for child_node in root_node.children() {
        model_mesh_data = find_model_meshes(&child_node, new_trans, model_mesh_data, joint_ids);
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
        }
    }
    Ok(GltfFiles {
        gltf: gltf_file,
        bin: bin_file,
    })
}
