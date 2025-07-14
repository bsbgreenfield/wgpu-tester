use std::path::PathBuf;

use crate::model::{
    animation::animation::SimpleAnimation,
    loader::util::{decode_gltf_data_uri, get_data_files, get_root_nodes, load_models_from_gltf},
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
        let gltf = Gltf::open(&files.gltf).map_err(|e| GltfFileLoadError::GltfError(e))?;
        let binary_data = match files.bin {
            Some(bin_file) => std::fs::read(bin_file).map_err(|e| GltfFileLoadError::IoErr(e))?,
            None => {
                let mut bin_data = Vec::<u8>::new();
                for buffer in gltf.buffers() {
                    let data = match buffer.source() {
                        gltf::buffer::Source::Bin => Err(GltfFileLoadError::NoBinaryFile),
                        gltf::buffer::Source::Uri(uri) => {
                            decode_gltf_data_uri(uri).map_err(|_| GltfFileLoadError::BadFile)
                        }
                    };
                    bin_data.extend(data?);
                }
                bin_data
            }
        };
        let root_node_ids = get_root_nodes(&gltf).map_err(|e| GltfFileLoadError::GltfError(e))?;
        let mut joint_ids: Vec<usize> = Vec::new();
        for skin in gltf.skins() {
            for joint in skin.joints() {
                joint_ids.push(joint.index());
            }
        }
        let nodes = gltf.nodes();
        let (models, local_transforms) = load_models_from_gltf(
            root_node_ids,
            &joint_ids,
            nodes,
            &gltf.animations(),
            &gltf.buffers(),
        );
        let gltf_data = GltfData {
            models,
            binary_data,
            local_transforms,
        };

        Ok(gltf_data)
    }
}

pub struct GltfData {
    pub models: Vec<GModel>,
    pub binary_data: Vec<u8>,
    pub local_transforms: Vec<LocalTransform>,
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
