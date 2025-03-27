use crate::object::{Object, ObjectTransform, ToRawMatrix};
use cgmath::Transform;
use wgpu::util::DeviceExt;

pub struct Camera {
    fov: f32,
    aspect_ratio: f32,
    zfar: f32,
    znear: f32,
    eye_pos: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
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
                y: 0.0,
                z: 2.0,
            },
            target: cgmath::Point3::new(0.0, 0.0, 0.0),
        }
    }
    pub fn perspective_matrix(&self) -> cgmath::Matrix4<f32> {
        let p = (self.fov / 2.0).tan();
        let x_factor = self.aspect_ratio / p;
        let z_aspect = self.zfar / (self.zfar - self.znear);
        let z_norm = -1.0 * self.znear * (self.zfar / (self.zfar - self.znear));
        cgmath::Matrix4::<f32>::new(
            x_factor, 0.0, 0.0, 0.0, 0.0, p, 0.0, 0.0, 0.0, 0.0, z_aspect, z_norm, 0.0, 0.0, 1.0,
            0.0,
        )
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
            view_proj: camera.perspective_matrix().into(),
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

pub struct Scene {
    pub objects: Vec<Object>,
    pub instance_data: InstanceData,
    pub camera: Camera,
    pub camera_uniform: CameraUniform,
}

impl Scene {
    pub fn get_camera_bind_group(
        &self,
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
    pub fn setup_with_default_camera(
        objects: Vec<Object>,
        instance_data: InstanceData,
        ar: f32,
    ) -> Self {
        let camera: Camera = Camera::new(std::f32::consts::FRAC_PI_2, ar, 0.1, 100.0);
        let camera_uniform = CameraUniform::new(&camera);
        Scene {
            objects,
            camera,
            instance_data,
            camera_uniform,
        }
    }
}
