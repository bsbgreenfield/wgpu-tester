use std::{ops::Range, rc::Rc, thread::current};

use gltf::{
    accessor::{DataType, Iter},
    animation::Channel,
    Node,
};

use crate::{model::model::LocalTransform, transforms};

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
#[derive(Debug)]
pub enum NodeType {
    Node,
    Mesh,
}
pub struct AnimationNode {
    pub children: Vec<AnimationNode>,
    transform: [[f32; 4]; 4],
    pub samplers: Option<Vec<AnimationSampler>>,
    node_type: NodeType,
    node_id: usize,
    mesh_id: Option<usize>,
}
impl AnimationNode {
    pub fn new(node: &Node, children: Vec<AnimationNode>) -> Self {
        match node.mesh() {
            Some(mesh) => AnimationNode {
                children,
                transform: node.transform().matrix(),
                samplers: None,
                node_type: NodeType::Mesh,
                node_id: node.index(),
                mesh_id: Some(node.mesh().unwrap().index()),
            },
            None => AnimationNode {
                children,
                transform: node.transform().matrix(),
                samplers: None,
                node_type: NodeType::Node,
                node_id: node.index(),
                mesh_id: None,
            },
        }
    }

    pub fn print(&self) {
        println!("node {} with sampler {:?}", self.node_id, self.samplers);
        if self.children.len() > 0 {
            println!("children of this node:");
            for child in self.children.iter() {
                child.print();
            }
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

#[derive(Debug)]
pub enum AnimationType {
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

#[derive(Debug)]
pub struct AnimationSampler {
    pub animation_type: AnimationType,
    interpolation: Interpolation,
    /// the affected node
    pub times: Vec<f32>,
    pub transforms: Vec<[f32; 4]>,
    current: Option<AnimationSample>,
}

#[derive(Debug)]
struct AnimationSample {
    end_time: f32,
    transform_index: usize,
}
pub fn attach_samplers(animation_node: &mut AnimationNode, channels: &Vec<Channel>) {
    let relevant_channels: Vec<&Channel> = channels
        .iter()
        .filter(|c| c.target().node().index() == animation_node.node_id)
        .collect();
    if relevant_channels.len() > 0 {
        let animation_samplers: Vec<AnimationSampler> =
            AnimationSampler::from_channels(relevant_channels);
        animation_node.samplers = Some(animation_samplers);
    }
    for node_child in animation_node.children.iter_mut() {
        attach_samplers(node_child, channels);
    }
}

fn get_animation_times(times_accessor: &gltf::Accessor) -> (usize, usize) {
    assert_eq!(times_accessor.data_type(), DataType::F32);
    let length = times_accessor.count() * 4;
    let offset = times_accessor.offset() + (times_accessor.view().unwrap().offset());
    (offset, length)
}

fn get_animation_transforms(
    transforms_accessor: &gltf::Accessor,
    animation_type: &gltf::animation::Property,
) -> (usize, usize) {
    assert_eq!(transforms_accessor.data_type(), DataType::F32);
    let length = match *animation_type {
        gltf::animation::Property::Rotation => transforms_accessor.count() * 16, // there should be
        // 16 bytes of data
        gltf::animation::Property::Translation => transforms_accessor.count() * 12, // there should
        // be 123 bytes of data
        _ => todo!("havent implemented scale or morph yet"),
    };

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
            let transforms =
                get_animation_transforms(&channel.sampler().output(), &channel.target().property());
            let sampler = AnimationSampler {
                animation_type,
                interpolation,
                times: vec![times.0 as f32, times.1 as f32],
                transforms: vec![[transforms.0 as f32, transforms.1 as f32, 0f32, 0f32]],
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
