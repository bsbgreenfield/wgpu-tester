use std::{collections::HashMap, time::Duration};

use cgmath::{InnerSpace, Matrix, Vector3};
use gltf::{animation::Channel, Node};

use crate::model::{
    animation::{
        animation::AnimationInstance,
        util::{
            get_animation_times, get_animation_transforms, AnimationType, Interpolation, IDENTITY,
            NO_ROTATION, NO_TRANSLATION,
        },
    },
    model::ModelAnimationData,
};

#[derive(Debug, PartialEq, Eq, PartialOrd)]
pub enum NodeType {
    Node,
    Mesh,
    Joint(usize),
}
type ModelAnimationMap = HashMap<usize, Vec<AnimationSampler>>;
pub struct AnimationNode {
    pub children: Vec<AnimationNode>,
    transform: cgmath::Matrix4<f32>,
    pub samplers: Option<ModelAnimationMap>,
    pub node_type: NodeType,
    pub(super) node_id: usize,
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
                    let default = AnimationSample {
                        end_time: sampler.times[0],
                        transform_index: -1,
                    };
                    map.insert(sampler.id, Some(default));
                }
            }
        }
        for child_node in &self.children {
            child_node.get_default_samples(animation_index, map);
        }
    }
    pub(super) fn initialize_sampled_transforms(
        &self,
        mesh_transforms: &mut Vec<[[f32; 4]; 4]>,
        joint_transforms: &mut Vec<[[f32; 4]; 4]>,
    ) {
        match self.node_type {
            NodeType::Mesh => mesh_transforms.push(self.transform.into()),
            NodeType::Joint(joint_idx) => {
                joint_transforms.push(self.transform.into());
            }

            NodeType::Node => {}
        }
        for child in self.children.iter() {
            child.initialize_sampled_transforms(mesh_transforms, joint_transforms);
        }
    }

    pub fn attach_sampler_sets(
        &mut self,
        channels: &Vec<Channel>,
        is_animated: &mut bool,
        buffer_offsets: &Vec<u64>,
    ) -> bool {
        let relevant_channels: Vec<&Channel> = channels
            .iter()
            .filter(|c| c.target().node().index() == self.node_id)
            .collect();
        let maybe_samplers: Option<Vec<AnimationSampler>> =
            AnimationSampler::from_channels(&relevant_channels, buffer_offsets);
        if let Some(samplers) = maybe_samplers {
            *is_animated = true;
            self.add_sampler_set(channels[0].animation().index(), samplers);
        }
        for node_child in self.children.iter_mut() {
            node_child.attach_sampler_sets(channels, is_animated, buffer_offsets);
        }
        *is_animated // will be false only if there are no samplers for any for the nodes
    }
    pub(super) fn add_sampler_set(
        &mut self,
        animation_index: usize,
        samplers: Vec<AnimationSampler>,
    ) {
        match &mut self.samplers {
            Some(sampler_map) => {
                if let Some(_) = sampler_map.get_mut(&animation_index) {
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

    pub(super) fn update_node_transforms(
        &self,
        instance: &mut AnimationInstance,
        base_translation: cgmath::Matrix4<f32>,
        animation_data: &ModelAnimationData,
        skin_ibms: &HashMap<usize, Vec<[[f32; 4]; 4]>>,
    ) -> bool {
        let mut node_is_done: bool = true;

        // optionaly allocate for a unique transform composed of TRS components
        // otherwise this nodes transform is base * current
        let mut current_frame_transform: Option<cgmath::Matrix4<f32>> = None;

        if let Some(sample_map) = &self.samplers {
            if let Some(sampler_set) = sample_map.get(&instance.animation_index) {
                let mut rotation: Option<cgmath::Quaternion<f32>> = None;
                let mut translation: Option<cgmath::Vector3<f32>> = None;
                let mut scale: Option<cgmath::Vector3<f32>> = None;
                for sampler in sampler_set {
                    let corresponding_sample = instance.current_samples.get(&sampler.id).unwrap();
                    // if the sampler is finished, skip it.
                    if corresponding_sample.is_none() {
                        continue;
                    }
                    // otherwise, get the new sample, which may be Active or Done
                    let maybe_current_sample =
                        sampler.sample((*corresponding_sample).unwrap(), instance.time_elapsed);
                    // if it is Active, do the transforms, unless it hasnt started yet (idx is -1)
                    // if its Done apply the final transform and set the sample to None in the
                    // instace map

                    match maybe_current_sample {
                        SampleResult::Active(current_sample) => {
                            node_is_done = false;
                            if current_sample.transform_index == -1 {
                                continue;
                            }
                            let i = current_sample.transform_index as usize;
                            let first_transform = sampler.transforms[i];
                            let second_transform = sampler.transforms[i + 1];
                            let amount: f32 = (instance.time_elapsed.as_secs_f32()
                                - sampler.times[i])
                                / (sampler.times[i + 1] - sampler.times[i]);
                            match sampler.animation_type {
                                AnimationType::Rotation => {
                                    let q1 = cgmath::Quaternion::from(first_transform).normalize();
                                    let q2 = cgmath::Quaternion::from(second_transform).normalize();
                                    rotation = Some(q1.nlerp(q2, amount.abs()));
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
                                AnimationType::Scale => {
                                    let t_diff = cgmath::Vector3::<f32>::from([
                                        second_transform[0] - first_transform[0],
                                        second_transform[1] - first_transform[1],
                                        second_transform[2] - first_transform[2],
                                    ]);
                                    let t_interp = t_diff * amount;
                                    scale = Some(cgmath::Vector3::from([
                                        first_transform[0] + t_interp[0],
                                        first_transform[1] + t_interp[1],
                                        first_transform[2] + t_interp[2],
                                    ]));
                                }
                            };
                            *instance.current_samples.get_mut(&sampler.id).unwrap() =
                                Some(current_sample);
                        }
                        SampleResult::Done(last_index) => {
                            let transform = sampler.transforms[last_index];
                            match sampler.animation_type {
                                AnimationType::Rotation => {
                                    let q1 = cgmath::Quaternion::from(transform).normalize();
                                    rotation = Some(q1);
                                }
                                AnimationType::Translation => {
                                    translation = Some(Vector3::new(
                                        transform[0],
                                        transform[1],
                                        transform[2],
                                    ));
                                }
                                AnimationType::Scale => {
                                    scale = Some(Vector3::new(
                                        transform[0],
                                        transform[1],
                                        transform[2],
                                    ));
                                }
                            };
                            *instance.current_samples.get_mut(&sampler.id).unwrap() = None;
                        }
                    }
                }
                current_frame_transform = Some(
                    cgmath::Matrix4::from_translation(translation.unwrap_or(NO_TRANSLATION))
                        * cgmath::Matrix4::from(rotation.unwrap_or(NO_ROTATION))
                        * match scale {
                            Some(s) => cgmath::Matrix4::<f32>::from_nonuniform_scale(s.x, s.y, s.z),
                            None => IDENTITY,
                        },
                );
            }
        }
        let new_base_transform = match self.node_type {
            NodeType::Mesh => {
                let nbt = base_translation * current_frame_transform.unwrap_or(self.transform);
                let mesh_id = animation_data
                    .mesh_animation_data
                    .node_to_lt_index
                    .get(&self.node_id)
                    .unwrap();
                instance.mesh_transforms[*mesh_id] = nbt.into();
                nbt
            }
            NodeType::Joint(ibm_idx) => {
                // get the inverse bind matrix for this joint
                let inverse_bind_matrix: cgmath::Matrix4<f32> = skin_ibms[&0][ibm_idx].into();
                // get the index of this joint within the joint transforms buffer
                let joint_index = animation_data
                    .joint_animation_data
                    .joint_to_joint_index
                    .get(&self.node_id)
                    .unwrap();

                // caluculate the globa transform of this joint node
                let local_transform = self.transform * current_frame_transform.unwrap_or(IDENTITY);
                let global_transform = base_translation * local_transform;
                instance.joint_transforms[*joint_index] =
                    (global_transform * inverse_bind_matrix).into();

                global_transform
            }
            NodeType::Node => base_translation * current_frame_transform.unwrap_or(self.transform),
        };
        // apply the new transform to the base translation using the optional TRS components
        // assign the mesh transform to the proper slot for this in.stance
        // if any one of the child nodes is still processing, set done to false
        for child_node in &self.children {
            if !child_node.update_node_transforms(
                instance,
                new_base_transform,
                animation_data,
                skin_ibms,
            ) {
                node_is_done = false;
            }
        }
        node_is_done
    }

    pub fn new(node: &Node, children: Vec<AnimationNode>, joint_ids: &Vec<usize>) -> Self {
        match joint_ids.binary_search(&node.index()) {
            Ok(ibm_idx) => AnimationNode {
                children,
                samplers: None,
                transform: cgmath::Matrix4::from(node.transform().matrix()),
                node_type: NodeType::Joint(ibm_idx),
                node_id: node.index(),
            },

            Err(_) => match node.mesh() {
                Some(_) => AnimationNode {
                    children,
                    transform: cgmath::Matrix4::from(node.transform().matrix()),
                    samplers: None,
                    node_type: NodeType::Mesh,
                    node_id: node.index(),
                },

                None => AnimationNode {
                    children,
                    transform: cgmath::Matrix4::from(node.transform().matrix()),
                    samplers: None,
                    node_type: NodeType::Node,
                    node_id: node.index(),
                },
            },
        }
    }

    pub fn print(&self) {
        println!("node {}", self.node_id);
        if let Some(samplers) = &self.samplers {
            for entry in samplers {
                println!("{}: {}", entry.0, entry.1.len());
            }
        }
        if self.children.len() > 0 {
            println!("children:");
            for child in self.children.iter() {
                child.print();
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub(super) enum SampleResult {
    Active(AnimationSample),
    Done(usize),
}

#[derive(Debug, Clone, Copy)]
pub(super) struct AnimationSample {
    pub(super) end_time: f32,
    pub(super) transform_index: i32,
}

#[derive(Debug)]
pub struct AnimationSampler {
    pub(super) id: usize,
    pub animation_type: AnimationType,
    pub interpolation: Interpolation,
    /// the affected node
    pub times: Vec<f32>,
    pub transforms: Vec<[f32; 4]>,
}
impl AnimationSampler {
    pub fn from_channels(channels: &Vec<&Channel>, buffer_offsets: &Vec<u64>) -> Option<Vec<Self>> {
        let mut samplers: Vec<AnimationSampler> = Vec::new();
        for channel in channels.iter() {
            let animation_type = AnimationType::from_property(&channel.target().property());
            let interpolation = Interpolation::from(channel.sampler().interpolation());
            let times = get_animation_times(&channel.sampler().input(), buffer_offsets);
            let transforms = get_animation_transforms(
                &channel.sampler().output(),
                buffer_offsets,
                &channel.target().property(),
            );
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

    pub(super) fn sample(
        &self,
        current_sample: AnimationSample,
        time_elapsed: Duration,
    ) -> SampleResult {
        // if the current time elapsed has surpassed the threshold time for this sample
        // we need to calculate a new sample
        if time_elapsed.as_secs_f32() >= current_sample.end_time {
            let idx = (current_sample.transform_index + 1) as usize;
            // loop through the times after the current time
            // skipping the first time, as that is already the end time
            // if we hit a time that is greater than the time elapsed, at times[i]
            // we know that times[i] is our new end time, and i - 1 is our new t index
            // if we reach the end of the times, this sampler is done, return None
            for i in (idx..self.times.len()).skip(1) {
                if time_elapsed.as_secs_f32() > self.times[i] {
                    continue;
                } else {
                    return SampleResult::Active(AnimationSample {
                        end_time: self.times[i],
                        transform_index: (i as i32 - 1),
                    });
                }
            }
            return SampleResult::Done(self.times.len() - 1);
        }
        return SampleResult::Active(current_sample);
    }
}
