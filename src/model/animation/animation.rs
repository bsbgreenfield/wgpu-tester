use std::{collections::HashMap, rc::Rc, time::Duration};

use cgmath::SquareMatrix;

use crate::model::animation::animation_node::{AnimationNode, AnimationSample};

pub struct AnimationFrame<'a> {
    pub lt_offsets: Vec<usize>,
    pub transform_slices: Vec<&'a [[[f32; 4]; 4]]>,
}

pub(super) struct AnimationInstance {
    /// the node tree for the model
    pub(super) animation_node: Rc<AnimationNode>,
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
    pub(super) fn process_animation_frame(
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
