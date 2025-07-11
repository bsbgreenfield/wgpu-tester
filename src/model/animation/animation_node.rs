use std::{collections::HashMap, time::Duration};

use cgmath::{InnerSpace, Vector3};
use gltf::{animation::Channel, Node};

use crate::model::animation::{
    animation::AnimationInstance,
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
    pub(super) fn initialize_sampled_transforms(&self, transforms: &mut Vec<[[f32; 4]; 4]>) {
        if self.node_type == NodeType::Mesh {
            transforms.push(self.transform.into());
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

    pub(super) fn update_mesh_transforms(
        &self,
        instance: &mut AnimationInstance,
        base_translation: cgmath::Matrix4<f32>,
        node_to_lt_index_map: &HashMap<usize, usize>,
    ) -> bool {
        let mut node_is_done: bool = true;

        // optionaly allocate for a unique transform composed of TRS components
        // otherwise this nodes transform is base * current
        let mut composed_transform: Option<cgmath::Matrix4<f32>> = None;

        if let Some(sample_map) = &self.samplers {
            if let Some(sampler_set) = sample_map.get(&instance.animation_index) {
                let mut rotation: Option<cgmath::Quaternion<f32>> = None;
                let mut translation: Option<cgmath::Vector3<f32>> = None;
                let scale: Option<cgmath::Vector3<f32>> = None;
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
                                _ => todo!("implement scaling!!!"),
                            };
                            *instance.current_samples.get_mut(&sampler.id).unwrap() =
                                Some(current_sample);
                            node_is_done = false;
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
                                _ => todo!("implement scaling!!!"),
                            };
                            *instance.current_samples.get_mut(&sampler.id).unwrap() = None;
                        }
                    }
                }
                composed_transform = Some(
                    base_translation
                        * cgmath::Matrix4::from_translation(translation.unwrap_or(NO_TRANSLATION))
                        * cgmath::Matrix4::from(rotation.unwrap_or(NO_ROTATION))
                        * match scale {
                            Some(s) => cgmath::Matrix4::<f32>::from_nonuniform_scale(s.x, s.y, s.z),
                            None => IDENTITY,
                        },
                );
            }
        }
        // apply the new transform to the base translation using the optional TRS components
        // assign the mesh transform to the proper slot for this instance
        if self.node_type == NodeType::Mesh {
            instance.mesh_transforms[node_to_lt_index_map[&self.node_id]] = composed_transform
                .unwrap_or(base_translation * self.transform)
                .into();
        }
        for child_node in &self.children {
            if !child_node.update_mesh_transforms(
                instance,
                composed_transform.unwrap_or(base_translation * self.transform),
                node_to_lt_index_map,
            ) {
                node_is_done = false;
            }
        }
        node_is_done
    }

    pub fn new(node: &Node, children: Vec<AnimationNode>) -> Self {
        match node.mesh() {
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

#[derive(Copy, Clone, Debug)]
enum SampleResult {
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

    fn sample(&self, current_sample: AnimationSample, time_elapsed: Duration) -> SampleResult {
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
