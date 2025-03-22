use crate::{app::app_state::InstanceData, object::Object};

// a scene will contain
pub struct Scene {
    pub objects: Vec<Object>,
    pub instances: InstanceData,
}
