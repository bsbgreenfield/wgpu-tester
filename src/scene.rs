use crate::object::{Object, ObjectTransform, ToRawMatrix};
use cgmath::{Matrix4, SquareMatrix, Transform};
use wgpu::util::DeviceExt;

pub struct Camera {
    fov: f32,
    aspect_ratio: f32,
    zfar: f32,
    znear: f32,
    eye_pos: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
}
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

impl Camera {
    pub fn new(fov: f32, aspect_ratio: f32, znear: f32, zfar: f32) -> Self {
        Self {
            fov,
            aspect_ratio,
            zfar,
            znear,
            eye_pos: cgmath::Point3 {
                x: 0.0,
                y: 1.0,
                z: 10.0,
            },
            up: cgmath::Vector3::unit_y(),
            target: cgmath::Point3::new(0.0, 0.0, 0.0),
        }
    }
    pub fn perspective_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye_pos, self.target, self.up);
        let proj = cgmath::perspective(
            cgmath::Rad(self.fov),
            self.aspect_ratio,
            self.znear,
            self.zfar,
        );
        proj * view
    }

    pub fn get_buffer(&self, camera_uniform: CameraUniform, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}
impl CameraUniform {
    pub fn new(camera: &Camera) -> Self {
        Self {
            view_proj: (OPENGL_TO_WGPU_MATRIX * camera.perspective_matrix()).into(),
        }
    }
}

#[derive(Debug)]
pub struct ObjectInstances {
    pub transforms: Vec<ObjectTransform>,
    pub num_instances: u32,
    pub offset_val: u32,
}

pub struct InstanceData {
    pub instance_transform_buffer: wgpu::Buffer,
    pub object_instances: Vec<ObjectInstances>,
}
impl InstanceData {
    pub fn new(object_instances_list: Vec<ObjectInstances>, device: &wgpu::Device) -> Self {
        let tot_num_instances: usize = object_instances_list
            .iter()
            .map(|e| e.transforms.len())
            .sum();
        let mut data = Vec::<[[f32; 4]; 4]>::with_capacity(tot_num_instances);

        for object_instances in object_instances_list.iter() {
            let raw_matrices = object_instances
                .transforms
                .iter()
                .map(|t| t.as_raw_matrix());
            for raw_data in raw_matrices {
                data.push(raw_data);
            }
        }

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance buffer"),
            contents: bytemuck::cast_slice(&data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            instance_transform_buffer: buffer,
            object_instances: object_instances_list,
        }
    }

    fn create_instance_buffer_from_raw(
        instance_data: &[[[f32; 4]; 4]],
        device: &wgpu::Device,
    ) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        })
    }
}
impl ObjectInstances {
    pub fn from_transforms(transforms: Vec<ObjectTransform>, offset_val: u32) -> Self {
        let num_instances = transforms.len() as u32;
        Self {
            transforms,
            num_instances,
            offset_val,
        }
    }
    pub fn as_raw_data(&self) -> Vec<[[f32; 4]; 4]> {
        self.transforms
            .iter()
            .map(|t| t.as_raw_matrix())
            .collect::<Vec<[[f32; 4]; 4]>>()
    }
    pub fn apply_transforms(&mut self, t_matrices: &[cgmath::Matrix4<f32>]) {
        assert!(t_matrices.len() == self.transforms.len());
        for (i, transform) in self.transforms.iter_mut().enumerate() {
            transform.transform_matrix = transform.transform_matrix.concat(&t_matrices[i]);
        }
    }
}
pub fn get_camera_bind_group(
    camera_buffer: &wgpu::Buffer,
    device: &wgpu::Device,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    let camera_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Camera bind group layout"),
        });

    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
        label: Some("camera bind group"),
    });

    (camera_bind_group_layout, camera_bind_group)
}
fn get_camera_default(aspect_ratio: f32) -> (Camera, CameraUniform) {
    let camera = Camera::new(std::f32::consts::FRAC_PI_4, aspect_ratio, 0.1, 100.0);
    let camera_uniform: CameraUniform = CameraUniform::new(&camera);
    (camera, camera_uniform)
}

pub trait Scene {
    fn add_objects(&mut self, objects: Vec<Object>);
    fn add_instances(&mut self, instance_data: InstanceData);
    fn setup(&mut self, device: &wgpu::Device)
    where
        Self: Sized,
    {
        let objects = Self::create_objects(device);
        let instance_data = Self::create_instances(device, &objects);
        Self::add_objects(self, objects);
        Self::add_instances(self, instance_data);
    }
    fn get_instances(&self) -> &InstanceData;
    fn get_objects(&self) -> &Vec<Object>;
    fn new(device: &wgpu::Device, aspect_ratio: f32) -> Self
    where
        Self: Sized;
    fn create_objects(device: &wgpu::Device) -> Vec<Object>
    where
        Self: Sized;
    fn create_instances(device: &wgpu::Device, objects: &Vec<Object>) -> InstanceData
    where
        Self: Sized;
}

pub struct MyScene {
    pub objects: Vec<Object>,
    pub instance_data: InstanceData,
    pub camera: Camera,
    pub camera_uniform: CameraUniform,
}

impl Scene for MyScene {
    // for each object, we need to define an one object instance per
    // each instance that we want in the scene
    fn create_instances(device: &wgpu::Device, objects: &Vec<Object>) -> InstanceData {
        // local -> global transfomation matrix
        let transform: ObjectTransform = ObjectTransform {
            transform_matrix: cgmath::Matrix4::from_angle_z(cgmath::Deg(15.0))
                * cgmath::Matrix4::from_translation(cgmath::vec3(0.0, 0.0, -10.0)),
        };
        let transform_2: ObjectTransform = ObjectTransform {
            transform_matrix: cgmath::Matrix4::from_angle_z(cgmath::Deg(15.0))
                * cgmath::Matrix4::from_translation(cgmath::vec3(0.0, 0.0, 0.0)),
        };
        let instance_1 = ObjectInstances::from_transforms(vec![transform, transform_2], 0);
        InstanceData::new(vec![instance_1], device)
    }

    fn create_objects(device: &wgpu::Device) -> Vec<Object> {
        use crate::constants::*;
        use crate::util::create_objects;
        create_objects(vec![VERTICES], vec![&INDICES], device)
    }
    fn add_objects(&mut self, objects: Vec<Object>) {
        self.objects = objects;
    }
    fn add_instances(&mut self, instance_data: InstanceData) {
        self.instance_data = instance_data;
    }

    fn new(device: &wgpu::Device, aspect_ratio: f32) -> Self {
        let (camera, camera_uniform) = get_camera_default(aspect_ratio);
        Self {
            objects: Vec::new(),
            instance_data: InstanceData::new(Vec::new(), device),
            camera,
            camera_uniform,
        }
    }
    fn get_objects(&self) -> &Vec<Object> {
        &self.objects
    }
    fn get_instances(&self) -> &InstanceData {
        &self.instance_data
    }
}
