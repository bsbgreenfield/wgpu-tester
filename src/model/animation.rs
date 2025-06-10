pub struct Animation {
    pub animation_components: Vec<AnimationComponent>,
}

pub struct AnimationComponent {
    data: Option<InitializedAnimationData>,
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
            data: None,
            mesh_ids,
            times_data,
            transforms_data,
            interpolation,
        }
    }
}

struct InitializedAnimationData {
    lt_index: Option<usize>,
    times: Vec<f32>,
    transforms: Vec<AnimationTransform>,
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
enum AnimationTransform {
    Rotation(cgmath::Vector4<f32>),
    Translation(cgmath::Vector4<f32>),
    Scale(cgmath::Vector4<f32>),
}
