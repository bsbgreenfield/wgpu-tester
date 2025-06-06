use crate::{
    loader::loader::{GltfData, GltfLoader},
    model::util::InitializationError,
    scene::scene::{GScene2, GSceneData},
    transforms,
};
#[allow(dead_code)]
pub struct ScaffoldGlobalTransforms {
    instance_index: usize,
    model_index: usize,
    transform: [[f32; 4]; 4],
}
#[allow(dead_code)]
pub struct ScaffoldModelInstances<'a> {
    model_index: usize,
    transform: &'a [[[f32; 4]; 4]],
}

#[allow(dead_code)]
pub struct SceneScaffold<'a> {
    file_paths: &'a [&'a str],
    global_transforms: &'a [ScaffoldGlobalTransforms],
    instances: &'a [ScaffoldModelInstances<'a>],
}

impl<'a> SceneScaffold<'a> {
    pub fn create(
        &self,
        device: &wgpu::Device,
        aspect_ratio: f32,
    ) -> Result<GScene2, InitializationError> {
        // TODO: fix errors!!!!!
        let gltf_data: GltfData = GltfLoader::load_gltf2(self.file_paths[0])
            .map_err(|_| InitializationError::SceneInitializationError)?; // onyl one file path??
        let scene_data = GSceneData::new(gltf_data);
        let scene = scene_data.build_scene_init(device, aspect_ratio);
        Ok(scene)
    }
}

pub const CUBE: SceneScaffold = SceneScaffold {
    file_paths: &["box"],
    global_transforms: &[],
    instances: &[],
};
pub const FOX: SceneScaffold = SceneScaffold {
    file_paths: &["fox"],
    global_transforms: &[],
    instances: &[],
};
pub const TRUCK: SceneScaffold = SceneScaffold {
    file_paths: &["milk-truck"],
    global_transforms: &[],
    instances: &[],
};
pub const BRAIN: SceneScaffold = SceneScaffold {
    file_paths: &["brain-stem"],
    global_transforms: &[],
    instances: &[],
};
pub const DRAGON: SceneScaffold = SceneScaffold {
    file_paths: &["dragon"],
    global_transforms: &[],
    instances: &[],
};
const fn buggy_shrink(instance_index: usize, model_index: usize) -> ScaffoldGlobalTransforms {
    ScaffoldGlobalTransforms {
        instance_index,
        model_index,
        transform: transforms::scale(0.02),
    }
}
const fn move_right(instance_index: usize, model_index: usize) -> ScaffoldGlobalTransforms {
    ScaffoldGlobalTransforms {
        instance_index,
        model_index,
        transform: transforms::translation(5.0, 0.0, 0.0),
    }
}
pub const BUGGY: SceneScaffold = SceneScaffold {
    file_paths: &["buggy"],
    global_transforms: &[buggy_shrink(0, 0)],
    instances: &[],
};
pub const TRUCK_BOX: SceneScaffold = SceneScaffold {
    file_paths: &["milk-truck", "box"],
    global_transforms: &[move_right(0, 1)],
    instances: &[],
};

pub const BUGGY_BOX: SceneScaffold = SceneScaffold {
    file_paths: &["buggy", "milk-truck"],
    global_transforms: &[buggy_shrink(0, 0), move_right(0, 1)],
    instances: &[],
};
