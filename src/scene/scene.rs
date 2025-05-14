use super::util::*;
use crate::model::model::*;
use crate::model::util::*;
use crate::model::vertex::ModelVertex;
use cgmath::SquareMatrix;
use gltf::Node;
use std::rc::Rc;
use wgpu::util::DeviceExt;

use super::camera::Camera;
use super::camera::{get_camera_bind_group, get_camera_default};
use super::instances::InstanceData;

pub struct SceneBufferData {
    pub main_buffer_data: Rc<Vec<u8>>,
    pub vertex_buf: Vec<ModelVertex>,
    pub index_buf: Vec<u16>,
}
impl SceneBufferData {
    fn new(main_buffer_data: Rc<Vec<u8>>) -> Self {
        Self {
            main_buffer_data,
            vertex_buf: Vec::new(),
            index_buf: Vec::new(),
        }
    }
}

pub struct SceneMeshData {
    pub mesh_ids: Vec<u32>,
    pub mesh_instances: Vec<u32>,
    pub transformation_matrices: Vec<LocalTransform>,
}
impl SceneMeshData {
    fn new() -> Self {
        Self {
            mesh_ids: Vec::new(),
            mesh_instances: Vec::new(),
            transformation_matrices: Vec::new(),
        }
    }
}

pub struct GScene {
    pub models: Vec<GModel>,
    pub vertex_data: VertexData,
    pub index_data: IndexData,
    pub camera: Camera,
    pub instance_data: InstanceData,
}

impl GScene {
    pub fn init(&mut self, device: &wgpu::Device) {
        self.init_data(device);
        self.instance_data.init(device);
    }

    pub fn new<'a>(
        nodes: gltf::iter::Nodes<'a>,
        root_nodes_ids: Vec<usize>,
        buffer_data: Rc<Vec<u8>>,
        device: &wgpu::Device,
        aspect_ratio: f32,
    ) -> Result<Self, GltfErrors> {
        let nodes: Vec<_> = nodes.collect();
        let mut models = Vec::with_capacity(root_nodes_ids.len());
        let mut scene_buffer_data: SceneBufferData = SceneBufferData::new(buffer_data.clone());
        let mut scene_mesh_data = SceneMeshData::new();

        for id in root_nodes_ids.iter() {
            // get a ref to the root node

            let root_node: &Node<'a> = &nodes[*id];

            // find mesh id's and instances associated with this root node
            scene_mesh_data = find_meshes(
                root_node,
                scene_mesh_data,
                cgmath::Matrix4::identity().into(),
            );
            assert_eq!(
                scene_mesh_data.mesh_ids.len(),
                scene_mesh_data.mesh_instances.len()
            );

            // given the meshes that are included in this model, generate GMeshes
            // this also appends the vertex and index buffer with the data for these meshes
            let meshes: Vec<GMesh> =
                get_meshes(&scene_mesh_data.mesh_ids, &nodes, &mut scene_buffer_data)?;

            models.push(GModel {
                byte_data: buffer_data.clone(),
                meshes,
                mesh_instances: scene_mesh_data.mesh_instances.clone(),
            });
        }

        let (vertex_data, index_data) = GScene::new_data(scene_buffer_data);

        let camera = get_camera_default(aspect_ratio, device);
        //   println!("transformations: ");
        //   for t in scene_mesh_data.transformation_matrices.iter() {
        //       println!("{:?}", t);
        //   }

        let identity: [[f32; 4]; 4] = cgmath::Matrix4::<f32>::identity().into();
        let global_transform_data: Vec<[[f32; 4]; 4]> = vec![identity];

        // we need the number of meshes per model so that we can keep track of the proper
        // local transform offsets as we add new instances of models. The responsibility for
        // keeping track of this is delegated now to InstanceData

        // this seems rather dumb.
        let model_instances: Vec<usize> = models.iter().map(|_| 1).collect();
        let model_mesh_offsets = calculate_model_mesh_offsets(&models, &model_instances);

        let instance_data = InstanceData::new(
            model_instances,
            model_mesh_offsets,
            scene_mesh_data.transformation_matrices, // local transforms
            global_transform_data,                   // global transforms
        );

        Ok(Self {
            models,
            camera,
            vertex_data,
            index_data,
            instance_data,
        })
    }

    pub fn merge<'a>(
        mut scene1: GScene,
        mut scene2: GScene,
    ) -> Result<GScene, InitializationError<'a>> {
        let (vertex_count, index_count) = scene1.get_total_vertex_index_len();
        //TODO: check that these arent the same gltf file
        let vertex_data = scene1.vertex_data.extend(scene2.vertex_data);
        let index_data = scene1.index_data.extend(scene2.index_data);
        // grab the camera from the first scene
        // TODO: make camera part of init?
        let camera = scene1.camera;

        // when merging the second scene into the first, we need to adjust the offsets for the
        // vertex data and index data that is being stored in their primitives
        for model in scene2.models.iter_mut() {
            for mesh in model.meshes.iter_mut() {
                mesh.update_primitive_offsets(vertex_count, index_count);
            }
        }
        scene1.models.extend(scene2.models);
        let models = scene1.models;
        let instance_data: InstanceData = scene1.instance_data.merge(scene2.instance_data, &models);

        Ok(GScene {
            models,
            vertex_data,
            index_data,
            camera,
            instance_data,
        })
    }

    pub fn update_global_transform(
        &mut self,
        model_number: usize,
        model_instance_index: usize,
        new_transform: GlobalTransform,
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

    /// add an instance of an existing model to the scene. The number of instances corresponds to
    /// the size of the global transform vec
    /// [model_idx] : index of the model in the scenes models vec
    /// [global_transforms] global transform to apply to this instance
    pub fn add_model_instances(&mut self, model_idx: usize, global_transforms: Vec<[[f32; 4]; 4]>) {
        self.instance_data
            .add_model_instance(&self.models, model_idx, global_transforms);
    }

    pub fn get_camera_buf(&self) -> &wgpu::Buffer {
        &self.camera.camera_buffer
    }

    pub fn get_global_buf(&self) -> Result<&wgpu::Buffer, InitializationError> {
        if self.instance_data.global_transform_buffer.is_some() {
            return Ok(self.instance_data.global_transform_buffer.as_ref().unwrap());
        }
        Err(InitializationError::InstanceDataInitializationError(
            "Global buffer has not been initialized! Please call InstanceData.init() when your data is ready",
        ))
    }
    pub fn get_camera_bind_group(
        &self,
        device: &wgpu::Device,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        get_camera_bind_group(&self.camera.camera_buffer, device)
    }
    pub fn get_camera_uniform_data(&self) -> [[f32; 4]; 4] {
        self.camera.camera_uniform.view_proj
    }
    pub fn update_camera_pos(&mut self, x: f32, y: f32, z: f32) {
        self.camera.update_position(cgmath::point3(x, y, z));
    }
    pub fn get_speed(&self) -> f32 {
        return self.camera.speed;
    }
    pub fn get_vertex_buffer(&self) -> &Option<wgpu::Buffer> {
        return &self.vertex_data.vertex_buffer;
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
    pub fn update_global_transform_x(
        &mut self,
        instance_idx: usize,
        new_transform: GlobalTransform,
    ) {
        self.instance_data
            .update_global_transform_x(instance_idx, new_transform);
    }
    fn get_total_vertex_index_len(&self) -> (u32, u32) {
        let mut vertex_count = 0;
        let mut index_count = 0;
        for model in self.models.iter() {
            for mesh in model.meshes.iter() {
                vertex_count += mesh.get_total_vertex_len();
                index_count += mesh.get_total_index_len();
            }
        }
        (vertex_count, index_count)
    }
    fn new_data(scene_buffer_data: SceneBufferData) -> (VertexData, IndexData) {
        let vertex_data = VertexData::from_data(scene_buffer_data.vertex_buf);
        let index_data = IndexData::from_data(scene_buffer_data.index_buf);
        (vertex_data, index_data)
    }

    fn init_data(&mut self, device: &wgpu::Device) {
        self.vertex_data.init(device);
        self.index_data.init(device);
    }
}

trait SceneData<T> {
    fn from_data(data: T) -> Self;
    fn init(&mut self, device: &wgpu::Device);
    fn extend(self, other: Self) -> Self;
}

struct VertexData {
    vertices: Vec<ModelVertex>,
    vertex_buffer: Option<wgpu::Buffer>,
}
struct IndexData {
    indices: Vec<u16>,
    index_buffer: Option<wgpu::Buffer>,
}

impl SceneData<Vec<ModelVertex>> for VertexData {
    fn extend(mut self, other: Self) -> Self {
        self.vertices.extend(other.vertices);
        Self {
            vertices: self.vertices,
            vertex_buffer: None,
        }
    }
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
    fn extend(mut self, other: Self) -> Self {
        self.indices.extend(other.indices);
        Self {
            indices: self.indices,
            index_buffer: None,
        }
    }
    fn init(&mut self, device: &wgpu::Device) {
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Index Buffer"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.index_buffer = Some(index_buffer);
    }
    fn from_data(data: Vec<u16>) -> Self {
        IndexData {
            indices: data,
            index_buffer: None,
        }
    }
}
