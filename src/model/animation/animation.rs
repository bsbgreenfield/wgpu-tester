use std::{collections::HashMap, fmt::Debug, rc::Rc, time::Duration};

use cgmath::SquareMatrix;

use crate::model::{
    animation::{animation_controller::AnimationSample, animation_node::AnimationNode},
    model::ModelAnimationData,
};

pub(super) struct AnimationProcessingResult<'a> {
    pub(super) mesh_transforms: &'a [[[f32; 4]; 4]],
    pub(super) joint_transforms: &'a [[[f32; 4]; 4]],
    pub(super) joint_indices: &'a [usize],
    pub(super) is_done: bool,
}
pub struct AnimationFrame<'a> {
    pub lt_offsets: Vec<usize>,
    pub mesh_transform_slices: Vec<&'a [[[f32; 4]; 4]]>,
    pub joint_ids: Vec<&'a [usize]>,
    pub joint_transform_slices: Vec<&'a [[[f32; 4]; 4]]>,
}

pub struct MeshAnimationInstance;
pub struct JointAnimationInstance;

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
    pub(super) joint_transforms: Vec<[[f32; 4]; 4]>,
    /// a map of sampler id -> sample
    /// used to keep track of the last frames data
    pub(super) current_samples: HashMap<usize, Option<AnimationSample>>,
}
impl Debug for AnimationInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimationInstance")
            .field("animation index", &self.animation_index)
            .field("start time", &self.start_time)
            .finish()
    }
}

impl AnimationInstance {
    pub fn new(
        animation_node: Rc<AnimationNode>,
        model_instance_offset: usize,
        start_time: Duration,
        time_elapsed: Duration,
        animation_index: usize,
        mesh_transforms: Vec<[[f32; 4]; 4]>,
        joint_transforms: Vec<[[f32; 4]; 4]>,
        current_samples: HashMap<usize, Option<AnimationSample>>,
    ) -> Self {
        Self {
            animation_node,
            model_instance_offset,
            start_time,
            time_elapsed,
            animation_index,
            mesh_transforms,
            joint_transforms,
            current_samples,
        }
    }

    /// given the current timestamp, mutate this instance's mesh transforms,
    /// and return it as a slice
    pub(super) fn process_animation_frame<'a>(
        &'a mut self,
        timestamp: Duration,
        animation_data: &'a ModelAnimationData,
        skin_ibms: &HashMap<usize, Vec<cgmath::Matrix4<f32>>>,
    ) -> AnimationProcessingResult<'a> {
        self.time_elapsed = timestamp - self.start_time;
        // im not sure if there a good way to do this without cloning the node RC
        // i dont think its a big problem, but its annoying.
        let node = self.animation_node.clone();
        let done = node.update_node_transforms(
            self,
            cgmath::Matrix4::<f32>::identity(),
            animation_data,
            skin_ibms,
        );
        return AnimationProcessingResult {
            mesh_transforms: &self.mesh_transforms[..],
            joint_transforms: &self.joint_transforms[..],
            joint_indices: &animation_data.joint_animation_data.joint_indices[..],
            is_done: done,
        };
    }
}
