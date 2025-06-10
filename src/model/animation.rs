use crate::model::model::UninitModelElement;

pub struct AnimationData {
    animations: Vec<Animation>,
}

pub struct UninitAnimationData {
    pub mesh_ids: Vec<usize>,
    pub times_data: (usize, usize),
    pub transforms_data: (usize, usize),
    pub interpolation: Interpolation,
}

impl UninitModelElement<AnimationData> for UninitAnimationData {
    fn init(self) -> AnimationData {
        todo!()
    }
}

pub struct Animation {
    lt_index: Option<usize>,
    times: Option<Vec<f32>>,
    transforms: Vec<AnimationTransform>,
    times_data: (usize, usize),
    transforms_data: (usize, usize),
    interpolation: Interpolation,
}

struct InitializedAnimationData {
    lt_index: usize,
    times: Option<Vec<f32>>,
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

#[derive(Debug)]
pub struct GltfAnimationData {
    pub animation_components: Vec<GltfAnimationComponentData>,
}

#[derive(Debug)]
pub struct GltfAnimationComponentData {
    pub mesh_ids: Vec<usize>,
    pub times_data: (usize, usize),
    pub transforms_data: (usize, usize),
    pub interpolation: Interpolation,
}
