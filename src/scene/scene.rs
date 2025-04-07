use super::{camera::*, instances::*};
use crate::{
    model::{
        model::{DrawModel, Model, ObjectTransform},
        vertex::ModelVertex,
    },
    util::create_models,
};

pub enum SceneDrawError {
    DrawError,
}

/// a scene drawable can draw objects and and instances of objects to the screen
/// given a mutable [wgpu::RenderPass]. Any SceneDrawable may be instantiated with
/// a [SceneScaffold], in which case it will be auto populated with the objects
/// and instances provided by the scaffold (cloned)
/// otherwise, you **must** call scene.setup  
pub trait SceneDrawable {
    // required functions to be able to draw the data from the scene on the screen
    fn get_speed(&self) -> f32;
    fn get_instances(&self) -> Option<&InstanceData>;
    fn get_camera_buf(&self) -> &wgpu::Buffer;
    fn get_camera_uniform_data(&self) -> [[f32; 4]; 4];
    fn get_instances_mut(&mut self) -> Option<&mut InstanceData>;
    fn get_models(&self) -> Option<&Vec<Model>>;
    fn update_models(&mut self, objects: Option<Vec<Model>>);
    fn update_instances(
        &mut self,
        object_idx: usize,
        instance_indices: Vec<usize>,
        new_instances: &mut Vec<ObjectTransform>,
    ) -> Option<Vec<[[f32; 4]; 4]>>;
    fn add_models(&mut self, models: Vec<Model>);
    fn add_instances(&mut self, instance_data: InstanceData);
    fn update_camera_pos(&mut self, x: f32, y: f32, z: f32);
    fn update_camera_rot(&mut self, rot: cgmath::Point3<f32>);
    fn draw_scene<'a, 'b>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'b>,
    ) -> Result<(), SceneDrawError>
    where
        'a: 'b;
    fn setup(&mut self, device: &wgpu::Device, models: Vec<Model>, instance_data: InstanceData)
    where
        Self: Sized,
    {
        Self::add_models(self, models);
        Self::add_instances(self, instance_data);
    }
}

pub struct Scene {
    pub models: Option<Vec<Model>>,
    pub instance_data: Option<InstanceData>,
    pub camera: Camera,
}

impl Scene {
    /// instantiate a new scene. If used with a scaffold
    pub fn new(device: &wgpu::Device, aspect_ratio: f32, scaffold: Option<SceneScaffold>) -> Self {
        let camera = get_camera_default(aspect_ratio, device);
        let (models, instance_data): (Option<Vec<Model>>, Option<InstanceData>) = match scaffold {
            Some(s) => (
                Some(s.models.clone()),
                Some(Self::instances_from_scaffold(s, device)),
            ),
            None => (None, None),
        };
        Self {
            models,
            instance_data,
            camera,
        }
    }
    /// for each object in scaffold, create an instance of [ObjectInstances]
    /// and use these to create [InstanceData]
    fn instances_from_scaffold(mut scaffold: SceneScaffold, device: &wgpu::Device) -> InstanceData {
        let mut instances_vec = Vec::<ObjectInstances>::new();
        for model_num in 0..scaffold.models.len() {
            // drain the scaffold hashmap, build ObjInstances
            let maybe_obj_transforms = scaffold
                .instances_per_object
                .get(&(model_num as u32))
                .cloned();
            if let Some(obj_transforms) = maybe_obj_transforms {
                instances_vec.push(ObjectInstances::from_transforms(obj_transforms, 0));
            }
        }
        InstanceData::new(instances_vec, device)
    }
}

impl SceneDrawable for Scene {
    fn get_speed(&self) -> f32 {
        return self.camera.speed;
    }
    fn get_camera_uniform_data(&self) -> [[f32; 4]; 4] {
        self.camera.camera_uniform.view_proj
    }
    fn get_camera_buf(&self) -> &wgpu::Buffer {
        &self.camera.camera_buffer
    }
    fn update_camera_rot(&mut self, rot: cgmath::Point3<f32>) {
        self.camera.update_rot(rot);
    }
    fn update_camera_pos(&mut self, x: f32, y: f32, z: f32) {
        self.camera.update_position(cgmath::point3(x, y, z));
    }
    fn add_models(&mut self, models: Vec<Model>) {
        self.models = Some(models);
    }
    fn add_instances(&mut self, instance_data: InstanceData) {
        self.instance_data = Some(instance_data);
    }
    fn get_models(&self) -> Option<&Vec<Model>> {
        self.models.as_ref()
    }
    fn get_instances(&self) -> Option<&InstanceData> {
        self.instance_data.as_ref()
    }
    fn get_instances_mut(&mut self) -> Option<&mut InstanceData> {
        self.instance_data.as_mut()
    }
    fn update_models(&mut self, models: Option<Vec<Model>>) {
        self.models = models;
    }
    fn update_instances(
        &mut self,
        object_idx: usize,
        instance_indices: Vec<usize>,
        new_instances: &mut Vec<ObjectTransform>,
    ) -> Option<Vec<[[f32; 4]; 4]>> {
        if let Some(instance_data) = self.instance_data.as_mut() {
            instance_data.update_object_instances(object_idx, instance_indices, new_instances);
            Some(instance_data.get_raw_data())
        } else {
            None
        }
    }

    fn draw_scene<'a, 'b>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'b>,
    ) -> Result<(), SceneDrawError>
    where
        'a: 'b,
    {
        match self.instance_data.as_ref() {
            Some(instance_data) => {
                render_pass.set_vertex_buffer(1, instance_data.instance_transform_buffer.slice(..));
                match self.models.as_ref() {
                    Some(models) => {
                        for (idx, model) in models.iter().enumerate() {
                            let num_instances = instance_data.object_instances[idx].num_instances;
                            let offset = instance_data.object_instances[idx].offset_val;
                            render_pass.draw_model_instanced(model, offset..num_instances + offset);
                        }
                    }
                    None => (),
                }
            }
            None => (),
        }
        Ok(())
    }
}

use std::collections::HashMap;

pub struct SceneScaffold {
    models: Vec<Model>,
    instances_per_object: HashMap<u32, Vec<ObjectTransform>>,
}

impl SceneScaffold {
    pub fn from_vertices(
        vertices: Vec<&[ModelVertex]>,
        indices: Vec<&[u32]>,
        device: &wgpu::Device,
    ) -> Self {
        let models = create_models(vertices, indices, device);
        Self {
            models,
            instances_per_object: HashMap::new(),
        }
    }
    pub fn new(models: Vec<Model>) -> Self {
        Self {
            models,
            instances_per_object: HashMap::new(),
        }
    }

    pub fn add_instances(&mut self, object_index: u32, transforms: Vec<ObjectTransform>) {
        self.instances_per_object.insert(object_index, transforms);
    }
}
