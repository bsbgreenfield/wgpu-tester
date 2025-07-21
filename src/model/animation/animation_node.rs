use std::collections::HashMap;

use cgmath::{InnerSpace, SquareMatrix};
use gltf::{animation::Channel, Node};

use crate::model::{
    animation::{
        animation::AnimationInstance,
        animation_controller::{
            AnimationSample, AnimationSampler, AnimationTransforms, SampleResult,
        },
        util::{IDENTITY, NO_ROTATION, NO_TRANSLATION},
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
    pub rot: cgmath::Quaternion<f32>,
    pub trans: cgmath::Vector3<f32>,
    pub scale: cgmath::Vector3<f32>,
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
        let t = cgmath::Matrix4::<f32>::from_translation(self.trans);
        let r = cgmath::Matrix4::<f32>::from(self.rot);
        let s =
            cgmath::Matrix4::<f32>::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z);
        let transform = t * r * s;
        match self.node_type {
            NodeType::Mesh => mesh_transforms.push(transform.into()),
            NodeType::Joint(_) => {
                joint_transforms.push(transform.into());
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
        main_buffer_data: &Vec<u8>,
    ) -> bool {
        let relevant_channels: Vec<&Channel> = channels
            .iter()
            .filter(|c| c.target().node().index() == self.node_id)
            .collect();
        assert!(relevant_channels.len() <= 3);
        let maybe_samplers: Option<Vec<AnimationSampler>> =
            AnimationSampler::from_channels(&relevant_channels, buffer_offsets, main_buffer_data);
        if let Some(samplers) = maybe_samplers {
            *is_animated = true;
            self.add_sampler_set(channels[0].animation().index(), samplers);
        }
        for node_child in self.children.iter_mut() {
            node_child.attach_sampler_sets(channels, is_animated, buffer_offsets, main_buffer_data);
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
        skin_ibms: &HashMap<usize, Vec<cgmath::Matrix4<f32>>>,
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

                            let amount: f32 = (instance.time_elapsed.as_secs_f32()
                                - sampler.times[i])
                                / (sampler.times[i + 1] - sampler.times[i]);
                            match &sampler.transforms {
                                AnimationTransforms::Rotation(quaternion_vec) => {
                                    let first_quat = quaternion_vec[i];
                                    let second_quat = quaternion_vec[i + 1];
                                    rotation = Some(first_quat.nlerp(second_quat, amount));
                                }
                                AnimationTransforms::Translation(translation_vec) => {
                                    let first_trans = &translation_vec[i];
                                    let second_trans = &translation_vec[i + 1];

                                    let t_diff = second_trans - first_trans;
                                    let t_interp = t_diff * amount;
                                    translation = Some(first_trans + t_interp);
                                }
                                AnimationTransforms::Scale(scale_vec) => {
                                    let first_scale = &scale_vec[i];
                                    let second_scale = &scale_vec[i + 1];

                                    let t_diff = first_scale - second_scale;
                                    let t_interp = t_diff * amount;
                                    scale = Some(first_scale + t_interp);
                                }
                            }
                            *instance.current_samples.get_mut(&sampler.id).unwrap() =
                                Some(current_sample);
                        }
                        SampleResult::Done(last_index) => {
                            match &sampler.transforms {
                                AnimationTransforms::Rotation(quats) => {
                                    rotation = Some(quats[last_index]);
                                }
                                AnimationTransforms::Translation(vecs) => {
                                    translation = Some(vecs[last_index]);
                                }
                                AnimationTransforms::Scale(s_vecs) => {
                                    scale = Some(s_vecs[last_index]);
                                }
                            }
                            *instance.current_samples.get_mut(&sampler.id).unwrap() = None;
                        }
                    }
                }
                current_frame_transform = Some(
                    cgmath::Matrix4::from_translation(translation.unwrap_or(self.trans))
                        * cgmath::Matrix4::from(rotation.unwrap_or(self.rot))
                        * match scale {
                            Some(s) => cgmath::Matrix4::<f32>::from_nonuniform_scale(s.x, s.y, s.z),
                            None => cgmath::Matrix4::<f32>::from_nonuniform_scale(
                                self.scale[0],
                                self.scale[1],
                                self.scale[2],
                            ),
                        },
                );
            }
        }

        let animation_transform = current_frame_transform.unwrap_or(self.static_transform());

        let global = base_translation * animation_transform;
        match self.node_type {
            NodeType::Mesh => {
                let mesh_id = animation_data
                    .mesh_animation_data
                    .node_to_lt_index
                    .get(&self.node_id)
                    .unwrap();
                if animation_data.is_skeletal {
                    instance.mesh_transforms[*mesh_id] = cgmath::Matrix4::<f32>::identity().into();
                } else {
                    instance.mesh_transforms[*mesh_id] = global.into();
                }
            }
            NodeType::Joint(ibm_idx) => {
                let inverse_bind_matrix: cgmath::Matrix4<f32> = skin_ibms.get(&0).unwrap()[ibm_idx];
                // get the index of this joint within the joint transforms buffer
                instance.joint_transforms[ibm_idx] = (global * inverse_bind_matrix).into();
            }
            NodeType::Node => {}
        }
        // apply the new transform to the base translation using the optional TRS components
        // assign the mesh transform to the proper slot for this in.stance
        // if any one of the child nodes is still processing, set done to false
        for child_node in &self.children {
            if !child_node.update_node_transforms(instance, global, animation_data, skin_ibms) {
                node_is_done = false;
            }
        }
        node_is_done
    }

    fn static_transform(&self) -> cgmath::Matrix4<f32> {
        let a = cgmath::Matrix4::<f32>::from_translation(self.trans)
            * cgmath::Matrix4::<f32>::from(self.rot)
            * cgmath::Matrix4::<f32>::from_nonuniform_scale(
                self.scale[0],
                self.scale[1],
                self.scale[2],
            );
        //println!("{:?}", a.x);
        //println!("{:?}", a.y);
        //println!("{:?}", a.z);
        //println!("{:?}\n\n", a.w);
        a
    }

    pub fn new(
        node: &Node,
        children: Vec<AnimationNode>,
        joint_to_joint_indices: &HashMap<usize, usize>,
    ) -> Self {
        let decomposed = node.transform().decomposed();
        let t = decomposed.0;
        let r = decomposed.1;
        let s = decomposed.2;
        let trans = cgmath::Vector3::<f32>::new(t[0], t[1], t[2]);
        let rot = cgmath::Quaternion::<f32>::new(r[3], r[0], r[1], r[2]);
        let scale = cgmath::Vector3::<f32>::new(s[0], s[1], s[2]);
        match joint_to_joint_indices.get(&node.index()) {
            Some(joint_index) => AnimationNode {
                children,
                samplers: None,
                trans,
                rot,
                scale,
                node_type: NodeType::Joint(*joint_index),
                node_id: node.index(),
            },
            None => match node.mesh() {
                Some(_) => AnimationNode {
                    children,
                    trans,
                    rot,
                    scale,
                    samplers: None,
                    node_type: NodeType::Mesh,
                    node_id: node.index(),
                },

                None => AnimationNode {
                    children,
                    trans,
                    rot,
                    scale,
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
                println!("     Animation #{}: {} sampler(s)", entry.0, entry.1.len());
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
