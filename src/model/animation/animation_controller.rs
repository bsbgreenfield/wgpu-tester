use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
    time::{Duration, UNIX_EPOCH},
};

use cgmath::SquareMatrix;

use crate::model::animation::{
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
        let exvlusive_node_reference: &mut AnimationNode =
            Rc::get_mut(&mut animation.animation_node)
                .expect("this should be the only reference to the node");
        copy_data_for_animation(
            exvlusive_node_reference,
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

pub struct AnimationFrame<'a> {
    pub lt_offsets: Vec<usize>,
    pub transform_slices: Vec<&'a [[[f32; 4]; 4]]>,
}

pub(super) struct AnimationInstance {
    /// the node tree for the model
    animation_node: Rc<AnimationNode>,
    /// the offset in the local transform buffer that this instance affects
    pub(super) model_instance_offset: usize,
    pub(super) start_time: Duration,
    pub(super) time_elapsed: Duration,
    /// global index of the animation as defined in the gltf file
    pub(super) animation_index: usize,
    /// the set of transforms affected by the samplers
    /// of this instances node tree
    pub(super) mesh_transforms: Vec<[[f32; 4]; 4]>,
    /// a map of sampler id -> sample
    /// used to keep track of the last frames data
    pub(super) current_samples: HashMap<usize, Option<AnimationSample>>,
}

impl AnimationInstance {
    /// given the current timestamp, mutate this instance's mesh transforms,
    /// and return it as a slice
    fn process_animation_frame(
        &mut self,
        timestamp: Duration,
        node_to_lt_index_map: &HashMap<usize, usize>,
    ) -> (&[[[f32; 4]; 4]], bool) {
        self.time_elapsed = timestamp - self.start_time;
        // im not sure if there a good way to do this without cloning the node RC
        // i dont think its a big problem, but its annoying.
        let node = self.animation_node.clone();
        let done = node.update_mesh_transforms(
            self,
            cgmath::Matrix4::<f32>::identity(),
            node_to_lt_index_map,
        );
        return (&self.mesh_transforms[..], done);
    }
}

pub struct SimpleAnimation {
    pub animation_node: Rc<AnimationNode>,
    pub model_id: usize,
    pub node_to_lt_index_map: HashMap<usize, usize>,
}
impl SimpleAnimation {
    pub fn new(
        animation_node: AnimationNode,
        model_id: usize,
        node_to_lt_index_map: HashMap<usize, usize>,
    ) -> Self {
        Self {
            node_to_lt_index_map,
            animation_node: Rc::new(animation_node),
            model_id,
        }
    }
    pub fn print(&self) {
        println!("Animation on model {}", self.model_id);
        println!("Node: ");
        self.animation_node.print();
    }
}
