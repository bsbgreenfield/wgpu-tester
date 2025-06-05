use crate::{
    loader::loader::{GltfData, GltfLoader},
    model::util::InitializationError,
    scene::{
        instances::InstanceData,
        scene::{GScene2, GSceneData},
    },
    transforms,
};

use super::scene::GScene;

pub struct ScaffoldGlobalTransforms {
    instance_index: usize,
    model_index: usize,
    transform: [[f32; 4]; 4],
}
pub struct ScaffoldModelInstances<'a> {
    model_index: usize,
    transform: &'a [[[f32; 4]; 4]],
}

pub struct SceneScaffold<'a> {
    file_paths: &'a [&'a str],
    global_transforms: &'a [ScaffoldGlobalTransforms],
    instances: &'a [ScaffoldModelInstances<'a>],
}

impl<'a> SceneScaffold<'a> {
    pub fn create2(
        &self,
        device: &wgpu::Device,
        aspect_ratio: f32,
    ) -> Result<GScene2, InitializationError> {
        // TODO: fix errors!!!!!
        let gltf_data: GltfData = GltfLoader::load_gltf2(self.file_paths[0])
            .map_err(|_| InitializationError::SceneInitializationError)?; // onyl one file path??
        let scene_data = GSceneData::new(gltf_data);
        let scene = scene_data.build_scene(device, aspect_ratio);
        Ok(scene)
    }
    //TODO: add better error handling
    pub fn create(
        &self,
        device: &wgpu::Device,
        aspect_ratio: f32,
    ) -> Result<GScene, InitializationError> {
        let mut gltfs: Vec<GScene> = Vec::with_capacity(self.file_paths.len());

        // TEST!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
        let f = self.file_paths.first().unwrap();
        let _ = GltfLoader::load_gltf2(&f);
        for file in self.file_paths {
            let maybe_scene = Err(());
            match maybe_scene {
                Ok(scene) => gltfs.push(scene),
                Err(_) => return Err(InitializationError::SceneInitializationError),
            }
        }
        let mut maybe_return_scene: Option<GScene> = None;
        let no_scenes = gltfs.len().clone();
        match no_scenes {
            0 => return Err(InitializationError::SceneInitializationError),
            1 => {
                let _ = maybe_return_scene.insert(gltfs.remove(0));
            }
            _ => {
                let mut iter = gltfs.into_iter();
                let mut s = iter.next().unwrap();
                println!("for _ in 1..{}", iter.len());
                for _ in 1..no_scenes {
                    println!("merging???????");
                    s = GScene::merge(s, iter.next().unwrap())?;
                }
                let _ = maybe_return_scene.insert(s);
            }
        }

        if let Some(mut return_scene) = maybe_return_scene {
            for gt in self.global_transforms.iter() {
                return_scene.update_global_transform(
                    gt.model_index,
                    gt.instance_index,
                    gt.transform,
                );
            }
            for instance_update in self.instances.iter() {
                return_scene.add_model_instances(
                    instance_update.model_index,
                    instance_update.transform.to_vec(),
                );
            }
            // this needs to error handle!!
            return_scene.init(device);
            return Ok(return_scene);
        } else {
            return Err(InitializationError::SceneInitializationError);
        }
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
//ScaffoldModelInstances {
//        model_index: 0,
//        transform: &[crate::transforms::translation_2(5.0, 0.0, -5.5)],
//    }
