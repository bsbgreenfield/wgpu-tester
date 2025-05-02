use wgpu::util::DeviceExt;

use crate::model::model2::GlobalTransform;
pub struct InstanceData2 {
    pub local_transform_buffer: wgpu::Buffer,
    pub global_transform_buffer: wgpu::Buffer,
    pub model_index_buffer: wgpu::Buffer,
    pub global_transform_data: Vec<[[f32; 4]; 4]>,
}

impl InstanceData2 {
    pub fn new(
        local_transform_buffer: wgpu::Buffer,
        global_transform_data: Vec<[[f32; 4]; 4]>,
        device: &wgpu::Device,
    ) -> Self {
        let global_transform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                contents: bytemuck::cast_slice(&global_transform_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                label: Some("global instance buffer"),
            });

        let model_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: &[0],
            label: Some("Model index buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            local_transform_buffer,
            global_transform_buffer,
            global_transform_data,
            model_index_buffer,
        }
    }
    pub fn update_global_transform_x(
        &mut self,
        instance_idx: usize,
        new_transform: GlobalTransform,
    ) {
        self.global_transform_data[instance_idx] =
            new_transform * self.global_transform_data[instance_idx];
    }
}
