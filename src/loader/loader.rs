use std::path::PathBuf;

use super::util::*;
use crate::model::model::{GMesh2, LocalTransform};
use gltf::Gltf;

enum ModelFileType {
    GLTF,
    OTHER,
}

trait ModelLoader<'a> {
    fn get_models(file_type: ModelFileType, dir_path: &'a str) -> Vec<GModel2>;
}

pub struct GltfLoader;

impl<'a> ModelLoader<'a> for GltfLoader {
    fn get_models(file_type: ModelFileType, dir_path: &'a str) -> Vec<GModel2> {
        vec![]
    }
}

// could abstract this even further by requiring a function which returns some kind of
// box<dyn modelData>, but that seems like overkill for now.
impl GltfLoader {
    /// process the given dir to get one gltf file, one binary file, and optional extra files
    pub fn load_gltf2(dir_name: &str) -> Result<GltfData, GltfFileLoadError> {
        let dir_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("res")
            .join(dir_name);
        if !dir_path.is_dir() {
            return Err(GltfFileLoadError::IoErr(
                std::io::ErrorKind::NotFound.into(),
            ));
        }
        let files = get_data_files(dir_path)?;
        let gltf = Gltf::open(&files.0).map_err(|e| GltfFileLoadError::GltfError(e))?;
        let binary_data = std::fs::read(files.1).map_err(|e| GltfFileLoadError::IoErr(e))?;
        let root_node_ids = get_root_nodes(&gltf).map_err(|e| GltfFileLoadError::GltfError(e))?;
        let nodes = gltf.nodes();
        let (models, local_transforms) = load_models_from_gltf(root_node_ids, nodes);
        let gltf_data = GltfData {
            models,
            binary_data,
            local_transforms,
        };

        Ok(gltf_data)
    }
}

pub struct GltfData {
    pub models: Vec<GModel2>,
    pub binary_data: Vec<u8>,
    pub local_transforms: Vec<LocalTransform>,
}

pub struct AnimationData;

pub struct GModel2 {
    pub animation_data: Option<AnimationData>,
    pub meshes: Vec<GMesh2>,
    pub mesh_instances: Vec<u32>,
}

impl GModel2 {
    pub fn new(
        animation_data: Option<AnimationData>,
        meshes: Vec<GMesh2>,
        mesh_instances: Vec<u32>,
    ) -> Self {
        Self {
            animation_data,
            meshes,
            mesh_instances,
        }
    }
}
