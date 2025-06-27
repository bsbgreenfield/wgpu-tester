use std::time::Duration;

use crate::model::animation::animation_controller::get_scene_animation_data;
use crate::model::animation::animation_controller::SceneAnimationController;
use crate::model::animation::animation_controller::SimpleAnimation;
use crate::model::loader::loader::GltfData;
use crate::model::model::*;
use crate::model::util::*;
use crate::model::vertex::ModelVertex;
use crate::scene::scene_scaffolds::SceneScaffold;
use wgpu::util::DeviceExt;

use super::camera::Camera;
use super::camera::{get_camera_bind_group, get_camera_default};
use super::instances::InstanceData;

pub struct GScene {
    pub models: Vec<GModel>,
    vertex_data: VertexData,
    index_data: IndexData,
    pub(super) instance_data: InstanceData,
    camera: Option<Camera>,
    animation_controller: SceneAnimationController,
}

impl GScene {
    pub fn initialize_animation(
        &mut self,
        model_id: usize,
        instance_idx: usize,
        animation_index: usize,
    ) {
        let ( offset, len ) = self
            .instance_data
            .get_instance_local_offset(instance_idx, model_id);
        self.animation_controller
            .initialize_animation(animation_index, offset, len);
    }
    pub fn get_animation_frame(&mut self, timestamp: Duration) -> bool {
        let maybe_animation_frame = self.animation_controller.do_animations(timestamp);
        match maybe_animation_frame {
            Some(animation_frame) => {
                self.instance_data
                    .apply_animation_frame_unchecked(animation_frame);

                return true;
            }
            None => return false,
        }
    }
    pub fn init(&mut self, device: &wgpu::Device, aspect_ratio: f32) {
        self.vertex_data.init(device);
        self.index_data.init(device);
        self.instance_data.init(device);
        let camera = get_camera_default(aspect_ratio, device);
        self.camera = Some(camera); // TODO: allow for adding a custom camera
    }
    pub fn get_camera_buf(&self) -> &wgpu::Buffer {
        &self.camera.as_ref().unwrap().camera_buffer
    }
    pub fn get_instance_local_offset(&self, instance_idx: usize, model_id: usize) -> (usize, usize) {
        self.instance_data.get_instance_local_offset(instance_idx, model_id)
    }

    pub fn get_global_buf(&self) -> Result<&wgpu::Buffer, InitializationError> {
        if self.instance_data.global_transform_buffer.is_some() {
            return Ok(self.instance_data.global_transform_buffer.as_ref().unwrap());
        }
        Err(InitializationError::InstanceDataInitializationError(
            Box::new(String::from(    
            "Global buffer has not been initialized! Please call InstanceData.init() when your data is ready"
            ))
        ))
    }
    pub fn update_global_transform(
        &mut self,
        model_number: usize,
        model_instance_index: usize,
        new_transform: [[f32; 4]; 4],
    ) {
        let mut instance_count = 0;
        // skip all preceding models
        for idx in 0..model_number {
            instance_count += self.instance_data.model_instances[idx];
        }
        // skip all preceding instances of this model
        instance_count += model_instance_index;
        println!("updating model at index {}", instance_count);
        self.instance_data
            .update_global_transform_x(instance_count, new_transform);
    }
    pub fn get_camera_bind_group(
        &self,
        device: &wgpu::Device,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        if let Some(camera) = &self.camera {
            get_camera_bind_group(&camera.camera_buffer, device)
        } else {
            panic!("no camera")
        }
    }
    pub fn get_camera_uniform_data(&self) -> [[f32; 4]; 4] {
        self.camera.as_ref().unwrap().camera_uniform.view_proj
    }
    pub fn update_camera_pos(&mut self, x: f32, y: f32, z: f32) {
        self.camera
            .as_mut()
            .unwrap()
            .update_position(cgmath::point3(x, y, z));
    }
    pub fn get_speed(&self) -> f32 {
        return self.camera.as_ref().unwrap().speed;
    }
    pub fn get_vertex_buffer(&self) -> &Option<wgpu::Buffer> {
        return &self.vertex_data.vertex_buffer;
    }
    pub fn get_local_transform_data(&self) -> &Vec<LocalTransform> {
        &self.instance_data.local_transform_data
    }
    pub fn get_local_transform_buffer(&self) -> &Option<wgpu::Buffer> {
        &self.instance_data.local_transform_buffer
    }
    pub fn get_global_transform_data(&self) -> &Vec<[[f32; 4]; 4]> {
        &self.instance_data.global_transform_data
    }
    pub fn get_global_transform_buffer(&self) -> &Option<wgpu::Buffer> {
        &self.instance_data.global_transform_buffer
    }
    pub fn get_index_buffer(&self) -> &Option<wgpu::Buffer> {
        return &self.index_data.index_buffer;
    }
    pub fn get_model_instances(&self) -> &Vec<usize> {
        &self.instance_data.model_instances
    }
    pub fn update_global_transform_x(&mut self, instance_idx: usize, new_transform: [[f32; 4]; 4]) {
        self.instance_data
            .update_global_transform_x(instance_idx, new_transform);
    }
}

/// an uninitialized scene
pub struct GSceneData {
    pub models: Vec<GModel>,
    vertex_vec: Vec<ModelVertex>,
    index_vec: Vec<u16>,
    local_transforms: Vec<LocalTransform>,
    simple_animations: Vec<SimpleAnimation>,
}

impl GSceneData {
    pub fn build_scene_init(self, device: &wgpu::Device, aspect_ratio: f32) -> GScene {
        let mut scene = self.build_scene_uninit();
        scene.init(device, aspect_ratio);
        scene
    }

    pub fn build_scene_from_scaffold(
        self,
        device: &wgpu::Device,
        aspect_ratio: f32,
        scaffold: &SceneScaffold,
    ) -> Result<GScene, InitializationError> {
        let instance_data =
            InstanceData::from_scaffold(scaffold, self.local_transforms, &self.models)?;
        let vertex_data = VertexData::from_data(self.vertex_vec);
        let index_data = IndexData::from_data(self.index_vec);
        let animation_controller = SceneAnimationController::new(self.simple_animations);
        let mut scene = GScene {
            animation_controller,
            models: self.models,
            vertex_data,
            instance_data,
            index_data,
            camera: None,
        };
        scene.init(device, aspect_ratio);
        return Ok(scene);
    }
    pub fn build_scene_uninit(self) -> GScene {
        let instance_data = InstanceData::default_from_scene(&self.models, self.local_transforms);
        let vertex_data = VertexData::from_data(self.vertex_vec);
        let index_data = IndexData::from_data(self.index_vec);
        let animation_controller = SceneAnimationController::new(self.simple_animations);

        GScene {
            models: self.models,
            vertex_data,
            instance_data,
            index_data,
            camera: None,
            animation_controller,
        }
    }

    pub fn new(mut gltf_data: GltfData) -> Self {
        // build out vertex and index data from the models, meshes, and primitives by referencing
        // the main blob
        let vertex_vec =
            Self::get_scene_vertex_buffer_data(&mut gltf_data.models, &gltf_data.binary_data);
        let index_vec =
            Self::get_scene_index_buffer_data(&mut gltf_data.models, &gltf_data.binary_data);
        let simple_animations =
            get_scene_animation_data(gltf_data.simple_animations, &gltf_data.binary_data);

        Self {
            models: gltf_data.models,
            vertex_vec,
            index_vec,
            local_transforms: gltf_data.local_transforms,
            simple_animations,
        }
    }

    fn get_scene_vertex_buffer_data(
        models: &mut Vec<GModel>,
        main_buffer_data: &Vec<u8>,
    ) -> Vec<ModelVertex> {
        let mut vertex_buffer_data = Vec::<ModelVertex>::new();
        // loop through the models -> meshes -> primitives to build out the vertex buffer
        let mut buffer_offset_val = 0;
        for model in models.iter_mut() {
            vertex_buffer_data
                .extend(model.get_model_vertex_data(main_buffer_data, &mut buffer_offset_val));
        }
        vertex_buffer_data
    }
    fn get_scene_index_buffer_data(
        models: &mut Vec<GModel>,
        main_buffer_data: &Vec<u8>,
    ) -> Vec<u16> {
        let mut range_vec: Vec<std::ops::Range<usize>> = Vec::new();
        for model in models.iter() {
            model.build_range_vec(&mut range_vec); // MUTATE RANGE VEC
        }
        let index_vec = GModel::get_model_index_data(main_buffer_data, &range_vec);
        // add in the relative buffer offset and len based on the new composed data vec
        for model in models.iter_mut() {
            model.set_model_primitive_offsets(&range_vec);
        }
        index_vec
    }
}

trait SceneData<T> {
    fn from_data(data: T) -> Self;
    fn init(&mut self, device: &wgpu::Device);
}

pub struct VertexData {
    vertices: Vec<ModelVertex>,
    vertex_buffer: Option<wgpu::Buffer>,
}
pub struct IndexData {
    indices: Vec<u16>,
    index_buffer: Option<wgpu::Buffer>,
}

impl SceneData<Vec<ModelVertex>> for VertexData {
    fn init(&mut self, device: &wgpu::Device) {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Vertex Buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        self.vertex_buffer = Some(vertex_buffer);
    }

    fn from_data(data: Vec<ModelVertex>) -> Self {
        VertexData {
            vertices: data,
            vertex_buffer: None,
        }
    }
}

impl SceneData<Vec<u16>> for IndexData {
    fn init(&mut self, device: &wgpu::Device) {
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Index Buffer"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.index_buffer = Some(index_buffer);
    }
    fn from_data(data: Vec<u16>) -> Self {
        Self {
            indices: data,
            index_buffer: None,
        }
    }
}
