use std::{collections::HashMap, fmt::Debug, marker::PhantomData, rc::Rc, time::Duration};

use cgmath::SquareMatrix;

use crate::model::animation::animation_node::{AnimationNode, AnimationSample};

pub(super) struct AnimationProcessingResult<'a> {
    pub(super) mesh_transforms: &'a [[[f32; 4]; 4]],
    pub(super) joint_transforms: &'a [[[f32; 4]; 4]],
    pub(super) is_done: bool,
}
pub struct AnimationFrame<'a> {
    pub lt_offsets: Vec<usize>,
    pub mesh_transform_slices: Vec<&'a [[[f32; 4]; 4]]>,
    pub joint_transform_slices: Vec<&'a [[[f32; 4]; 4]]>,
}

pub struct MeshAnimationInstance;
pub struct JointAnimationInstance;

#[derive(Debug)]
pub(super) enum GAnimationInstance {
    MeshAnimationInstanceType(AnimationInstance<MeshAnimationInstance>),
    JointAnimationInstanceType(AnimationInstance<JointAnimationInstance>),
}
impl GAnimationInstance {
    pub fn new_mesh_animation(
        animation_node: Rc<AnimationNode>,
        model_instance_offset: usize,
        start_time: Duration,
        time_elapsed: Duration,
        animation_index: usize,
        mesh_transforms: Vec<[[f32; 4]; 4]>,
        joint_transforms: Vec<[[f32; 4]; 4]>,
        current_samples: HashMap<usize, Option<AnimationSample>>,
    ) -> Self {
        Self::MeshAnimationInstanceType(AnimationInstance {
            animation_node,
            model_instance_offset,
            start_time,
            time_elapsed,
            animation_index,
            mesh_transforms,
            joint_transforms,
            current_samples,
            _ty: PhantomData::<MeshAnimationInstance>,
        })
    }
    pub fn new_joint_animation(
        animation_node: Rc<AnimationNode>,
        model_instance_offset: usize,
        start_time: Duration,
        time_elapsed: Duration,
        animation_index: usize,
        mesh_transforms: Vec<[[f32; 4]; 4]>,
        joint_transforms: Vec<[[f32; 4]; 4]>,
        current_samples: HashMap<usize, Option<AnimationSample>>,
    ) -> Self {
        Self::JointAnimationInstanceType(AnimationInstance {
            animation_node,
            model_instance_offset,
            start_time,
            time_elapsed,
            animation_index,
            mesh_transforms,
            joint_transforms,
            current_samples,
            _ty: PhantomData::<JointAnimationInstance>,
        })
    }

    pub(super) fn get_model_instance_offset(&self) -> usize {
        match self {
            Self::MeshAnimationInstanceType(a) => a.model_instance_offset,
            Self::JointAnimationInstanceType(a) => a.model_instance_offset,
        }
    }

    pub(super) fn process_animation_frame<'a>(
        &'a mut self,
        timestamp: Duration,
        mesh_to_lt_index_map: &HashMap<usize, usize>,
        joint_to_joint_index_map: &HashMap<usize, usize>,
    ) -> AnimationProcessingResult<'a> {
        match self {
            Self::MeshAnimationInstanceType(a) => {
                a.process_animation_frame(timestamp, mesh_to_lt_index_map, joint_to_joint_index_map)
            }
            Self::JointAnimationInstanceType(a) => todo!(),
        }
    }
}

pub(super) struct AnimationInstance<T> {
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
    _ty: PhantomData<T>,
}
impl<T> Debug for AnimationInstance<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimationInstance")
            .field("animation index", &self.animation_index)
            .field("start time", &self.start_time)
            .finish()
    }
}

impl AnimationInstance<MeshAnimationInstance> {
    /// given the current timestamp, mutate this instance's mesh transforms,
    /// and return it as a slice
    pub(super) fn process_animation_frame<'a>(
        &'a mut self,
        timestamp: Duration,
        mesh_to_lt_index_map: &HashMap<usize, usize>,
        joint_to_joint_index_map: &HashMap<usize, usize>,
    ) -> AnimationProcessingResult<'a> {
        self.time_elapsed = timestamp - self.start_time;
        // im not sure if there a good way to do this without cloning the node RC
        // i dont think its a big problem, but its annoying.
        let node = self.animation_node.clone();
        let done = node.update_node_transforms(
            self,
            cgmath::Matrix4::<f32>::identity(),
            mesh_to_lt_index_map,
            joint_to_joint_index_map,
        );
        return AnimationProcessingResult {
            mesh_transforms: &self.mesh_transforms[..],
            joint_transforms: &self.joint_transforms[..],
            is_done: done,
        };
    }
}

pub struct SimpleAnimation {
    pub animation_node: Rc<AnimationNode>,
    pub model_id: usize,
    pub node_to_lt_index_map: HashMap<usize, usize>,
    pub is_joint_animation: bool,
}
impl SimpleAnimation {
    pub fn new(
        animation_node: AnimationNode,
        model_id: usize,
        node_to_lt_index_map: HashMap<usize, usize>,
        is_joint_animation: bool,
    ) -> Self {
        Self {
            node_to_lt_index_map,
            animation_node: Rc::new(animation_node),
            model_id,
            is_joint_animation,
        }
    }
    pub fn print(&self) {
        println!("Animation on model {}", self.model_id);
        println!("Node: ");
        self.animation_node.print();
    }
}
