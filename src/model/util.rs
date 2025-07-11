use crate::model::model::GMesh;
use gltf::Accessor;
use std::fmt::Debug;

#[derive(Debug)]
pub enum GltfErrors {
    NoIndices,
    NoView,
    NoPrimitive,
    IndicesError(String),
    VericesError(String),
    NormalsError(String),
}

#[derive(Debug)]
pub enum InitializationError {
    InstanceDataInitializationError(Box<String>),
    SceneMergeError(Box<String>),
    SceneInitializationError,
}
pub enum AttributeType {
    Position,
    Normal,
    Index,
}

pub(super) fn get_primitive_data(
    maybe_accessor: Option<&Accessor>,
    _attribute_type: AttributeType,
    buffer_offsets: &Vec<u64>,
) -> Result<Option<(u32, u32)>, GltfErrors> {
    match maybe_accessor {
        Some(accessor) => {
            let byte_size = accessor.size();
            let buffer_view = accessor.view().ok_or(GltfErrors::NoView)?;
            let buffer_offset = buffer_offsets[buffer_view.buffer().index()];
            let offset = buffer_view.offset() + accessor.offset() + buffer_offset as usize;
            return Ok(Some((offset as u32, (accessor.count() * byte_size) as u32)));
        }
        None => Ok(None),
    }
}

pub(super) fn get_model_meshes(
    mesh_ids: &Vec<u32>,
    nodes: &Vec<gltf::Node>,
    buffer_offsets: &Vec<u64>,
) -> Result<Vec<GMesh>, GltfErrors> {
    let mut meshes = Vec::<GMesh>::new();
    for mesh_id in mesh_ids.iter() {
        let mesh = nodes
            .iter()
            .find(|n| n.mesh().is_some() && n.mesh().unwrap().index() as u32 == *mesh_id)
            .unwrap()
            .mesh()
            .unwrap();

        let g_mesh = GMesh::new(&mesh, buffer_offsets)?;
        meshes.push(g_mesh);
    }

    Ok(meshes)
}
