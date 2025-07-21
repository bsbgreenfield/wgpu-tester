use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, UNIX_EPOCH},
};

use gltf::animation::Channel;

use crate::model::{
    animation::{
        animation::*,
        util::{AnimationType, InterpolationType},
    },
    model::{GModel, ModelAnimationData},
    util::{copy_binary_data_from_gltf, AttributeType},
};

/// Keeps track of which animations are currently playing.
/// The controllers functions are
/// 1. adding or removing active animation indices based on user input and time
/// 2. owning all animation structs
/// 3. interface between animations and the app.
pub struct SceneAnimationController {
    dead_animations: Vec<usize>,
    pub(super) active_animations: Vec<VecDeque<AnimationInstance>>,
    pub(super) active_animation_count: usize,
    pub(super) skin_ibms: HashMap<usize, Vec<cgmath::Matrix4<f32>>>,
}

impl SceneAnimationController {
    pub fn new(model_no: usize, skin_ibms: HashMap<usize, Vec<cgmath::Matrix4<f32>>>) -> Self {
        let mut active_animations = Vec::with_capacity(model_no);
        for _ in 0..model_no {
            active_animations.push(VecDeque::with_capacity(10));
        }
        Self {
            dead_animations: vec![0; model_no],
            active_animations,
            active_animation_count: 0,
            skin_ibms,
        }
    }

    pub fn initialize_animation(
        &mut self,
        animation_data: &ModelAnimationData,
        animation_index: usize,
        model_instance_offset: usize,
        model_mesh_instance_count: usize,
        model_joint_instance_count: usize,
    ) {
        let animation_node = animation_data.animation_node.clone();
        let mut mesh_transforms: Vec<[[f32; 4]; 4]> = Vec::with_capacity(model_mesh_instance_count);
        let mut joint_transforms: Vec<[[f32; 4]; 4]> =
            Vec::with_capacity(model_joint_instance_count);
        let mut sample_map = HashMap::<usize, Option<AnimationSample>>::new();
        animation_node.get_default_samples(animation_index, &mut sample_map);
        animation_node.initialize_sampled_transforms(&mut mesh_transforms, &mut joint_transforms);
        let start_time = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap();

        let animation_instance = AnimationInstance::new(
            animation_node,
            model_instance_offset,
            start_time,
            Duration::ZERO,
            animation_index,
            mesh_transforms,
            joint_transforms,
            sample_map,
        );
        self.active_animations[animation_data.model_index].push_back(animation_instance);
        self.active_animation_count += 1;
    }

    pub fn do_animations<'a>(
        &'a mut self,
        timestamp: Duration,
        models: &'a Vec<GModel>,
    ) -> Option<AnimationFrame<'a>> {
        // process any animations that were marked as done last frame
        for (idx, dead_animation_count) in self.dead_animations.iter_mut().enumerate() {
            let count = dead_animation_count.clone();
            for _ in 0..count {
                self.active_animations[idx].pop_front();
                *dead_animation_count -= 1;
                self.active_animation_count -= 1;
            }
        }

        // if there are no active animations, do nothing
        if self.active_animation_count == 0 {
            return None;
        }
        let len = self.active_animations.len();

        let mut frame = AnimationFrame {
            mesh_transform_slices: Vec::with_capacity(len),
            joint_transform_slices: Vec::with_capacity(len),
            joint_ids: Vec::new(),
            lt_offsets: Vec::with_capacity(len),
        };

        for (idx, bucket) in self.active_animations.iter_mut().enumerate() {
            if bucket.len() == 0 {
                continue;
            }
            let animation_data = &models[idx].animation_data.as_ref().unwrap();
            for animation_instance in bucket.iter_mut() {
                frame
                    .lt_offsets
                    .push(animation_instance.model_instance_offset);
                let animation_processing_result = animation_instance.process_animation_frame(
                    timestamp,
                    animation_data,
                    &self.skin_ibms,
                );
                frame
                    .mesh_transform_slices
                    .push(animation_processing_result.mesh_transforms);
                frame
                    .joint_transform_slices
                    .push(animation_processing_result.joint_transforms);
                frame
                    .joint_ids
                    .push(animation_processing_result.joint_indices);
                if animation_processing_result.is_done {
                    self.dead_animations[idx] += 1;
                }
            }
        }
        Some(frame)
    }
}

#[derive(Copy, Clone, Debug)]
pub(super) enum SampleResult {
    Active(AnimationSample),
    Done(usize),
}

#[derive(Debug)]
pub enum AnimationTransforms {
    Rotation(Vec<cgmath::Quaternion<f32>>),
    Translation(Vec<cgmath::Vector3<f32>>),
    Scale(Vec<cgmath::Vector3<f32>>),
}
impl AnimationTransforms {
    pub(super) fn len(&self) -> usize {
        match self {
            Self::Rotation(r) => r.len(),
            Self::Translation(t) => t.len(),
            Self::Scale(s) => s.len(),
        }
    }

    fn from_byte_slice(attribute_type: AttributeType, slice: &[u8]) -> Self {
        let f32_slice: &[f32] = bytemuck::cast_slice(slice);
        match attribute_type {
            AttributeType::RotationT => {
                let mut quat_vec: Vec<cgmath::Quaternion<f32>> = Vec::new();
                for i in 0..f32_slice.len() / 4 {
                    let quat_slice: &[f32; 4] = &f32_slice[i * 4..i * 4 + 4].try_into().unwrap();
                    let quat = cgmath::Quaternion::<f32>::new(
                        quat_slice[3],
                        quat_slice[0],
                        quat_slice[1],
                        quat_slice[2],
                    );
                    quat_vec.push(quat);
                }
                Self::Rotation(quat_vec)
            }
            AttributeType::TranslationT => {
                let mut trans_vec: Vec<cgmath::Vector3<f32>> = Vec::new();
                for i in 0..f32_slice.len() / 3 {
                    let trans_slice: &[f32; 3] = &f32_slice[i * 3..i * 3 + 3].try_into().unwrap();
                    let vec =
                        cgmath::Vector3::<f32>::new(trans_slice[0], trans_slice[1], trans_slice[2]);
                    trans_vec.push(vec);
                }

                Self::Translation(trans_vec)
            }
            AttributeType::ScaleT => {
                let mut scale_vec: Vec<cgmath::Vector3<f32>> = Vec::new();
                for i in 0..f32_slice.len() / 3 {
                    let scale_slice: &[f32; 3] = &f32_slice[i * 3..i * 3 + 3].try_into().unwrap();
                    let vec =
                        cgmath::Vector3::<f32>::new(scale_slice[0], scale_slice[1], scale_slice[2]);
                    scale_vec.push(vec);
                }

                Self::Scale(scale_vec)
            }
            _ => panic!("unable to create a transform from this attribute type"),
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub(super) struct AnimationSample {
    pub(super) end_time: f32,
    pub(super) transform_index: i32,
}

#[derive(Debug)]
pub struct AnimationSampler {
    pub(super) id: usize,
    pub interpolation: InterpolationType,
    /// the affected node
    pub times: Vec<f32>,
    pub transforms: AnimationTransforms,
}
impl AnimationSampler {
    pub fn from_channels(
        channels: &Vec<&Channel>,
        buffer_offsets: &Vec<u64>,
        main_buffer_data: &Vec<u8>,
    ) -> Option<Vec<Self>> {
        let mut samplers: Vec<AnimationSampler> = Vec::new();
        for channel in channels.iter() {
            let times_u8 = copy_binary_data_from_gltf(
                &channel.sampler().input(),
                crate::model::util::AttributeType::Times,
                buffer_offsets,
                main_buffer_data,
            )
            .expect("Should be times data");
            let attrib_type = AttributeType::from_animation_channel(channel);
            match attrib_type {
                AttributeType::RotationT | AttributeType::TranslationT | AttributeType::ScaleT => {
                    assert!(
                        channel.sampler().output().data_type() == gltf::accessor::DataType::F32
                    );
                }
                _ => panic!("unexpected attribute"),
            }
            let transforms_u8 = copy_binary_data_from_gltf(
                &channel.sampler().output(),
                attrib_type,
                buffer_offsets,
                main_buffer_data,
            )
            .expect("Should be transforms");

            let interp = InterpolationType::from(channel.sampler().interpolation());
            let sampler = AnimationSampler {
                id: channel.sampler().index(),
                interpolation: interp,
                times: bytemuck::cast_slice::<u8, f32>(&times_u8).to_vec(),
                transforms: AnimationTransforms::from_byte_slice(attrib_type, &transforms_u8),
            };
            assert_eq!(
                sampler.times.len(),
                sampler.transforms.len(),
                "{:?} There are {} times and {} transforms",
                attrib_type,
                sampler.times.len(),
                sampler.transforms.len()
            );
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
