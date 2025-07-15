use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
    time::{Duration, UNIX_EPOCH},
};

use crate::{
    model::{
        animation::{
            animation::*,
            animation_node::{AnimationNode, AnimationSample},
            util::copy_data_for_animation,
        },
        model::{GModel, ModelAnimationData},
    },
    transforms,
};

pub fn get_scene_animation_data(models: &mut Vec<GModel>, main_buffer_data: &Vec<u8>) {
    for (idx, model) in models.iter_mut().enumerate() {
        if let Some(animation_data) = model.animation_data.as_mut() {
            let exclusive_node_reference: &mut AnimationNode =
                Rc::get_mut(&mut animation_data.animation_node)
                    .expect("should be an exclusive reference");
            copy_data_for_animation(exclusive_node_reference, idx, main_buffer_data);
        }
    }
}

/// for each mdoel with one or more animation nodes, extract the times and translations data
/// from the main blob and put them in the relevant samplers.
//pub fn get_scene_animation_data(
//    mut simple_animations: Vec<SimpleAnimation>,
//    main_buffer_data: &Vec<u8>,
//) -> Vec<SimpleAnimation> {
//    for animation in simple_animations.iter_mut() {
//        let exclusive_node_reference: &mut AnimationNode =
//            Rc::get_mut(&mut animation.animation_node)
//                .expect("this should be the only reference to the node");
//        copy_data_for_animation(
//            exclusive_node_reference,
//            animation.model_id,
//            main_buffer_data,
//        );
//    }
//    simple_animations
//}

/// Keeps track of which animations are currently playing.
/// The controllers functions are
/// 1. adding or removing active animation indices based on user input and time
/// 2. owning all animation structs
/// 3. interface between animations and the app.
pub struct SceneAnimationController {
    dead_animations: Vec<usize>,
    pub(super) active_animations: Vec<VecDeque<GAnimationInstance>>,
    pub(super) active_animation_count: usize,
}

impl SceneAnimationController {
    pub fn new(model_no: usize) -> Self {
        let mut active_animations = Vec::with_capacity(model_no);
        for _ in 0..model_no {
            active_animations.push(VecDeque::with_capacity(10));
        }
        Self {
            dead_animations: vec![0; model_no],
            active_animations,
            active_animation_count: 0,
        }
    }

    pub fn initialize_animation(
        &mut self,
        animation_data: &ModelAnimationData,
        animation_index: usize,
        model_instance_offset: usize,
        model_mesh_instance_count: usize,
        model_joint_instance_count: usize,
    ) {
        let animation_node = animation_data.animation_node.clone();
        let mut mesh_transforms: Vec<[[f32; 4]; 4]> = Vec::with_capacity(model_mesh_instance_count);
        let mut joint_transforms: Vec<[[f32; 4]; 4]> =
            Vec::with_capacity(model_joint_instance_count);
        let mut sample_map = HashMap::<usize, Option<AnimationSample>>::new();
        animation_node.get_default_samples(animation_index, &mut sample_map);
        animation_node.initialize_sampled_transforms(&mut mesh_transforms, &mut joint_transforms);
        println!("Joint transforms: {:?}", joint_transforms);
        println!("Mesh transforms: {:?}", mesh_transforms);

        let start_time = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap();

        let animation_instance = GAnimationInstance::new_mesh_animation(
            animation_node,
            model_instance_offset,
            start_time,
            Duration::ZERO,
            animation_index,
            mesh_transforms,
            joint_transforms,
            sample_map,
        );
        self.active_animations[animation_data.model_index].push_back(animation_instance);
        self.active_animation_count += 1;
    }

    pub fn do_animations<'a>(
        &'a mut self,
        timestamp: Duration,
        models: &'a Vec<GModel>,
    ) -> Option<AnimationFrame<'a>> {
        // process any animations that were marked as done last frame
        for (idx, dead_animation_count) in self.dead_animations.iter_mut().enumerate() {
            let count = dead_animation_count.clone();
            for _ in 0..count {
                self.active_animations[idx].pop_front();
                *dead_animation_count -= 1;
                self.active_animation_count -= 1;
            }
        }

        // if there are no active animations, do nothing
        if self.active_animation_count == 0 {
            return None;
        }
        let len = self.active_animations.len();

        let mut frame = AnimationFrame {
            mesh_transform_slices: Vec::with_capacity(len),
            joint_transform_slices: Vec::with_capacity(len),
            joint_ids: Vec::new(),
            lt_offsets: Vec::with_capacity(len),
        };

        for (idx, bucket) in self.active_animations.iter_mut().enumerate() {
            if bucket.len() == 0 {
                continue;
            }
            let animation_data = &models[idx].animation_data.as_ref().unwrap();
            for animation_instance in bucket.iter_mut() {
                frame
                    .lt_offsets
                    .push(animation_instance.get_model_instance_offset());
                let animation_processing_result =
                    animation_instance.process_animation_frame(timestamp, animation_data);
                frame
                    .mesh_transform_slices
                    .push(animation_processing_result.mesh_transforms);
                frame
                    .joint_transform_slices
                    .push(animation_processing_result.joint_transforms);
                frame
                    .joint_ids
                    .push(animation_processing_result.joint_indices);
                if animation_processing_result.is_done {
                    self.dead_animations[idx] += 1;
                }
            }
        }
        Some(frame)
    }
}
