#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use crate::{
        model::{
            animation::animation_controller::{get_scene_animation_data, SceneAnimationController},
            loader::{self, loader::GltfData},
        },
        scene::scene::GSceneData,
    };

    #[test]
    fn test_box() {
        //        let gltf_data: GltfData = loader::loader::GltfLoader::load_gltf("box-animated").unwrap();
        //        let g2: GltfData = loader::loader::GltfLoader::load_gltf("box-animated").unwrap();
        //        let sa = get_scene_animation_data(gltf_data.simple_animations, &gltf_data.binary_data);
        //        let mut controller: SceneAnimationController = SceneAnimationController::new(sa);
        //        assert!(controller.animations.len() == 1);
        //        // validate samplers
        //
        //        let scene_data = GSceneData::new(g2);
        //        let scene = scene_data.build_scene_uninit();
        //        let (offset, len) = scene.get_instance_local_offset(0, 1);
        //        controller.initialize_animation(0, offset, len);
        //        assert!(controller.active_animations.len() == 1);
        //        assert!(controller.active_animations[0][0].time_elapsed == Duration::ZERO);
        //        println!("{:?}", controller.active_animations[0][0].start_time);
        //        assert!(controller.active_animations[0][0].current_samples.len() == 2);
        //        assert!(controller.active_animations[0][0].current_samples[&0].is_some());
        //        let start_time = std::time::SystemTime::now()
        //            .duration_since(UNIX_EPOCH)
        //            .unwrap();
        //
        //        controller.do_animations(start_time);
        //        assert_eq!(
        //            controller.active_animations[0][0]
        //                .current_samples
        //                .get(&0)
        //                .unwrap()
        //                .unwrap()
        //                .end_time,
        //            1.25
        //        );
        //        assert_eq!(
        //            controller.active_animations[0][0]
        //                .current_samples
        //                .get(&0)
        //                .unwrap()
        //                .unwrap()
        //                .transform_index,
        //            -1
        //        );
        //        assert!(controller.active_animations[0][0].current_samples[&1].is_some());
        //        assert_eq!(
        //            controller.active_animations[0][0]
        //                .current_samples
        //                .get(&1)
        //                .unwrap()
        //                .unwrap()
        //                .end_time,
        //            1.25
        //        );
        //        assert_eq!(
        //            controller.active_animations[0][0]
        //                .current_samples
        //                .get(&1)
        //                .unwrap()
        //                .unwrap()
        //                .transform_index,
        //            0
        //        );
        //
        //        assert_eq!(controller.active_animations[0][0].node_transforms.len(), 2);
        //        controller.do_animations(start_time + Duration::from_secs(1));
        //        controller.do_animations(start_time + Duration::from_millis(1500));
        //        controller.do_animations(start_time + Duration::from_millis(2600));
        //        controller.do_animations(start_time + Duration::from_millis(3800));

        // assert!(
        //     controller.active_animations[0].current_samples[&0]
        //         .unwrap()
        //         .end_time
        //         == 0.0
        // );
        // assert!(
        //     controller.active_animations[0].current_samples[&0]
        //         .unwrap()
        //         .transform_index
        //         == -1
        // );
        // assert!(controller.active_animations[0].current_samples[&1].is_some());
        // assert!(
        //     controller.active_animations[0].current_samples[&1]
        //         .unwrap()
        //         .end_time
        //         == 2.5
        // );
        // assert!(
        //     controller.active_animations[0].current_samples[&1]
        //         .unwrap()
        //         .transform_index
        //         == 0
        // );
    }
}
