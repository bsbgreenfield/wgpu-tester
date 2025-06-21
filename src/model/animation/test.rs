#[cfg(test)]
mod tests {
    use crate::model::{
        animation::animation_controller::SceneAnimationController,
        loader::{self, loader::GltfData},
    };

    use super::*;

    #[test]
    fn test_box() {
        let gltf_data: GltfData = loader::loader::GltfLoader::load_gltf("box-animated").unwrap();
        let controller: SceneAnimationController =
            SceneAnimationController::new(gltf_data.simple_animations);
    }
}
