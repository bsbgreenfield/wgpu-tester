use crate::{
    model::{animation::animation::AnimationFrame, model::GModel},
    scene::{scene_scaffolds::SceneScaffold, util::calculate_model_mesh_offsets},
    transforms,
};
use cgmath::SquareMatrix;
use std::ops::Range;
use wgpu::util::DeviceExt;

use crate::model::{
    model::{GlobalTransform, LocalTransform},
    util::InitializationError,
};

pub(super) struct InstanceData {
    pub model_instances: Vec<usize>,
    pub local_transform_buffer: Option<wgpu::Buffer>,
    pub local_transform_data: Vec<LocalTransform>,
    pub global_transform_buffer: Option<wgpu::Buffer>,
    pub global_transform_data: Vec<[[f32; 4]; 4]>,
    /// These are the offsets which correspond to the slot
    /// of the first local transform for this model.
    /// All local transforms which refer to a model would be located in the range from
    /// [instance_local_offsets[model_idx] .. (instance_local_offsets[model_idx] +
    /// model.mesh_instance_count * model_instances[model_idx])]
    model_instances_local_offsets: Vec<usize>,
    pub joint_global_transforms: Vec<[[f32; 4]; 4]>,
    pub joint_transform_buffer: Option<wgpu::Buffer>,
}

impl InstanceData {
    pub fn get_instance_local_offset(
        &self,
        instance_idx: usize,
        model_idx: usize,
    ) -> (usize, usize) {
        // the location of the first instance of this model in the local transform buffer
        let model_local_offset = self.model_instances_local_offsets[model_idx];
        let model_mesh_count = (self.model_instances_local_offsets[model_idx + 1]
            - model_local_offset)
            / self.model_instances[model_idx];
        return (
            model_local_offset + (instance_idx * model_mesh_count),
            model_mesh_count,
        );
    }

    /// unsafe function
    /// Each animation frame contains a reference to an array x.
    /// This array contains one or more arrays of raw matrices [[f32;4];4] y_matrix.
    /// for each y_matrix in x, we want replace a region of local_transform data
    /// at the specified offset for y_matrix. y_matrix has exactly the same length as this region
    pub fn apply_animation_frame_unchecked(&mut self, animation_frame: AnimationFrame) {
        for (idx, offset) in animation_frame.lt_offsets.iter().enumerate() {
            let t_slices = animation_frame.mesh_transform_slices[idx]; // y_matrix slice
            unsafe {
                // the model index stored in the first local transform at the provided offset
                let model_id = self.local_transform_data.get_unchecked(*offset).model_index;
                // the raw pointer to the region of memory that is being ovewritten
                let ptr = self.local_transform_data.as_mut_ptr().add(*offset);
                // for each matrix in y_matrix, overwrite pointer[i] with a LocalTransform
                for (i, matrix) in t_slices.iter().enumerate() {
                    std::ptr::write(
                        ptr.add(i),
                        LocalTransform {
                            transform_matrix: *matrix,
                            model_index: model_id,
                        },
                    );
                }
            }
        }
        // TODO: if we want to animate multiple simultaneous instances, we will need to store
        // separate copyies of the joint global transforms, just like we already do for local
        // transforms
        for (slice_index, joint_indices) in animation_frame.joint_ids.iter().enumerate() {
            for (i, joint_index) in joint_indices.iter().enumerate() {
                self.joint_global_transforms[*joint_index] =
                    animation_frame.joint_transform_slices[slice_index][i];
            }
        }
    }
    /// create Instance data with one instance of each model, each positioned at the origin
    pub fn default_from_scene(
        models: &Vec<GModel>,
        local_transform_data: Vec<LocalTransform>,
        joint_transforms: Vec<[[f32; 4]; 4]>,
    ) -> Self {
        // one instance of each model
        let model_instances: Vec<usize> = (0..models.len()).into_iter().map(|_| 1).collect();
        // every model goes at the origin
        let global_transform_data: Vec<[[f32; 4]; 4]> = (0..models.len())
            .into_iter()
            .map(|_| cgmath::Matrix4::<f32>::identity().into())
            .collect();

        let model_mesh_counts: Vec<usize> = models
            .iter()
            .map(|model| model.mesh_instances.iter().sum::<u32>() as usize)
            .collect();
        let mut model_instances_local_offsets = Vec::with_capacity(models.len());
        model_instances_local_offsets.push(0);
        model_mesh_counts
            .iter()
            .for_each(|count| model_instances_local_offsets.push(*count));
        // leave the final value in order to calculate the last model's mesh count
        Self {
            model_instances,
            local_transform_buffer: None,
            local_transform_data,
            global_transform_buffer: None,
            global_transform_data,
            model_instances_local_offsets,
            joint_global_transforms: joint_transforms,
            joint_transform_buffer: None,
        }
    }

    pub fn from_scaffold(
        scaffold: &SceneScaffold,
        local_transform_data: Vec<LocalTransform>,
        joint_transforms: Vec<[[f32; 4]; 4]>,
        models: &Vec<GModel>,
    ) -> Result<Self, InitializationError> {
        let mut model_instances = Vec::new();
        let mut global_transform_data = Vec::new();
        // fill out the data for model instances and global transform
        // data assuming that there will be one instance of each model
        for _ in models.iter() {
            model_instances.push(1);
            global_transform_data.push(transforms::identity());
        }
        // apply the transform values for the base instances, if any
        for gt_override in scaffold.global_transform_overrides {
            global_transform_data[gt_override.model_idx] = gt_override.transform;
        }
        // build out the model instance offset vec with the proper values,
        // with one extra value at the end to calculate the last model instances mesh count
        let model_mesh_counts: Vec<usize> = models
            .iter()
            .map(|model| model.mesh_instances.iter().sum::<u32>() as usize)
            .collect();
        let mut model_instances_local_offsets: Vec<usize> = Vec::with_capacity(models.len() + 1);
        model_instances_local_offsets.push(0);
        model_mesh_counts.iter().for_each(|mesh_count| {
            model_instances_local_offsets.push(*mesh_count);
        });

        println!(
            "JOINT TRANSFORM LENGTH {:?} * {:?} = {:?}",
            joint_transforms.len(),
            size_of::<[[f32; 4]; 4]>(),
            (joint_transforms.len() * size_of::<[[f32; 4]; 4]>())
        );
        let mut instance_data = Self {
            model_instances,
            local_transform_buffer: None,
            local_transform_data,
            global_transform_buffer: None,
            global_transform_data,
            model_instances_local_offsets,
            joint_global_transforms: joint_transforms,
            joint_transform_buffer: None,
        };

        // add the additional instances
        for additional_instance in scaffold.additional_instances {
            let result = instance_data.add_model_instance(
                models,
                additional_instance.model_index,
                additional_instance.global_transforms.to_vec(),
            );
            if let Err(e) = result {
                return Err(e);
            }
        }
        Ok(instance_data)
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
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let global_transform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                contents: bytemuck::cast_slice(&self.global_transform_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                label: Some("global instance buffer"),
            });

        let joint_buffer = if self.joint_global_transforms.len() > 0 {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                contents: bytemuck::cast_slice(&self.joint_global_transforms),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                label: Some("Joint transform buffer"),
            })
        } else {
            // create the buffer with dummy data
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                contents: bytemuck::cast_slice::<[[f32; 4]; 4], u8>(&[
                    cgmath::Matrix4::<f32>::identity().into(),
                ]),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                label: Some("Joint transform buffer"),
            })
        };
        println!("Creating Joint Buffer with Size {:?}", joint_buffer.size());

        self.global_transform_buffer = Some(global_transform_buffer);
        self.local_transform_buffer = Some(local_transform_buffer);
        self.joint_transform_buffer = Some(joint_buffer);
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
        let instance_offset = self.model_instances_local_offsets[model_index];
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
        let num_new_mesh_instances = model_mesh_count * new_instance_count;
        for offset_val in self
            .model_instances_local_offsets
            .iter_mut()
            .skip(instance_offset + 1)
        {
            *offset_val += num_new_mesh_instances;
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
    #[allow(dead_code)]
    /// merge the instance data together
    pub fn merge(mut self, mut other: Self, models: &Vec<GModel>) -> Self {
        let number_of_models = self.model_instances.iter().sum::<usize>();
        for local_transform in other.local_transform_data.iter_mut() {
            local_transform.model_index += number_of_models as u32;
        }
        self.model_instances.extend(other.model_instances);
        let model_instances = self.model_instances;

        let model_instances_local_offsets = calculate_model_mesh_offsets(models, &model_instances);

        self.local_transform_data.extend(other.local_transform_data);
        let local_transform_data = self.local_transform_data;
        self.global_transform_data
            .extend(other.global_transform_data);

        let global_transform_data = self.global_transform_data;
        let local_transform_buffer = None;
        let global_transform_buffer = None;

        Self {
            model_instances,
            model_instances_local_offsets,
            local_transform_data,
            global_transform_data,
            local_transform_buffer,
            global_transform_buffer,
            joint_global_transforms: todo!(),
            joint_transform_buffer: todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_frame_update() {
        // instance data contains a scene that has two models.
        // The first model has three mesh instances. There are two instances of the model.
        // the second model has four mesh instances. There is one instance of the model.
        let original_matrix_1 = [[1.0; 4]; 4];
        let original_matrix_2 = [[2.0; 4]; 4];
        let mut instance_data_local_transforms = vec![
            LocalTransform {
                transform_matrix: original_matrix_1,
                model_index: 0,
            };
            6
        ];
        let model_2s_transforms = vec![
            LocalTransform {
                transform_matrix: original_matrix_2,
                model_index: 1,
            };
            4
        ];
        instance_data_local_transforms.extend(model_2s_transforms);

        // we want to change the transforms for the second instance of the first model
        // with these values (3 matrices full of 3.0s)
        let new_matrices = vec![[[3f32; 4]; 4]; 3];

        // create the instance_data
        let mut instance_data = InstanceData {
            model_instances: vec![2, 1],
            model_instances_local_offsets: vec![0, 6],
            local_transform_buffer: None,
            local_transform_data: instance_data_local_transforms,
            global_transform_buffer: None,
            global_transform_data: vec![],
            joint_global_transforms: vec![],
            joint_transform_buffer: None,
        };

        //create the animation frame
        let animation_frame = AnimationFrame {
            lt_offsets: vec![3],
            mesh_transform_slices: vec![&new_matrices[..]],
            joint_transform_slices: vec![],
            joint_ids: vec![],
        };

        instance_data.apply_animation_frame_unchecked(animation_frame);

        assert_eq!(
            instance_data.local_transform_data[0].transform_matrix,
            [[1.0; 4]; 4]
        );
        assert_eq!(instance_data.local_transform_data[0].model_index, 0,);
        assert_eq!(
            instance_data.local_transform_data[1].transform_matrix,
            [[1.0; 4]; 4]
        );
        assert_eq!(instance_data.local_transform_data[1].model_index, 0,);
        assert_eq!(
            instance_data.local_transform_data[2].transform_matrix,
            [[1.0; 4]; 4]
        );
        assert_eq!(instance_data.local_transform_data[2].model_index, 0,);
        // the new ones
        assert_eq!(
            instance_data.local_transform_data[3].transform_matrix,
            [[3.0; 4]; 4]
        );
        assert_eq!(instance_data.local_transform_data[3].model_index, 0,);
        assert_eq!(
            instance_data.local_transform_data[4].transform_matrix,
            [[3.0; 4]; 4]
        );
        assert_eq!(instance_data.local_transform_data[4].model_index, 0,);
        assert_eq!(
            instance_data.local_transform_data[5].transform_matrix,
            [[3.0; 4]; 4]
        );
        assert_eq!(instance_data.local_transform_data[5].model_index, 0,);

        // the rest
        assert_eq!(
            instance_data.local_transform_data[6].transform_matrix,
            [[2.0; 4]; 4]
        );
        assert_eq!(instance_data.local_transform_data[6].model_index, 1,);
        assert_eq!(
            instance_data.local_transform_data[7].transform_matrix,
            [[2.0; 4]; 4]
        );
        assert_eq!(instance_data.local_transform_data[7].model_index, 1,);
        assert_eq!(
            instance_data.local_transform_data[8].transform_matrix,
            [[2.0; 4]; 4]
        );
        assert_eq!(instance_data.local_transform_data[8].model_index, 1,);
        assert_eq!(
            instance_data.local_transform_data[9].transform_matrix,
            [[2.0; 4]; 4]
        );
        assert_eq!(instance_data.local_transform_data[9].model_index, 1,);
    }
}
