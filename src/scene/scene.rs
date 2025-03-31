use wgpu::{core::device, BufferSlice};

use super::{camera::*, instances::*};
use crate::{
    object::{DrawObject, Object, ObjectTransform},
    util::create_objects,
    vertex::Vertex,
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
    fn get_instances(&self) -> Option<&InstanceData>;
    fn get_objects(&self) -> Option<&Vec<Object>>;
    fn update_objects(&mut self, objects: Option<Vec<Object>>);
    fn update_instances(&mut self, instance_data: Option<InstanceData>);
    fn add_objects(&mut self, objects: Vec<Object>);
    fn add_instances(&mut self, instance_data: InstanceData);
    fn draw_scene<'a, 'b>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'b>,
    ) -> Result<(), SceneDrawError>
    where
        'a: 'b;
    fn setup(&mut self, device: &wgpu::Device, objects: Vec<Object>, instance_data: InstanceData)
    where
        Self: Sized,
    {
        Self::add_objects(self, objects);
        Self::add_instances(self, instance_data);
    }
}

pub struct Scene {
    pub objects: Option<Vec<Object>>,
    pub instance_data: Option<InstanceData>,
    pub camera: Camera,
    pub camera_uniform: CameraUniform,
}

impl Scene {
    /// instantiate a new scene. If used with a scaffold
    pub fn new(device: &wgpu::Device, aspect_ratio: f32, scaffold: Option<SceneScaffold>) -> Self {
        let (camera, camera_uniform) = get_camera_default(aspect_ratio);
        let (objects, instance_data): (Option<Vec<Object>>, Option<InstanceData>) = match scaffold {
            Some(s) => (
                Some(s.objects.clone()),
                Some(Self::instances_from_scaffold(s, device)),
            ),
            None => (None, None),
        };
        Self {
            objects,
            instance_data,
            camera,
            camera_uniform,
        }
    }
    /// for each object in scaffold, create an instance of [ObjectInstances]
    /// and use these to create [InstanceData]
    fn instances_from_scaffold(mut scaffold: SceneScaffold, device: &wgpu::Device) -> InstanceData {
        let mut instances_vec = Vec::<ObjectInstances>::new();
        for obj_num in 0..scaffold.objects.len() {
            // drain the scaffold hashmap, build ObjInstances
            let maybe_obj_transforms = scaffold
                .instances_per_object
                .get(&(obj_num as u32))
                .cloned();
            if let Some(obj_transforms) = maybe_obj_transforms {
                instances_vec.push(ObjectInstances::from_transforms(obj_transforms, 0));
            }
        }
        InstanceData::new(instances_vec, device)
    }
}

impl SceneDrawable for Scene {
    fn add_objects(&mut self, objects: Vec<Object>) {
        self.objects = Some(objects);
    }
    fn add_instances(&mut self, instance_data: InstanceData) {
        self.instance_data = Some(instance_data);
    }
    fn get_objects(&self) -> Option<&Vec<Object>> {
        self.objects.as_ref()
    }
    fn get_instances(&self) -> Option<&InstanceData> {
        self.instance_data.as_ref()
    }
    fn update_objects(&mut self, objects: Option<Vec<Object>>) {
        self.objects = objects;
    }
    fn update_instances(&mut self, instance_data: Option<InstanceData>) {
        self.instance_data = instance_data;
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
                match self.objects.as_ref() {
                    Some(objects) => {
                        for (idx, object) in objects.iter().enumerate() {
                            let num_instances = instance_data.object_instances[idx].num_instances;
                            let offset = instance_data.object_instances[idx].offset_val;
                            render_pass
                                .draw_object_instanced(object, offset..num_instances + offset);
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

use std::{collections::HashMap, error::Error, ops::Deref, slice};

pub struct SceneScaffold {
    objects: Vec<Object>,
    instances_per_object: HashMap<u32, Vec<ObjectTransform>>,
}

impl SceneScaffold {
    pub fn from_vertices(
        vertices: Vec<&[Vertex]>,
        indices: Vec<&[u32]>,
        device: &wgpu::Device,
    ) -> Self {
        let objects = create_objects(vertices, indices, device);
        Self {
            objects,
            instances_per_object: HashMap::new(),
        }
    }
    pub fn new(objects: Vec<Object>) -> Self {
        Self {
            objects,
            instances_per_object: HashMap::new(),
        }
    }

    pub fn add_instances(&mut self, object_index: u32, transforms: Vec<ObjectTransform>) {
        self.instances_per_object.insert(object_index, transforms);
    }
}
