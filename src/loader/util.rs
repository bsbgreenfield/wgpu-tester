use std::{
    fs::{self, ReadDir},
    path::PathBuf,
};

use cgmath::SquareMatrix;
use gltf::Gltf;

use crate::{
    loader::loader::GModel2,
    model::{
        model::{GMesh2, LocalTransform},
        util::GltfErrors,
        vertex::ModelVertex,
    },
};

struct ModelMeshData {
    pub mesh_ids: Vec<u32>,
    pub mesh_instances: Vec<u32>,
    pub transformation_matrices: Vec<LocalTransform>,
}
impl ModelMeshData {
    fn new() -> Self {
        Self {
            mesh_ids: Vec::new(),
            mesh_instances: Vec::new(),
            transformation_matrices: Vec::new(),
        }
    }
}

struct ModelBufferData {
    pub vertex_buf: Vec<ModelVertex>,
    pub index_buf: Vec<u16>,
    pub index_ranges: Vec<std::ops::Range<usize>>,
}
impl ModelBufferData {
    fn new() -> Self {
        Self {
            vertex_buf: Vec::new(),
            index_buf: Vec::new(),
            index_ranges: Vec::new(),
        }
    }
}
pub(super) struct GltfBinaryExtras {
    animation: Option<PathBuf>,
    textures: Option<Vec<PathBuf>>,
}
pub(super) type GltfFiles = (PathBuf, PathBuf, Option<GltfBinaryExtras>);

#[derive(Debug)]
pub enum GltfFileLoadError {
    NoGltfFile,
    NoBinaryFile,
    MultipleBinaryFiles,
    IoErr(std::io::Error),
    GltfError(gltf::Error),
    BadFile,
}
pub(super) fn load_models_from_gltf<'a>(
    root_nodes_ids: Vec<usize>,
    nodes: gltf::iter::Nodes<'a>,
) -> (Vec<GModel2>, Vec<LocalTransform>) {
    let nodes: Vec<_> = nodes.collect(); // collect the data into a vec so it can be indexed
    let mut models = Vec::<GModel2>::with_capacity(root_nodes_ids.len());
    let mut local_transform_data = Vec::<LocalTransform>::new();
    for rid in root_nodes_ids.iter() {
        let mut model_mesh_data = ModelMeshData::new();
        let root_node: &gltf::Node<'a> = &nodes[*rid];
        model_mesh_data = find_model_meshes(
            root_node,
            cgmath::Matrix4::<f32>::identity(),
            model_mesh_data,
        );
        let meshes =
            get_model_meshes(&model_mesh_data.mesh_ids, &nodes).expect("meshes for this model");
        let g_model = GModel2::new(None, meshes, model_mesh_data.mesh_instances);
        local_transform_data.extend(model_mesh_data.transformation_matrices);
        models.push(g_model);
    }
    (models, local_transform_data)
}

fn get_model_meshes(
    mesh_ids: &Vec<u32>,
    nodes: &Vec<gltf::Node>,
) -> Result<Vec<GMesh2>, GltfErrors> {
    let mut meshes = Vec::<GMesh2>::new();
    for mesh_id in mesh_ids.iter() {
        let mesh = nodes
            .iter()
            .find(|n| n.mesh().is_some() && n.mesh().unwrap().index() as u32 == *mesh_id)
            .unwrap()
            .mesh()
            .unwrap();
        let g_mesh = GMesh2::new(&mesh)?;
        meshes.push(g_mesh);
    }

    Ok(meshes)
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
        model_mesh_data = find_model_meshes(&child_node, base_translation, model_mesh_data)
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
    let mut gltf_file: Option<PathBuf> = None;
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
    gltf_file = Some(gltf_entry.path());

    // step 2: assert that there is only a single binary file and grab it
    let entries: ReadDir = fs::read_dir(&dir_path).map_err(|e| GltfFileLoadError::IoErr(e))?;
    for entry in entries {
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
            }
        } else {
            return Err(GltfFileLoadError::BadFile);
        }
    }
    Ok((gltf_file.unwrap(), bin_file.unwrap(), None))
}
