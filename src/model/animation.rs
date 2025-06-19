use std::{collections::HashMap, rc::Rc};

use cgmath::{Quaternion, SquareMatrix, Vector3, Zero};
use gltf::{
    accessor::{DataType, Iter},
    animation::Channel,
    Node,
};
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
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

type ModelAnimationMap = HashMap<usize, Vec<AnimationSampler>>;
pub struct AnimationNode {
    pub children: Vec<AnimationNode>,
    transform: cgmath::Matrix4<f32>,
    pub samplers: Option<ModelAnimationMap>,
    node_type: NodeType,
    node_id: usize,
    mesh_id: Option<usize>,
}
impl AnimationNode {
    fn initialize_sampled_transforms(&self, transforms: &mut Vec<[[f32; 4]; 4]>) {
        if self.samplers.is_some() {
            transforms.push(self.transform.clone().into());
        }
        for child in self.children.iter() {
            child.initialize_sampled_transforms(transforms);
        }
    }

    fn add_sampler_set(&mut self, animation_index: usize, samplers: Vec<AnimationSampler>) {
        match &mut self.samplers {
            Some(sampler_map) => {
                if let Some(sampler_set) = sampler_map.get_mut(&animation_index) {
                    //TODO: return a result
                    panic!("we already assigned the samplers for this animation!");
                } else {
                    sampler_map.insert(animation_index, samplers);
                }
            }
            None => {
                let mut new_map: HashMap<usize, Vec<AnimationSampler>> = HashMap::new();
                new_map.insert(animation_index, samplers);
                self.samplers = Some(new_map);
            }
        }
    }

    fn update_mesh_transforms(
        &self,
        new_transforms: &mut Vec<[[f32; 4]; 4]>,
        instance: &AnimationInstance,
    ) {
        let mut rotation: Option<cgmath::Quaternion<f32>> = None;
        let mut translation: Option<cgmath::Vector3<f32>> = None;
        let mut scale: Option<cgmath::Matrix4<f32>> = None;
        if let Some(sample_map) = &self.samplers {
            if let Some(sample_set) = sample_map.get(&instance.animation_index) {
                for sampler in sample_set {
                    if let Some(current_sample) = &instance.current {
                        let first_transform = sampler.transforms[current_sample.transform_index];
                        let second_transform =
                            sampler.transforms[current_sample.transform_index + 1];
                        let amount: f32 = (instance.time_elapsed
                            - sampler.times[current_sample.transform_index])
                            / (sampler.times[current_sample.transform_index + 1]
                                - sampler.times[current_sample.transform_index])
                            - sampler.times[current_sample.transform_index];
                    }
                }
            }
        }
    }
    /// given the current Animation Sample,
    /// get the interpolated transform that should
    /// be applied to the meshes
    fn interpolate(&mut self, timestamp: f32) -> cgmath::Matrix4<f32> {
        let mut rotation: Option<cgmath::Quaternion<f32>> = None;
        let mut translation: Option<cgmath::Vector3<f32>> = None;
        let mut scale: Option<cgmath::Matrix4<f32>> = None;
        if let Some(samplers) = &mut self.samplers {
            for sampler in samplers.iter() {
                if let Some(current_sample) = &sampler.current {
                    let first_transform = sampler.transforms[current_sample.transform_index];
                    let second_transform = sampler.transforms[current_sample.transform_index + 1];
                    let amount: f32 = (timestamp - sampler.times[current_sample.transform_index])
                        / (sampler.times[current_sample.transform_index + 1]
                            - sampler.times[current_sample.transform_index])
                        - sampler.times[current_sample.transform_index];
                    match sampler.animation_type {
                        AnimationType::Rotation => {
                            let q1 = cgmath::Quaternion::from(first_transform);
                            let q2 = cgmath::Quaternion::from(second_transform);
                            rotation = Some(q1.nlerp(q2, amount));
                        }
                        AnimationType::Translation => {
                            let t_diff = cgmath::Vector3::<f32>::from([
                                second_transform[0] - first_transform[0],
                                second_transform[1] - first_transform[1],
                                second_transform[2] - first_transform[2],
                            ]);
                            let t_interp = t_diff * amount;
                            translation = Some(cgmath::Vector3::from([
                                first_transform[0] + t_interp[0],
                                first_transform[1] + t_interp[1],
                                first_transform[2] + t_interp[2],
                            ]));
                        }
                        _ => todo!("implement scaling!!!"),
                    };
                }
            }
        }
        self.transform = self.transform
            * cgmath::Matrix4::from(rotation.unwrap_or(Quaternion::zero()))
            * cgmath::Matrix4::from_translation(translation.unwrap_or(Vector3::zero()));
        self.transform
    }
    pub fn new(node: &Node, children: Vec<AnimationNode>) -> Self {
        match node.mesh() {
            Some(mesh) => AnimationNode {
                children,
                transform: cgmath::Matrix4::from(node.transform().matrix()),
                samplers: None,
                node_type: NodeType::Mesh,
                node_id: node.index(),
                mesh_id: Some(mesh.index()),
            },
            None => AnimationNode {
                children,
                transform: cgmath::Matrix4::from(node.transform().matrix()),
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

pub struct AnimationFrame {
    model_id: usize,
    transforms: Vec<(usize, [[f32; 4]; 4])>,
}

struct AnimationInstance {
    /// the node tree for the model
    animation_node: Rc<AnimationNode>,
    time_elapsed: f32,
    /// global index of the animation as defined in the gltf file
    animation_index: usize,
    /// the set of transforms affected by the samplers
    /// of this instances node tree
    mesh_transforms: Vec<[[f32; 4]; 4]>,
    ///
    current: Option<AnimationSample>,
}

impl AnimationInstance {
    fn process_animation_frame(&mut self, timestamp: f32) {
        todo!()
    }
}

/// Keeps track of which animations are currently playing.
/// The controllers functions are
/// 1. adding or removing active animation indices based on user input and time
/// 2. owning all animation structs
/// 3. interface between animations and the app.
pub struct SceneAnimationController {
    active_animations: Vec<AnimationInstance>,
    active_model_instances: Vec<usize>,
    animations: Vec<SimpleAnimation>,
}

impl SceneAnimationController {
    pub fn new(animations: Vec<SimpleAnimation>) -> Self {
        Self {
            active_animations: vec![],
            active_model_instances: vec![],
            animations,
        }
    }

    pub fn initiate_animation(&mut self, animation_index: usize) {
        // clone a shared reference to the animation node tree
        let animation_node = self.animations[animation_index].animation_node.clone();
        // get copies of the initial state of the animated nodes
        let mut sampled_transforms: Vec<[[f32; 4]; 4]> = Vec::new();
        &animation_node.initialize_sampled_transforms(&mut sampled_transforms);
        let animation_instance = AnimationInstance {
            animation_node,
            time_elapsed: 0f32,
            animation_index,
            sampled_transforms,
        };
        self.active_animations.push(animation_instance);
    }

    pub fn do_animations(&mut self, timestamp: f32) {
        for animation_instance in self.active_animations.iter_mut() {
            animation_instance.process_animation_frame(timestamp);
        }
    }
}

fn get_animation_frame(
    animation_node: &mut AnimationNode,
    timestamp: f32,
    animation_frame: &mut AnimationFrame,
) {
    match animation_node.node_type {
        NodeType::Mesh => animation_frame.transforms.push((
            animation_node.mesh_id.unwrap(),
            animation_node.transform.into(),
        )),
        NodeType::Node => {
            for child in animation_node.children.iter_mut() {
                get_animation_frame(child, timestamp, animation_frame);
            }
        }
    }
}

pub struct SimpleAnimation {
    pub animation_node: Rc<AnimationNode>,
    pub model_id: usize,
}
impl SimpleAnimation {
    pub fn new(animation_node: AnimationNode, model_id: usize) -> Self {
        Self {
            animation_node: Rc::new(animation_node),
            model_id,
        }
    }
}

#[derive(Debug)]
pub struct AnimationSampler {
    pub animation_type: AnimationType,
    pub interpolation: Interpolation,
    /// the affected node
    pub times: Vec<f32>,
    pub transforms: Vec<[f32; 4]>,
}

#[derive(Debug)]
struct AnimationSample {
    end_time: f32,
    transform_index: usize,
}
impl AnimationSampler {
    pub fn from_channels(channels: &Vec<&Channel>) -> Option<Vec<Self>> {
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
            };
            samplers.push(sampler);
        }
        if samplers.len() > 0 {
            return Some(samplers);
        } else {
            return None;
        }
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
}
/// for a given animation defined by a set of Channels

pub fn attach_sampler_sets(animation_node: &mut AnimationNode, channels: &Vec<Channel>) {
    let relevant_channels: Vec<&Channel> = channels
        .iter()
        .filter(|c| c.target().node().index() == animation_node.node_id)
        .collect();
    let maybe_samplers: Option<Vec<AnimationSampler>> =
        AnimationSampler::from_channels(&relevant_channels);
    if let Some(samplers) = maybe_samplers {
        animation_node.add_sampler_set(channels[0].animation().index(), samplers);
    }
    for node_child in animation_node.children.iter_mut() {
        attach_sampler_sets(node_child, channels);
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
