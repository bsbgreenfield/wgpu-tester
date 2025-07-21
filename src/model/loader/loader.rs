use std::{collections::HashMap, path::PathBuf};

use crate::{
    model::{
        loader::util::{
            decode_gltf_data_uri, get_data_files, get_root_nodes, load_models_from_gltf,
        },
        model::{GModel, LocalTransform},
    },
    scene::scene::PrimitiveData,
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
        let nodes = gltf.nodes();
        let gltf_data: GltfData = load_models_from_gltf(
            root_node_ids,
            nodes,
            &gltf.animations(),
            &gltf.buffers(),
            &gltf.skins(),
            binary_data,
        );

        Ok(gltf_data)
    }
}

pub struct ModelPrimitiveData {
    pub model_id: usize,
    pub primitive_data: Vec<PrimitiveData>,
}

pub struct GltfData {
    pub models: Vec<GModel>,
    pub binary_data: Vec<u8>,
    pub model_primitive_data: Vec<ModelPrimitiveData>,
    pub local_transforms: Vec<LocalTransform>,
    pub joint_transforms: Vec<[[f32; 4]; 4]>,
    pub skin_ibms: HashMap<usize, Vec<cgmath::Matrix4<f32>>>,
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
