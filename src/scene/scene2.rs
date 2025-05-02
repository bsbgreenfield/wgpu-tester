use crate::model::model2::*;
use crate::model::util::*;
use crate::model::vertex::ModelVertex;
use cgmath::SquareMatrix;
use gltf::Node;
use std::rc::Rc;
use wgpu::util::DeviceExt;

use super::camera::get_camera_default;
use super::camera::Camera;
use super::instances2::InstanceData2;

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
    pub transformation_matrices: Vec<[[f32; 4]; 4]>,
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

fn cg(mut m: [[f32; 4]; 4]) -> cgmath::Matrix4<f32> {
    for a in m.iter_mut() {
        for b in a.iter_mut() {
            *b = (*b * 100000.0).round() / 100000.0;
        }
    }
    cgmath::Matrix4::<f32>::from(m)
}
fn test(nodes: Vec<Node>) {
    let rn = nodes
        .iter()
        .find(|n| n.index() == 5)
        .unwrap()
        .transform()
        .matrix();
    let four = nodes
        .iter()
        .find(|n| n.index() == 4)
        .unwrap()
        .transform()
        .matrix();
    let three = nodes
        .iter()
        .find(|n| n.index() == 3)
        .unwrap()
        .transform()
        .matrix();
    let wheel2 = nodes
        .iter()
        .find(|n| n.index() == 2)
        .unwrap()
        .transform()
        .matrix();

    let m = cg(rn) * cg(four) * cg(three) * cg(wheel2);
    println!("{:?}", m);
}
pub struct GScene {
    pub models: Vec<GModel>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub camera: Camera,
    pub instance_data: InstanceData2,
}

impl GScene {
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

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Vertex Buffer"),
            contents: bytemuck::cast_slice(&scene_buffer_data.vertex_buf),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Index Buffer"),
            contents: bytemuck::cast_slice(&scene_buffer_data.index_buf),
            usage: wgpu::BufferUsages::INDEX,
        });

        let local_transformation_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Local transform buffer"),
                contents: bytemuck::cast_slice(&scene_mesh_data.transformation_matrices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let camera = get_camera_default(aspect_ratio, device);
        //   println!("transformations: ");
        //   for t in scene_mesh_data.transformation_matrices.iter() {
        //       println!("{:?}", t);
        //   }

        let offset_x: [[f32; 4]; 4] =
            cgmath::Matrix4::<f32>::from_translation(cgmath::Vector3::<f32>::new(0.8, 0.5, 0.0))
                .into();
        let identity: [[f32; 4]; 4] = cgmath::Matrix4::<f32>::identity().into();
        let global_transform_data: Vec<[[f32; 4]; 4]> = vec![identity];
        let instance_data =
            InstanceData2::new(local_transformation_buffer, global_transform_data, device);

        Ok(Self {
            models,
            camera,
            vertex_buffer,
            index_buffer,
            instance_data,
        })
    }

    pub fn get_camera_buf(&self) -> &wgpu::Buffer {
        &self.camera.camera_buffer
    }

    pub fn get_global_buf(&self) -> &wgpu::Buffer {
        &self.instance_data.global_transform_buffer
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
}
