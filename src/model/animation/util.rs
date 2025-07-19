use cgmath::{Matrix4, Quaternion, Vector3, Vector4};
use gltf::accessor::DataType;

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
        gltf::animation::Property::Scale => transforms_accessor.count() * 12,
        _ => todo!("havent implemented morph yet"),
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
pub enum InterpolationType {
    Linear,
}
impl From<gltf::animation::Interpolation> for InterpolationType {
    fn from(value: gltf::animation::Interpolation) -> Self {
        match value {
            gltf::animation::Interpolation::Linear => InterpolationType::Linear,
            _ => todo!(),
        }
    }
}
