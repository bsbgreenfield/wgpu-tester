use crate::model::{
    model::{GMesh, GModel, LocalTransform},
    util::GltfErrors,
};

use super::scene::*;
use gltf::Node;
impl GScene {
    pub fn print_transforms(&self) {
        let mut lt = self.instance_data.local_transform_data.iter();
        for (i, m) in self.models.iter().enumerate() {
            println!("MODEL {i} ---------------------------------------");
            for (idx, _) in m.mesh_instances.iter().enumerate() {
                println!("MESH {idx} ----------------------------------");
                for _ in 0..m.mesh_instances[idx] {
                    let maybet = lt.next();
                    if let Some(t) = maybet {
                        println!("            {t:?}");
                    }
                }
            }
        }
    }
}
pub(super) fn find_meshes(
    root_node: &Node,
    mut scene_mesh_data: SceneMeshData,
    mut base_translation: cgmath::Matrix4<f32>,
) -> SceneMeshData {
    'block: {
        let cg_trans = cgmath::Matrix4::<f32>::from(root_node.transform().matrix());
        base_translation = base_translation * cg_trans;
        if let Some(mesh) = root_node.mesh() {
            // this is an instance of a mesh. Push the current base translation
            let local_transform: LocalTransform = LocalTransform {
                model_index: 0,
                transform_matrix: base_translation.into(),
            };
            scene_mesh_data
                .transformation_matrices
                .push(local_transform);
            // check mesh_ids to see if this particular mesh has already been added, if so, the index
            // of the match is equal to the index within mesh_instances that we want to increment by 1
            for (idx, m) in scene_mesh_data.mesh_ids.iter().enumerate() {
                if *m == mesh.index() as u32 {
                    scene_mesh_data.mesh_instances[idx] += 1;
                    break 'block;
                }
            }
            // this mesh has not been added: append to both vecs
            scene_mesh_data.mesh_ids.push(mesh.index() as u32);
            scene_mesh_data.mesh_instances.push(1);
        }
    }
    for child_node in root_node.children() {
        scene_mesh_data = find_meshes(&child_node, scene_mesh_data, base_translation);
    }

    scene_mesh_data
}

pub(super) fn get_meshes(
    mesh_ids: &Vec<u32>,
    nodes: &Vec<Node>,
    scene_buffer_data: &mut SceneBufferData,
) -> Result<Vec<GMesh>, GltfErrors> {
    let mut meshes = Vec::<GMesh>::new();
    for mesh_id in mesh_ids.iter() {
        // cursed?
        let mesh = nodes
            .iter()
            .find(|n| n.mesh().is_some() && n.mesh().unwrap().index() as u32 == *mesh_id)
            .unwrap()
            .mesh()
            .unwrap();
        let g_mesh = GMesh::new(&mesh, scene_buffer_data)?;
        meshes.push(g_mesh);
    }
    Ok(meshes)
}

pub(super) fn calculate_model_mesh_offsets(
    models: &Vec<GModel>,
    model_instances: &Vec<usize>,
) -> Vec<usize> {
    let mut model_mesh_offsets = Vec::with_capacity(models.len());
    let mut sum = 0;
    for (idx, model) in models.iter().enumerate() {
        model_mesh_offsets.push(sum);
        sum += (model.mesh_instances.iter().sum::<u32>() as usize) * model_instances[idx];
    }
    model_mesh_offsets
}
