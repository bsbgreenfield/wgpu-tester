use cgmath::{Matrix4, Quaternion, Vector3, Vector4};
use gltf::accessor::DataType;

use crate::model::animation::animation_node::AnimationNode;

pub(super) fn copy_data_for_animation(
    animation_node: &mut AnimationNode,
    model_id: usize,
    main_buffer_data: &Vec<u8>,
) {
    if let Some(sampler_map) = &mut animation_node.samplers {
        for sample_set in sampler_map {
            for sampler in sample_set.1 {
                let times_slice = bytemuck::cast_slice::<u8, f32>(
                    &main_buffer_data[(sampler.times[0] as usize)
                        ..((sampler.times[0] + sampler.times[1]) as usize)],
                );
                match sampler.animation_type {
                    AnimationType::Rotation => {
                        let transforms_slice = bytemuck::cast_slice::<u8, [f32; 4]>(
                            &main_buffer_data[(sampler.transforms[0][0] as usize)
                                ..((sampler.transforms[0][0] + sampler.transforms[0][1]) as usize)],
                        );
                        sampler.transforms = transforms_slice.to_vec();
                    }
                    AnimationType::Translation => {
                        let mut padded_slices: Vec<[f32; 4]> = Vec::new();
                        let transforms_slice = bytemuck::cast_slice::<u8, [f32; 3]>(
                            &main_buffer_data[(sampler.transforms[0][0] as usize)
                                ..((sampler.transforms[0][0] + sampler.transforms[0][1]) as usize)],
                        );
                        for slice in transforms_slice.iter() {
                            padded_slices.push([slice[0], slice[1], slice[2], 0.0]);
                        }
                        sampler.transforms = padded_slices;
                    }
                    _ => todo!("havent implemented this type of animation yet (scale?)"),
                }
                sampler.times = times_slice.to_vec();
                assert_eq!(
                    sampler.times.len(),
                    sampler.transforms.len(),
                    "There should be an equal number of keyframe times as transforms"
                );
            }
        }
    }
    for child_node in animation_node.children.iter_mut() {
        copy_data_for_animation(child_node, model_id, main_buffer_data);
    }
}
pub(super) fn get_animation_times(
    times_accessor: &gltf::Accessor,
    buffer_offsets: &Vec<u64>,
) -> (usize, usize) {
    assert_eq!(times_accessor.data_type(), DataType::F32);
    let buffer_view = times_accessor.view().unwrap();
    let buffer_offset = buffer_offsets[buffer_view.buffer().index()] as usize;
    let length = times_accessor.count() * 4;
    let offset = times_accessor.offset() + (buffer_view.offset()) + buffer_offset;
    (offset, length)
}

pub(super) fn get_animation_transforms(
    transforms_accessor: &gltf::Accessor,
    buffer_offsets: &Vec<u64>,
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
    let buffer_view = transforms_accessor.view().unwrap();
    let buffer_offset = buffer_offsets[buffer_view.buffer().index()] as usize;

    let offset = transforms_accessor.offset() + (buffer_view.offset()) + buffer_offset;
    (offset, length)
}
#[derive(Debug, Clone, Copy)]
pub enum AnimationType {
    Rotation,
    Translation,
    Scale,
    // others?
}

pub(super) const NO_TRANSLATION: Vector3<f32> = Vector3::new(0.0, 0.0, 0.0);
pub(super) const NO_ROTATION: Quaternion<f32> = Quaternion::new(1.0, 0.0, 0.0, 0.0); // w = 1
pub(super) const IDENTITY: Matrix4<f32> = Matrix4::<f32>::from_cols(
    Vector4::new(1.0, 0.0, 0.0, 0.0),
    Vector4::new(0.0, 1.0, 0.0, 0.0),
    Vector4::new(0.0, 0.0, 1.0, 0.0),
    Vector4::new(0.0, 0.0, 0.0, 1.0),
);

impl AnimationType {
    pub(super) fn from_property(property: &gltf::animation::Property) -> Self {
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
