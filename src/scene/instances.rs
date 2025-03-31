use crate::object::{ObjectTransform, ToRawMatrix};
use cgmath::Transform;
use wgpu::util::DeviceExt;
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
