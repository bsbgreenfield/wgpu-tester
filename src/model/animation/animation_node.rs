use std::{collections::HashMap, time::Duration};

use gltf::{animation::Channel, Node};

use crate::model::animation::{
    animation_controller::AnimationInstance,
    util::{
        get_animation_times, get_animation_transforms, AnimationType, Interpolation, NodeType,
        IDENTITY, NO_ROTATION, NO_TRANSLATION,
    },
};

type ModelAnimationMap = HashMap<usize, Vec<AnimationSampler>>;
pub struct AnimationNode {
    pub children: Vec<AnimationNode>,
    transform: cgmath::Matrix4<f32>,
    pub samplers: Option<ModelAnimationMap>,
    node_type: NodeType,
    pub(super) node_id: usize,
    mesh_id: Option<usize>,
}
impl AnimationNode {
    pub(super) fn get_default_samples(
        &self,
        animation_index: usize,
        map: &mut HashMap<usize, Option<AnimationSample>>,
    ) {
        if let Some(sampler_map) = &self.samplers {
            if let Some(samplers) = sampler_map.get(&animation_index) {
                for sampler in samplers {
                    map.insert(
                        sampler.id,
                        Some(AnimationSample {
                            end_time: 0.0,
                            transform_index: 0,
                        }),
                    );
                }
            }
        }
        for child_node in &self.children {
            child_node.get_default_samples(animation_index, map);
        }
    }
    pub(super) fn initialize_sampled_transforms(&self, transforms: &mut Vec<[[f32; 4]; 4]>) {
        if self.samplers.is_some() {
            transforms.push(self.transform.clone().into());
        }
        for child in self.children.iter() {
            child.initialize_sampled_transforms(transforms);
        }
    }

    pub fn attach_sampler_sets(&mut self, channels: &Vec<Channel>, is_animated: &mut bool) {
        let relevant_channels: Vec<&Channel> = channels
            .iter()
            .filter(|c| c.target().node().index() == self.node_id)
            .collect();
        let maybe_samplers: Option<Vec<AnimationSampler>> =
            AnimationSampler::from_channels(&relevant_channels);
        if let Some(samplers) = maybe_samplers {
            *is_animated = true;
            self.add_sampler_set(channels[0].animation().index(), samplers);
        }
        for node_child in self.children.iter_mut() {
            node_child.attach_sampler_sets(channels, is_animated);
        }
    }
    pub(super) fn add_sampler_set(
        &mut self,
        animation_index: usize,
        samplers: Vec<AnimationSampler>,
    ) {
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
    // the only reason we can get away with not indexing the meshes
    // here is by relying on the fact that the node tree will traverse the same
    // every time
    /// build out a new set of mesh transforms for this instance.
    pub(super) fn update_mesh_transforms(
        &self,
        new_transforms: &mut Vec<[[f32; 4]; 4]>,
        instance: &mut AnimationInstance,
    ) {
        let mut rotation: Option<cgmath::Quaternion<f32>> = None;
        let mut translation: Option<cgmath::Vector3<f32>> = None;
        let mut scale: Option<cgmath::Vector3<f32>> = None;
        if let Some(sample_map) = &self.samplers {
            if let Some(sample_set) = sample_map.get(&instance.animation_index) {
                // for each sampler in the nodes samplers for this animation,
                // get the new Option<sample> and if some, use that value to
                // calculate a new transform
                for sampler in sample_set {
                    // get the Option<AnimationSample> for this sampler/intance
                    let maybe_current_sample = sampler.sample(
                        *instance.current_samples.get(&sampler.id).unwrap(),
                        instance.time_elapsed,
                    );
                    // if the sample is active, interpolate the new transform by populating
                    // Transform, Rotatiom, or Scale for this node
                    if let Some(current_sample) = maybe_current_sample {
                        let i = current_sample.transform_index;
                        let first_transform = sampler.transforms[current_sample.transform_index];
                        let second_transform =
                            sampler.transforms[current_sample.transform_index + 1];
                        let amount: f32 = (instance.time_elapsed.as_secs_f32() - sampler.times[i])
                            / (sampler.times[i + 1] - sampler.times[i])
                            - sampler.times[i];
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
                    *instance.current_samples.get_mut(&sampler.id).unwrap() = maybe_current_sample;
                }
            }
            // after looping over all the active samplers for this node
            // and assigning the new Option<AnimationSample> to the intance sample map,
            // compose a transform matrix from the TRS components

            let transform =
                cgmath::Matrix4::from_translation(translation.unwrap_or(NO_TRANSLATION))
                    * cgmath::Matrix4::from(rotation.unwrap_or(NO_ROTATION))
                    * match scale {
                        Some(s) => cgmath::Matrix4::<f32>::from_nonuniform_scale(s.x, s.y, s.z),
                        None => IDENTITY,
                    };
            // if this is a regular node, pass this transform along as the base translation.
            // if this is a mesh, add it to the list and do the same
            if self.node_type == NodeType::Mesh {
                new_transforms.push(transform.into());
            }
            for child_node in self.children.iter() {
                child_node.update_mesh_transforms(new_transforms, instance);
            }
        }
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

#[derive(Debug, Clone, Copy)]
pub(super) struct AnimationSample {
    end_time: f32,
    transform_index: usize,
}
#[derive(Debug)]
pub struct AnimationSampler {
    id: usize,
    pub animation_type: AnimationType,
    pub interpolation: Interpolation,
    /// the affected node
    pub times: Vec<f32>,
    pub transforms: Vec<[f32; 4]>,
}
impl AnimationSampler {
    pub fn from_channels(channels: &Vec<&Channel>) -> Option<Vec<Self>> {
        let mut samplers: Vec<AnimationSampler> = Vec::new();
        for channel in channels.iter() {
            let animation_type = AnimationType::from_property(&channel.target().property());
            let interpolation = Interpolation::from(channel.sampler().interpolation());
            let times = get_animation_times(&channel.sampler().input());
            let transforms =
                get_animation_transforms(&channel.sampler().output(), &channel.target().property());
            let sampler = AnimationSampler {
                id: channel.sampler().index(),
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

    /// sample this sampler using the last frames sample as a baseline.
    /// if None, this sampler is done, so we can skip it.
    /// if the threshold time for the last sample hasn't been surpassed, return the same sample
    /// if the threshold time has been surpassed, either return the index or None
    fn sample(
        &self,
        current: Option<AnimationSample>,
        time_elapsed: Duration,
    ) -> Option<AnimationSample> {
        match current {
            None => None,
            Some(current_sample) => {
                if time_elapsed.as_secs_f32() > current_sample.end_time {
                    let idx = current_sample.transform_index + 1;
                    for i in idx..self.times.len() {
                        if self.times[idx] > time_elapsed.as_secs_f32() {
                            return Some(AnimationSample {
                                end_time: self.times[i],
                                transform_index: i,
                            });
                        }
                    }
                    return None;
                }
                return current;
            }
        }
    }
}
