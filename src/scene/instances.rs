use crate::{
    model::model::GModel,
    scene::{scene_scaffolds::SceneScaffold, util::calculate_model_mesh_offsets},
};
use cgmath::SquareMatrix;
use std::ops::Range;
use wgpu::util::DeviceExt;

use crate::model::{
    model::{GlobalTransform, LocalTransform},
    util::InitializationError,
};
#[allow(dead_code)]

pub(super) struct InstanceData {
    pub model_instances: Vec<usize>,
    pub local_transform_buffer: Option<wgpu::Buffer>,
    pub local_transform_data: Vec<LocalTransform>,
    pub global_transform_buffer: Option<wgpu::Buffer>,
    pub global_transform_data: Vec<[[f32; 4]; 4]>,
    pub instance_local_offsets: Vec<usize>,
}

#[allow(dead_code)]
impl InstanceData {
    /// create Instance data with one instance of each model, each positioned at the origin
    pub fn default_from_scene(
        model_count: usize,
        local_transform_data: Vec<LocalTransform>,
    ) -> Self {
        let model_instances: Vec<usize> = (0..model_count).into_iter().map(|_| 1).collect();
        let global_transform_data: Vec<[[f32; 4]; 4]> = (0..model_count)
            .into_iter()
            .map(|_| cgmath::Matrix4::<f32>::identity().into())
            .collect();
        let instance_local_offsets = vec![0];

        Self {
            model_instances,
            local_transform_buffer: None,
            local_transform_data,
            global_transform_buffer: None,
            global_transform_data,
            instance_local_offsets,
        }
    }

    pub fn from_scaffold(
        scaffold: &SceneScaffold,
        local_transform_data: Vec<LocalTransform>,
        models: &Vec<GModel>,
    ) -> Self {
        // TODO: allow for zero instances of a model
        let model_instances: Vec<usize> = (0..models.len()).map(|_| 1).collect();
        let global_transform_data: Vec<[[f32; 4]; 4]> = (0..models.len())
            .into_iter()
            .map(|_| cgmath::Matrix4::<f32>::identity().into())
            .collect();
        let instance_local_offsets: Vec<usize> = (0..models.len()).into_iter().map(|_| 0).collect();
        let mut instance_data = InstanceData {
            model_instances,
            local_transform_buffer: None,
            local_transform_data,
            global_transform_buffer: None,
            global_transform_data,
            instance_local_offsets,
        };
        // not good!
        for instance in scaffold.instances.iter() {
            let gts: Vec<[[f32; 4]; 4]> = scaffold
                .global_transforms
                .iter()
                .filter(|gt| gt.model_index == instance.model_index)
                .map(|gt| gt.transform)
                .collect();
            let _ = instance_data.add_model_instance(models, instance.model_index, gts);
        }
        println!("{:?}", instance_data.model_instances);

        for new_global_transform in scaffold.global_transforms.iter() {
            let mut instance_index = 0;
            for i in 0..new_global_transform.model_index {
                instance_index += instance_data.model_instances[i];
            }
            instance_index += new_global_transform.instance_index;
            instance_data.update_global_transform_x(instance_index, new_global_transform.transform);
        }

        instance_data
    }

    pub fn update_global_transform_x(&mut self, instance_idx: usize, new_transform: [[f32; 4]; 4]) {
        let t = GlobalTransform {
            transform_matrix: cgmath::Matrix4::from(new_transform),
        };
        self.global_transform_data[instance_idx] = t * self.global_transform_data[instance_idx];
    }

    pub fn init(&mut self, device: &wgpu::Device) {
        let local_transform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Local transform buffer"),
            contents: bytemuck::cast_slice(&self.local_transform_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
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

    fn add_mesh_transforms(
        &mut self,
        new_instance_count: usize,
        offset: usize,
        base_model_index: usize,
        base_transform_index: usize,
        mut transform_slice: Vec<LocalTransform>,
    ) -> Vec<LocalTransform> {
        // we want to place the new instance of this mesh right after the last instance
        // of the mesh. That will be:
        // offset (relative offset for this slice)
        // plus i (the number of instances we have already added in this pass)
        // plus model instance count (for one model, there is one instance of this
        // particular mesh instance, and so on)
        for i in 0..new_instance_count {
            let mut new_transform = transform_slice[offset].clone();
            new_transform.model_index = (i + base_model_index) as u32;
            transform_slice.insert(offset + i + base_transform_index, new_transform);
        }
        transform_slice
    }
    // this function is hilariously costly! probably warrants a restructuring of instancedata
    // even though it really shouldnt be in any hot loop
    // second note: the below code gets confusing because we are working with two type of "instance" semantically.
    // the first refers to a mesh instance for a model. There can be multiple instances of the same
    // mesh in a single model
    // the second refers to an instance OF THAT INSTANCE. so if there are two mesh instances (first
    // definition) in a model, and we want to add a new model, we need to add one new instance
    // (second definition) to each of those mesh instances (first definition)
    fn add_new_instances_local_data(
        &mut self,
        model_index: usize,
        new_instance_count: usize,
        models: &Vec<GModel>,
    ) -> Result<&mut Self, InitializationError> {
        // step 1: create a new vec from all the mesh instances associated with the model
        //
        let model_instance_count = self.model_instances[model_index];
        let model_mesh_count = models[model_index].mesh_instances.iter().sum::<u32>() as usize;
        let tot_model_count = self.model_instances.iter().sum::<usize>();

        // the offset for the first local transform for this model
        let instance_offset = self.instance_local_offsets[model_index];
        // the range of all the transforms that pertain to this model
        let model_mesh_range: Range<usize> = Range {
            start: instance_offset,
            end: model_mesh_count * model_instance_count + instance_offset,
        };
        let mut transform_slice = self.local_transform_data[model_mesh_range.clone()].to_vec();

        // step 2: expand the vec with the appropriate number of new transforms
        let mut offset = 0;
        for mesh_instance_count in models[model_index].mesh_instances.iter() {
            for _ in 0..*mesh_instance_count {
                transform_slice = self.add_mesh_transforms(
                    new_instance_count,
                    offset,
                    tot_model_count,
                    model_instance_count,
                    transform_slice,
                );
                offset += new_instance_count + model_instance_count;
            }
        }

        // step 3: splice the local transform data vec with this new expanded vec
        self.local_transform_data
            .splice(model_mesh_range, transform_slice);
        // step 4: increase the offsets for all the models after this one by the number of new
        // instances just added
        let num_new_instances = model_mesh_count * new_instance_count;
        for offset_val in self
            .instance_local_offsets
            .iter_mut()
            .skip(instance_offset + 1)
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
        let new_instance_count = global_transforms.len();
        self.global_transform_data.extend(global_transforms);

        // increment the number of mesh instances by num new instances
        self.model_instances[model_index] += new_instance_count;

        Ok(())
    }

    /// merge the instance data together
    pub fn merge(mut self, mut other: Self, models: &Vec<GModel>) -> Self {
        let number_of_models = self.model_instances.iter().sum::<usize>();
        for local_transform in other.local_transform_data.iter_mut() {
            local_transform.model_index += number_of_models as u32;
        }
        self.model_instances.extend(other.model_instances);
        let model_instances = self.model_instances;

        let instance_local_offsets = calculate_model_mesh_offsets(models, &model_instances);

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
