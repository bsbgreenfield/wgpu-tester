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
    pub transform: [[f32; 4]; 4],
}
#[allow(dead_code)]
pub struct AdditionalScaffoldModelInstances<'a> {
    pub model_index: usize,
    pub additional_instance_count: usize,
    pub global_transforms: &'a [[[f32; 4]; 4]],
}

pub struct ScaffoldGTOverride {
    pub transform: [[f32; 4]; 4],
    pub model_idx: usize,
}
#[allow(dead_code)]
pub struct SceneScaffold<'a> {
    file_paths: &'a [&'a str],
    pub additional_instances: &'a [AdditionalScaffoldModelInstances<'a>],
    pub global_transform_overrides: &'a [ScaffoldGTOverride],
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
        let scene = scene_data.build_scene_from_scaffold(device, aspect_ratio, self)?;
        Ok(scene)
    }
}

pub const FLEXY_BOX: SceneScaffold = SceneScaffold {
    file_paths: &["flexy-box"],
    global_transform_overrides: &[],
    additional_instances: &[],
};

pub const BUGGY: SceneScaffold = SceneScaffold {
    file_paths: &["buggy"],
    global_transform_overrides: &[ScaffoldGTOverride {
        transform: transforms::scale(0.02),
        model_idx: 0,
    }],
    additional_instances: &[],
};

pub const CUBE: SceneScaffold = SceneScaffold {
    file_paths: &["box"],
    global_transform_overrides: &[],
    additional_instances: &[AdditionalScaffoldModelInstances {
        model_index: 0,
        additional_instance_count: 1,
        global_transforms: &[transforms::translation(5.0, 5.0, 0.0)],
    }],
};
pub const FOX: SceneScaffold = SceneScaffold {
    global_transform_overrides: &[ScaffoldGTOverride {
        model_idx: 0,
        transform: transforms::scale(0.05),
    }],
    file_paths: &["fox"],
    additional_instances: &[],
};
pub const TRUCK: SceneScaffold = SceneScaffold {
    global_transform_overrides: &[],
    file_paths: &["milk-truck"],
    additional_instances: &[],
};
pub const BRAIN: SceneScaffold = SceneScaffold {
    global_transform_overrides: &[],
    file_paths: &["brain-stem"],
    additional_instances: &[],
};
pub const DRAGON: SceneScaffold = SceneScaffold {
    global_transform_overrides: &[],
    file_paths: &["dragon"],
    additional_instances: &[],
};
pub const BOX_ANIMATED: SceneScaffold = SceneScaffold {
    global_transform_overrides: &[],
    file_paths: &["box-animated"],
    additional_instances: &[],
};
pub const CMAN: SceneScaffold = SceneScaffold {
    global_transform_overrides: &[],
    file_paths: &["cesium-man"],
    additional_instances: &[],
};
pub const TRUCK_BOX: SceneScaffold = SceneScaffold {
    global_transform_overrides: &[ScaffoldGTOverride {
        model_idx: 1,
        transform: transforms::translation(5.0, 0.0, 0.0),
    }],
    file_paths: &["milk-truck", "box"],
    additional_instances: &[],
};

pub const MONKEY: SceneScaffold = SceneScaffold {
    global_transform_overrides: &[],
    file_paths: &["monkey"],
    additional_instances: &[],
};

pub const POLLY: SceneScaffold = SceneScaffold {
    global_transform_overrides: &[],
    file_paths: &["polly"],
    additional_instances: &[],
};
