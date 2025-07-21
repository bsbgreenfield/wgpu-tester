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
    animation::animation_node::{AnimationNode, NodeType},
    loader::loader::{GltfData, GltfFileLoadError, ModelPrimitiveData},
    model::{GModel, JointAnimationData, LocalTransform, MeshAnimationData, ModelAnimationData},
    util::{copy_binary_data_from_gltf, get_model_meshes, AttributeType},
};

struct ModelData {
    mesh_data: ModelMeshData,
    joint_data: ModelJointData,
}

struct ModelJointData {
    joint_ids: Vec<usize>,
    joint_pose_transforms: Vec<[[f32; 4]; 4]>,
    joint_to_joint_index_map: HashMap<usize, usize>,
}
struct ModelMeshData {
    mesh_ids: Vec<u32>,
    mesh_instances: Vec<u32>,
    mesh_transform_buckets: Vec<Vec<LocalTransform>>,
    node_to_lt_index_map: HashMap<usize, usize>,
}
impl Default for ModelMeshData {
    fn default() -> Self {
        Self {
            mesh_ids: Default::default(),
            mesh_transform_buckets: Default::default(),
            mesh_instances: Default::default(),
            node_to_lt_index_map: Default::default(),
        }
    }
}
impl Default for ModelJointData {
    fn default() -> Self {
        Self {
            joint_ids: Default::default(),
            joint_to_joint_index_map: Default::default(),
            joint_pose_transforms: Default::default(),
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
    joint_to_joint_indices: &HashMap<usize, usize>,
    has_mesh: &mut bool,
) -> AnimationNode {
    let children: Vec<AnimationNode> = node
        .children()
        .map(|child| build_animation_node_tree(&child, joint_to_joint_indices, has_mesh))
        .collect();
    let node = AnimationNode::new(node, children, joint_to_joint_indices);
    if node.node_type == NodeType::Mesh {
        *has_mesh = true;
    }
    node
}

fn get_inverse_bind_matrices(
    skin: &gltf::Skin,
    buffer_offsets: &Vec<u64>,
    main_buffer_data: &Vec<u8>,
) -> (usize, Vec<cgmath::Matrix4<f32>>) {
    let ibm_accessor = skin
        .inverse_bind_matrices()
        .expect("should be an accessor for ibms");
    let ibm_vec: Vec<[[f32; 4]; 4]> = bytemuck::cast_slice(
        &copy_binary_data_from_gltf(
            &ibm_accessor,
            AttributeType::IBMS,
            buffer_offsets,
            main_buffer_data,
        )
        .expect("should be ibm data"),
    )
    .to_vec();
    let ibm_cgmath: Vec<cgmath::Matrix4<f32>> = ibm_vec
        .iter()
        .map(|ibm| cgmath::Matrix4::<f32>::from(*ibm))
        .collect();
    (skin.index(), ibm_cgmath)
}

pub(super) fn load_models_from_gltf<'a>(
    root_nodes_ids: Vec<usize>,
    nodes: gltf::iter::Nodes<'a>,
    animations: &gltf::iter::Animations,
    buffers: &gltf::iter::Buffers,
    skins: &gltf::iter::Skins,
    main_buffer_data: Vec<u8>,
) -> GltfData {
    let nodes: Vec<_> = nodes.collect(); // collect the data into a vec so it can be indexed
    let mut models = Vec::<GModel>::with_capacity(root_nodes_ids.len());
    let mut model_primitive_data: Vec<ModelPrimitiveData> = Vec::new();
    let mut local_transform_data = Vec::<LocalTransform>::new();
    let mut joint_transform_data = Vec::<[[f32; 4]; 4]>::new();
    let mut joint_ids = Vec::<usize>::new();
    let mut skin_ibms: HashMap<usize, Vec<cgmath::Matrix4<f32>>> =
        HashMap::with_capacity(skins.len());
    let buffer_offsets: Vec<u64> = get_buffer_offsets(buffers);
    for skin in skins.clone().into_iter() {
        let (skin_idx, ibms) = get_inverse_bind_matrices(&skin, &buffer_offsets, &main_buffer_data);
        skin_ibms.insert(skin_idx, ibms);
        for joint in skin.joints().into_iter() {
            joint_ids.push(joint.index());
        }
    }
    for rid in root_nodes_ids.iter() {
        let mut model_data: ModelData = ModelData {
            mesh_data: ModelMeshData::default(),
            joint_data: ModelJointData::default(),
        };

        let root_node: &gltf::Node<'a> = &nodes[*rid];
        if root_node.camera().is_some() {
            continue;
        }

        model_data = get_model_data(
            root_node,
            cgmath::Matrix4::<f32>::identity(),
            model_data,
            &joint_ids,
            &skin_ibms,
        );

        // get a animation node trees
        let (maybe_animation_node, animation_count, mesh_animations) = load_animations(
            &root_node,
            animations,
            &model_data.joint_data.joint_to_joint_index_map,
            &buffer_offsets,
            &main_buffer_data,
        );

        // instantiate meshes, instantiate model
        let (meshes, primitive_data) = get_model_meshes(
            &model_data.mesh_data.mesh_ids,
            &nodes,
            &buffer_offsets,
            &main_buffer_data,
        )
        .expect("meshes for this model");
        model_primitive_data.push(ModelPrimitiveData {
            model_id: *rid,
            primitive_data,
        });

        let gmodel_animation_data: Option<ModelAnimationData> = match maybe_animation_node {
            Some(animation_node) => {
                let joint_count = model_data.joint_data.joint_to_joint_index_map.len().clone();
                let joint_indices: Vec<usize> = model_data
                    .joint_data
                    .joint_to_joint_index_map
                    .clone()
                    .into_values()
                    .collect();
                Some(ModelAnimationData {
                    animation_count,
                    model_index: models.len(),
                    animation_node: Rc::new(animation_node),
                    is_skeletal: joint_count > 0,
                    mesh_animation_data: MeshAnimationData {
                        mesh_animations,
                        node_to_lt_index: model_data.mesh_data.node_to_lt_index_map,
                    },
                    joint_animation_data: JointAnimationData {
                        joint_to_joint_index: model_data.joint_data.joint_to_joint_index_map,
                        joint_count,
                        joint_indices,
                    },
                })
            }

            None => None,
        };
        let g_model = GModel::new(
            *rid,
            meshes,
            model_data.mesh_data.mesh_instances,
            gmodel_animation_data,
        );

        assert_eq!(
            model_data.mesh_data.mesh_transform_buckets.len(),
            model_data.mesh_data.mesh_ids.len()
        );
        // add the local transformations to the running vec
        for i in 0..model_data.mesh_data.mesh_ids.len() {
            // TODO: avoid copying the data
            local_transform_data.extend(model_data.mesh_data.mesh_transform_buckets[i].clone());
        }
        joint_transform_data.extend(model_data.joint_data.joint_pose_transforms);

        models.push(g_model);
    }
    GltfData {
        models,
        binary_data: main_buffer_data,
        model_primitive_data,
        local_transforms: local_transform_data,
        joint_transforms: joint_transform_data,
        skin_ibms,
    }
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
    joint_to_joint_indices: &HashMap<usize, usize>,
    buffer_offsets: &Vec<u64>,
    main_buffer_data: &Vec<u8>,
) -> (Option<AnimationNode>, usize, Vec<usize>) {
    let mut animation_count = 0;
    let mut has_mesh = false;
    let mut mesh_animations: Vec<usize> = Vec::new();
    let mut animation_node =
        build_animation_node_tree(root_node, joint_to_joint_indices, &mut has_mesh);
    let mut is_animated = false;
    for animation in animations.clone().into_iter() {
        let channels: Vec<Channel> = animation.channels().into_iter().collect();
        if animation_node.attach_sampler_sets(
            &channels,
            &mut is_animated,
            buffer_offsets,
            main_buffer_data,
        ) {
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

fn get_model_data(
    root_node: &gltf::Node,
    base_translation: cgmath::Matrix4<f32>,
    mut model_data: ModelData,
    joint_ids: &Vec<usize>,
    skin_ibms: &HashMap<usize, Vec<cgmath::Matrix4<f32>>>,
) -> ModelData {
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
            for (idx, m) in model_data.mesh_data.mesh_ids.iter().enumerate() {
                transform_index += model_data.mesh_data.mesh_transform_buckets[idx].len();

                if *m == mesh.index() as u32 {
                    model_data.mesh_data.mesh_instances[idx] += 1;
                    model_data.mesh_data.mesh_transform_buckets[idx].push(local_transform);
                    break 'block;
                }
            }
            // if we get here, then the mesh is totally new
            model_data.mesh_data.mesh_ids.push(mesh.index() as u32);
            model_data.mesh_data.mesh_instances.push(1);
            model_data
                .mesh_data
                .mesh_transform_buckets
                .push(vec![local_transform]);

            //
        }
        let unique_kv = model_data
            .mesh_data
            .node_to_lt_index_map
            .insert(root_node.index(), transform_index);
        assert!(unique_kv.is_none()); // each node should be unique
    } else {
        match joint_ids
            .iter()
            .position(|joint_id| joint_id == &root_node.index())
        {
            Some(joint_index) => {
                model_data
                    .joint_data
                    .joint_to_joint_index_map
                    .insert(root_node.index(), joint_index);
                model_data.joint_data.joint_ids.push(root_node.index());
                let ibm: cgmath::Matrix4<f32> =
                    skin_ibms.get(&0).unwrap()[joint_ids.len() - 1].into();
                model_data
                    .joint_data
                    .joint_pose_transforms
                    .push((cgmath::Matrix4::<f32>::identity()).into());
            }
            None => {}
        }
    }

    for child_node in root_node.children() {
        model_data = get_model_data(&child_node, new_trans, model_data, joint_ids, skin_ibms);
    }
    model_data
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
