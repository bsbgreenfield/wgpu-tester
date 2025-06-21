use std::path::PathBuf;

use crate::model::{
    animation::animation_controller::SimpleAnimation,
    loader::util::{get_data_files, get_root_nodes, load_models_from_gltf},
    model::{GModel, LocalTransform},
};
use gltf::Gltf;

pub struct GltfLoader;

// could abstract this even further by requiring a function which returns some kind of
// box<dyn modelData>, but that seems like overkill for now.
impl GltfLoader {
    /// process the given dir to get one gltf file, one binary file, and optional extra files
    pub fn load_gltf(dir_name: &str) -> Result<GltfData, GltfFileLoadError> {
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
        let (models, local_transforms, simple_animations) =
            load_models_from_gltf(root_node_ids, nodes, &gltf.animations());
        let gltf_data = GltfData {
            models,
            binary_data,
            local_transforms,
            simple_animations,
        };

        Ok(gltf_data)
    }
}

pub struct GltfData {
    pub models: Vec<GModel>,
    pub binary_data: Vec<u8>,
    pub local_transforms: Vec<LocalTransform>,
    pub simple_animations: Vec<SimpleAnimation>,
}

#[derive(Debug)]
pub enum GltfFileLoadError {
    NoGltfFile,
    NoBinaryFile,
    MultipleBinaryFiles,
    IoErr(std::io::Error),
    GltfError(gltf::Error),
    BadFile,
}
