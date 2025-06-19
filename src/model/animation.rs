use cgmath::{Quaternion, SquareMatrix, Vector3, Zero};
use gltf::{
    accessor::{DataType, Iter},
    animation::Channel,
    Node,
};

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
    transform: cgmath::Matrix4<f32>,
    pub samplers: Option<Vec<AnimationSampler>>,
    node_type: NodeType,
    node_id: usize,
    mesh_id: Option<usize>,
}
impl AnimationNode {
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

/// Keeps track of which animations are currently playing.
/// The controllers functions are
/// 1. adding or removing active animation indices based on user input and time
/// 2. owning all animation structs
/// 3. interface between animations and the app.
pub struct SceneAnimationController {
    active_animations: Vec<usize>,
    active_model_instances: Vec<usize>,
    animations: Vec<SimpleAnimation>,
}

impl SceneAnimationController {
    fn animate(&mut self, timestamp: f32) {
        for i in &mut self.active_animations {
            let mut animation_frame = AnimationFrame {
                model_id: self.active_model_instances[*i],
                transforms: Vec::new(),
            };
            get_animation_frame(
                &mut self.animations[*i].root_node,
                timestamp,
                &mut animation_frame,
            );
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

struct SimpleAnimation {
    root_node: AnimationNode,
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
}
