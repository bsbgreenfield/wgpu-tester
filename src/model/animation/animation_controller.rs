use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
    time::{Duration, UNIX_EPOCH},
};

use crate::model::animation::{
    animation::*,
    animation_node::{AnimationNode, AnimationSample},
    util::copy_data_for_animation,
};
/// for each mdoel with one or more animation nodes, extract the times and translations data
/// from the main blob and put them in the relevant samplers.
pub fn get_scene_animation_data(
    mut simple_animations: Vec<SimpleAnimation>,
    main_buffer_data: &Vec<u8>,
) -> Vec<SimpleAnimation> {
    for animation in simple_animations.iter_mut() {
        let exclusive_node_reference: &mut AnimationNode =
            Rc::get_mut(&mut animation.animation_node)
                .expect("this should be the only reference to the node");
        copy_data_for_animation(
            exclusive_node_reference,
            animation.model_id,
            main_buffer_data,
        );
    }
    simple_animations
}

/// Keeps track of which animations are currently playing.
/// The controllers functions are
/// 1. adding or removing active animation indices based on user input and time
/// 2. owning all animation structs
/// 3. interface between animations and the app.
pub struct SceneAnimationController {
    dead_animations: Vec<usize>,
    pub(super) active_animations: Vec<VecDeque<AnimationInstance>>,
    pub(super) animations: Vec<SimpleAnimation>,
}

impl SceneAnimationController {
    pub fn new(animations: Vec<SimpleAnimation>) -> Self {
        let mut active_animations = Vec::with_capacity(animations.len());
        for _ in 0..animations.len() {
            active_animations.push(VecDeque::with_capacity(10));
        }
        Self {
            dead_animations: vec![0; animations.len()],
            active_animations,
            animations,
        }
    }

    pub fn initialize_animation(
        &mut self,
        animation_index: usize,
        model_instance_offset: usize,
        model_mesh_instance_count: usize,
    ) {
        // clone a shared reference to the animation node tree
        let animation_node = self.animations[animation_index].animation_node.clone();
        // get copies of the initial state of the animated nodes
        let mut mesh_transforms: Vec<[[f32; 4]; 4]> = Vec::with_capacity(model_mesh_instance_count);
        let mut sample_map = HashMap::<usize, Option<AnimationSample>>::new();
        let _ = &animation_node.get_default_samples(animation_index, &mut sample_map);
        let _ = &animation_node.initialize_sampled_transforms(&mut mesh_transforms);
        let start_time = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap();
        let animation_instance = AnimationInstance {
            model_instance_offset,
            animation_node,
            start_time,
            time_elapsed: Duration::ZERO,
            animation_index,
            mesh_transforms,
            current_samples: sample_map,
        };
        self.active_animations[animation_index].push_back(animation_instance);
    }

    pub fn do_animations<'a>(&'a mut self, timestamp: Duration) -> Option<AnimationFrame<'a>> {
        for (idx, dead_animation_count) in self.dead_animations.iter_mut().enumerate() {
            let count = dead_animation_count.clone();
            for _ in 0..count {
                self.active_animations[idx].pop_front();
                *dead_animation_count -= 1;
            }
        }
        let len = self.active_animations.len();
        if len == 0 {
            return None;
        }

        let mut frame = AnimationFrame {
            transform_slices: Vec::with_capacity(len),
            lt_offsets: Vec::with_capacity(len),
        };
        for (idx, animation_bucket) in self.active_animations.iter_mut().enumerate() {
            let map = &self.animations[idx].node_to_lt_index_map;
            for animation_instance in animation_bucket.iter_mut() {
                let offset = animation_instance.model_instance_offset;
                frame.lt_offsets.push(offset);
                let (transforms, done) = animation_instance.process_animation_frame(timestamp, map);
                frame.transform_slices.push(transforms);
                if done {
                    self.dead_animations[idx] += 1;
                }
            }
        }
        Some(frame)
    }
}
