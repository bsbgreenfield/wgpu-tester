use std::{ops::Range, rc::Rc, thread::current};

use gltf::accessor::Iter;

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
    samplers: Vec<AnimationSampler>,
}
enum AnimationType {
    Rotation,
    Translation,
    Scale,
    // others?
}
struct AnimationSampler {
    animation_type: AnimationType,
    interpolation: Interpolation,
    meshes: Vec<usize>,
    times: Vec<f32>,
    transforms: Vec<[f32; 4]>,
    current: Option<AnimationSample>,
}

struct AnimationSample {
    end_time: f32,
    transform_index: usize,
}

impl AnimationSampler {
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
