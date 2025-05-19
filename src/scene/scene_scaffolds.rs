use crate::model::util::{load_gltf, InitializationError};

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
    //TODO: add better error handling
    pub fn create(
        &self,
        device: &wgpu::Device,
        aspect_ratio: f32,
    ) -> Result<GScene, InitializationError> {
        let mut gltfs: Vec<GScene> = Vec::with_capacity(self.file_paths.len());
        for file in self.file_paths {
            let maybe_scene = load_gltf(&file, device, aspect_ratio);
            match maybe_scene {
                Ok(scene) => gltfs.push(scene),
                Err(_) => return Err(InitializationError::SceneInitializationError),
            }
        }
        let mut maybe_return_scene: Option<GScene> = None;
        match gltfs.len() {
            0 => return Err(InitializationError::SceneInitializationError),
            1 => {
                let _ = maybe_return_scene.insert(gltfs.remove(0));
            }
            _ => {
                let mut iter = gltfs.into_iter();
                let mut s = iter.next().unwrap();
                for _ in 0..iter.len() - 1 {
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
pub const BRAIN: SceneScaffold = SceneScaffold {
    file_paths: &["brain-stem"],
    global_transforms: &[],
    instances: &[],
};
pub const BUGGY: SceneScaffold = SceneScaffold {
    file_paths: &["buggy"],
    global_transforms: &[],
    instances: &[],
};
//ScaffoldModelInstances {
//        model_index: 0,
//        transform: &[crate::transforms::translation_2(5.0, 0.0, -5.5)],
//    }
