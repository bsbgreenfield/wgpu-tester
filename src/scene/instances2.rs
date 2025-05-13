use std::ops::Deref;

use wgpu::{core::global, util::DeviceExt};

use crate::model::{
    self,
    model2::{GModel, GlobalTransform, LocalTransform},
    util::InitializationError,
};

use super::instances::InstanceData;
pub struct InstanceData2 {
    pub model_instances: Vec<usize>,
    pub local_transform_buffer: Option<wgpu::Buffer>,
    pub local_transform_data: Vec<LocalTransform>,
    pub global_transform_buffer: Option<wgpu::Buffer>,
    pub global_transform_data: Vec<[[f32; 4]; 4]>,
    pub instance_local_offsets: Vec<usize>,
}

impl InstanceData2 {
    pub fn new(
        model_instances: Vec<usize>,
        model_mesh_offsets: Vec<usize>,
        local_transform_data: Vec<LocalTransform>,
        global_transform_data: Vec<[[f32; 4]; 4]>,
    ) -> Self {
        Self {
            model_instances,
            instance_local_offsets: model_mesh_offsets,
            local_transform_buffer: None,
            local_transform_data,
            global_transform_buffer: None,
            global_transform_data,
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

    pub fn init(&mut self, device: &wgpu::Device) {
        let local_transform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Local transform buffer"),
            contents: bytemuck::cast_slice(&self.local_transform_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        println!("initializing the global transform buffer ",);
        for g in self.global_transform_data.iter() {
            println!("{g:?}")
        }
        let global_transform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                contents: bytemuck::cast_slice(&self.global_transform_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                label: Some("global instance buffer"),
            });

        self.global_transform_buffer = Some(global_transform_buffer);
        self.local_transform_buffer = Some(local_transform_buffer);
    }

    pub fn add_model_instance(
        &mut self,
        models: &Vec<GModel>,
        model_index: usize,
        global_transforms: Vec<[[f32; 4]; 4]>,
    ) -> Result<(), InitializationError> {
        let count = global_transforms.len().clone();
        self.add_new_instances_local_data(model_index, global_transforms.len(), models)?
            .add_new_instances_global_data(model_index, global_transforms)?;
        println!(
            "successfully added {} model(s) at index {}",
            count, model_index
        );
        Ok(())
    }

    // this function is hilariously costly! probably warrants a restructuring of instancedata
    // even though it really shouldnt be in any hot loop
    fn add_new_instances_local_data(
        &mut self,
        model_index: usize,
        new_instance_count: usize,
        models: &Vec<GModel>,
    ) -> Result<&mut Self, InitializationError> {
        // step 1: create a new vec from all the mesh instances associated with the model
        let model_instance_count = self.model_instances[model_index];
        let model_mesh_count = models[model_index].mesh_instances.iter().sum::<u32>() as usize;

        let mut transform_slice = self.local_transform_data
            [self.instance_local_offsets[model_index]..model_mesh_count * model_instance_count]
            .to_vec();

        // step 2: expand the vec with the appropriate number of new transforms
        let mut offset = 0;
        for mesh_instance_count in models[model_index].mesh_instances.iter() {
            for _ in 0..*mesh_instance_count {
                for i in 0..new_instance_count {
                    // the number of transforms associated with this mesh instance is equal to
                    // the number of instances of this mesh that a single model has times the total
                    // number of models

                    let mut new_transform = transform_slice[offset].clone();
                    new_transform.model_index = (i + model_instance_count) as u32;
                    // the index of the last mesh instance is offset + the number of meshes we started
                    // with + the number of meshes we have already added
                    transform_slice.insert(offset + i + 1, new_transform);
                }
                // new offset is the
                offset += new_instance_count + 1;
            }
        }

        // step 3: splice the local transform data vec with this new expanded vec
        self.local_transform_data.splice(
            self.instance_local_offsets[model_index]..model_mesh_count,
            transform_slice,
        );
        // step 4: increase the offsets for all the models after this one by the number of new
        // instances just added
        let num_new_instances = model_mesh_count * new_instance_count;
        let this_model_offset_val = self.instance_local_offsets[model_index];
        for offset_val in self
            .instance_local_offsets
            .iter_mut()
            .skip(this_model_offset_val)
        {
            *offset_val += num_new_instances;
        }

        Ok(self)
    }

    /// insert the appropriate number of global transform matrices into the global transform data
    /// vector at the appropriate slot
    fn add_new_instances_global_data(
        &mut self,
        model_index: usize,
        global_transforms: Vec<[[f32; 4]; 4]>,
    ) -> Result<(), InitializationError> {
        let new_instance_count = global_transforms.len().clone();
        let mut offset_start: usize = 0;
        for i in 0..model_index {
            offset_start += self.model_instances[i];
        }
        // the number of
        let offset_end = self.model_instances[model_index];
        self.global_transform_data
            .splice(offset_end..offset_end, global_transforms);

        // increment the number of mesh instances by num new instances
        self.model_instances[model_index] += new_instance_count;

        Ok(())
    }

    pub fn merge(mut self, mut other: Self) -> Self {
        let number_of_models = self.model_instances.iter().sum::<usize>();
        for local_transform in other.local_transform_data.iter_mut() {
            local_transform.model_index += number_of_models as u32;
        }
        self.model_instances.extend(other.model_instances);
        let model_instances = self.model_instances;
        self.instance_local_offsets
            .extend(other.instance_local_offsets);
        let instance_local_offsets = self.instance_local_offsets;

        self.local_transform_data.extend(other.local_transform_data);
        let local_transform_data = self.local_transform_data;
        self.global_transform_data
            .extend(other.global_transform_data);

        let global_transform_data = self.global_transform_data;
        let local_transform_buffer = None;
        let global_transform_buffer = None;

        Self {
            model_instances,
            instance_local_offsets,
            local_transform_data,
            global_transform_data,
            local_transform_buffer,
            global_transform_buffer,
        }
    }
}
