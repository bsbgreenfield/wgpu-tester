use std::{ops::Range, rc::Rc, thread::current};

use gltf::{
    accessor::{DataType, Iter},
    animation::Channel,
};

use crate::model::model::LocalTransform;

pub struct Animation {
    pub animation_components: Vec<AnimationComponent>,
}

pub struct AnimationComponent {
    mesh_ids: Vec<usize>,
    times_data: (usize, usize),
    transforms_data: (usize, usize),
    interpolation: Interpolation,
}

impl AnimationComponent {
    pub fn new_uninit(
        mesh_ids: Vec<usize>,
        times_data: (usize, usize),
        transforms_data: (usize, usize),
        interpolation: Interpolation,
    ) -> Self {
        Self {
            mesh_ids,
            times_data,
            transforms_data,
            interpolation,
        }
    }
}

#[derive(Debug)]
pub enum Interpolation {
    Linear,
}
impl From<gltf::animation::Interpolation> for Interpolation {
    fn from(value: gltf::animation::Interpolation) -> Self {
        match value {
            gltf::animation::Interpolation::Linear => Interpolation::Linear,
            _ => todo!(),
        }
    }
}

/// Keeps track of which animations are currently playing.
/// The controllers functions are
/// 1. adding or removing active animation indices based on user input and time
/// 2. owning all animation structs
/// 3. interface between animations and the app.
pub struct SceneAnimationController {
    active_animations: Vec<usize>,
    animations: Vec<SimpleAnimation>,
}

struct SimpleAnimation {
    root_node: AnimationTransformNode,
}

struct AnimationTransformNode {
    node_id: usize,
    transform: [[f32; 4]; 4],
    sampler: Option<AnimationSampler>,
}
impl SimpleAnimation {
    fn get_transforms(
        node: &AnimationTransformNode,
        timestamp: f32,
        mut base_translation: cgmath::Matrix4<f32>,
        local_transforms: &mut Vec<[[f32; 4]; 4]>,
    ) {
        match &node.sampler {
            Some(sampler) => todo!(),
            None => {
                base_translation = base_translation * cgmath::Matrix4::<f32>::from(node.transform);
            }
        }
    }
}
enum AnimationType {
    Rotation,
    Translation,
    Scale,
    // others?
}

impl AnimationType {
    fn from(property: &gltf::animation::Property) -> Self {
        use gltf::animation::Property;
        match property {
            Property::Translation => AnimationType::Translation,
            Property::Rotation => AnimationType::Rotation,
            Property::Scale => AnimationType::Rotation,
            Property::MorphTargetWeights => todo!(),
        }
    }
}
pub struct AnimationSampler {
    animation_type: AnimationType,
    interpolation: Interpolation,
    /// the affected node
    times: Vec<f32>,
    transforms: Vec<[f32; 4]>,
    current: Option<AnimationSample>,
}

struct AnimationSample {
    end_time: f32,
    transform_index: usize,
}

fn get_animation_times(times_accessor: &gltf::Accessor) -> (usize, usize) {
    assert_eq!(times_accessor.data_type(), DataType::F32);
    let length = times_accessor.count() * 4;
    let offset = times_accessor.offset() + (times_accessor.view().unwrap().offset());
    (offset, length)
}

fn get_animation_transforms(transforms_accessor: &gltf::Accessor) -> (usize, usize) {
    assert_eq!(transforms_accessor.data_type(), DataType::F32);
    let length = transforms_accessor.count() * 16;
    let offset = transforms_accessor.offset() + (transforms_accessor.view().unwrap().offset());
    (offset, length)
}
impl AnimationSampler {
    pub fn from_channels(channels: Vec<&gltf::animation::Channel>) -> Vec<Self> {
        let mut samplers: Vec<AnimationSampler> = Vec::new();
        for channel in channels.iter() {
            let animation_type = AnimationType::from(&channel.target().property());
            let interpolation = Interpolation::from(channel.sampler().interpolation());
            let times = get_animation_times(&channel.sampler().input());
            let transforms = get_animation_transforms(&channel.sampler().output());
            let sampler = AnimationSampler {
                animation_type,
                interpolation,
                times,
                transforms,
                current: None,
            };
            samplers.push(sampler);
        }
        samplers
    }

    fn sample(&mut self, timestamp: f32) {
        if timestamp <= self.current.as_ref().unwrap().end_time {
            return;
        }
        let idx = self.current.as_ref().unwrap().transform_index + 1;
        for i in idx..self.times.len() {
            if self.times[idx] > timestamp {
                self.current = Some(AnimationSample {
                    end_time: self.times[i],
                    transform_index: i,
                });
                return;
            }
        }
        self.current = None;
    }

    /// given the current Animation Sample,
    /// get the interpolated transform that should
    /// be applied to the meshes
    fn interpolate() -> LocalTransform {
        todo!()
    }
}
