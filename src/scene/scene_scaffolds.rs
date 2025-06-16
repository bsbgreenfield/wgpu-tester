use crate::{
    model::{
        loader::loader::{GltfData, GltfLoader},
        util::InitializationError,
    },
    scene::scene::{GScene, GSceneData},
    transforms,
};
#[allow(dead_code)]
pub struct ScaffoldGlobalTransforms {
    pub instance_index: usize,
    pub model_index: usize,
    pub transform: [[f32; 4]; 4],
}
#[allow(dead_code)]
pub struct ScaffoldModelInstances {
    pub model_index: usize,
    instance_count: usize,
}

#[allow(dead_code)]
pub struct SceneScaffold<'a> {
    file_paths: &'a [&'a str],
    pub global_transforms: &'a [ScaffoldGlobalTransforms],
    pub instances: &'a [ScaffoldModelInstances],
}
impl<'a> SceneScaffold<'a> {
    pub fn create(
        &self,
        device: &wgpu::Device,
        aspect_ratio: f32,
    ) -> Result<GScene, InitializationError> {
        // TODO: fix errors!!!!!
        let gltf_data: GltfData = GltfLoader::load_gltf(self.file_paths[0])
            .map_err(|_| InitializationError::SceneInitializationError)?; // onyl one file path??
        let scene_data = GSceneData::new(gltf_data);
        let scene = scene_data.build_scene_from_scaffold(device, aspect_ratio, self);
        Ok(scene)
    }
}

pub const CUBE: SceneScaffold = SceneScaffold {
    file_paths: &["box"],
    global_transforms: &[ScaffoldGlobalTransforms {
        model_index: 0,
        instance_index: 1,
        transform: transforms::translation(5.0, 0.0, 0.0),
    }],
    instances: &[ScaffoldModelInstances {
        model_index: 0,
        instance_count: 2,
    }],
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
