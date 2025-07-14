use std::ops::Range;

use gltf::Primitive;

use crate::model::{
    util::{get_primitive_data, AttributeType, GltfErrors, InitializationError},
    vertex::ModelVertex,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct GPrimitive {
    position_offset: u32,
    position_length: u32,
    normal_offset: u32,
    normal_length: u32,
    pub indices_offset: u32,
    pub indices_length: u32,
    pub initialized_vertex_offset_len: Option<(u32, u32)>,
    pub initialized_index_offset_len: Option<(u32, u32)>,
    joints_offset: u32,
    joints_length: u32,
    weights_offset: u32,
    weights_length: u32,
}

impl GPrimitive {
    pub(super) fn new(primitive: Primitive, buffer_offsets: &Vec<u64>) -> Result<Self, GltfErrors> {
        let (_, position_accessor) = primitive
            .attributes()
            .find(|a| a.0 == gltf::Semantic::Positions)
            .unwrap();

        let (_, normals_accessor) = match primitive
            .attributes()
            .find(|a| a.0 == gltf::Semantic::Normals)
        {
            Some(normals) => (normals.0, Some(normals.1)),
            None => (gltf::Semantic::Normals, None),
        };
        let indices_accessor = primitive.indices();
        let (_, joints0_accessor) = match primitive
            .attributes()
            .find(|a| a.0 == gltf::Semantic::Joints(0))
        {
            Some(joints) => (joints.0, Some(joints.1)),
            None => (gltf::Semantic::Joints(0), None),
        };
        let (_, weights_accessor) = match primitive
            .attributes()
            .find(|a| a.0 == gltf::Semantic::Weights(0))
        {
            Some(weights) => (weights.0, Some(weights.1)),
            None => (gltf::Semantic::Weights(0), None),
        };

        let (position_offset, position_length) = get_primitive_data(
            Some(&position_accessor),
            AttributeType::Position,
            buffer_offsets,
        )?
        .ok_or(GltfErrors::VericesError(String::from(
            "could not extract position data",
        )))?;
        let (normal_offset, normal_length) = get_primitive_data(
            normals_accessor.as_ref(),
            AttributeType::Normal,
            buffer_offsets,
        )?
        .unwrap_or((0, 0));
        let (indices_offset, indices_length) = get_primitive_data(
            indices_accessor.as_ref(),
            AttributeType::Index,
            buffer_offsets,
        )?
        .unwrap_or((0, 0));

        let (joints_offset, joints_length) = get_primitive_data(
            joints0_accessor.as_ref(),
            AttributeType::Joints,
            buffer_offsets,
        )?
        .unwrap_or((0, 0));

        let (weights_offset, weights_length) = get_primitive_data(
            weights_accessor.as_ref(),
            AttributeType::Weights,
            buffer_offsets,
        )?
        .unwrap_or((0, 0));
        Ok(Self {
            position_offset,
            position_length,
            normal_offset,
            normal_length,
            indices_offset,
            indices_length,
            initialized_vertex_offset_len: None,
            initialized_index_offset_len: None,
            joints_offset,
            joints_length,
            weights_offset,
            weights_length,
        })
    }
    pub(super) fn get_vertex_data(&self, main_buffer_data: &Vec<u8>) -> Vec<ModelVertex> {
        let position_bytes = &main_buffer_data
            [self.position_offset as usize..(self.position_offset + self.position_length) as usize];
        let position_f32: &[f32] = bytemuck::cast_slice(position_bytes);
        let normal_bytes = &main_buffer_data
            [self.normal_offset as usize..(self.normal_offset + self.normal_length) as usize];
        let normals_f32: &[f32] = bytemuck::cast_slice(normal_bytes);
        let joints_bytes = &main_buffer_data
            [self.joints_offset as usize..(self.joints_offset + self.joints_length) as usize];
        println!("{joints_bytes:?}");
        let weights_bytes = &main_buffer_data
            [self.weights_offset as usize..(self.weights_offset + self.weights_length) as usize];
        let normalized_weights =
            Self::normalize_f32_to_u8(bytemuck::cast_slice(weights_bytes).to_vec());
        if normals_f32.len() > 0 {
            assert_eq!(normals_f32.len(), position_f32.len());
        }
        let vertex_vec: Vec<ModelVertex> = (0..(position_f32.len() / 3))
            .map(|i| {
                return ModelVertex {
                    position: position_f32[i * 3..i * 3 + 3].try_into().unwrap(),
                    normal: if normals_f32.len() > 0 {
                        normals_f32[i * 3..i * 3 + 3].try_into().unwrap()
                    } else {
                        [0.0, 0.0, 0.0]
                    },
                    joints: if joints_bytes.len() > 0 {
                        let j = joints_bytes[i * 4..i * 4 + 4].try_into().unwrap();
                        println!("joints: {:?}", j);
                        j
                    } else {
                        println!("joints: {:?}", [0, 0, 0, 0]);
                        [0, 0, 0, 0]
                    },
                    weights: if normalized_weights.len() > 0 {
                        let w = normalized_weights[i * 4..i * 4 + 4].try_into().unwrap();
                        println!("weights: {:?}", w);
                        w
                    } else {
                        println!("weights: {:?}", [0, 0, 0, 0]);
                        [0, 0, 0, 0]
                    },
                };
            })
            .collect();

        vertex_vec
    }
    fn normalize_f32_to_u8(input: Vec<f32>) -> Vec<u8> {
        input
            .into_iter()
            .map(|x| {
                let scaled = (x * 255.0).round();
                scaled.clamp(0.0, 255.0) as u8
            })
            .collect()
    }
    pub(super) fn get_index_data(
        main_buffer_data: &Vec<u8>,
        indices_ranges: &Vec<std::ops::Range<usize>>,
    ) -> Vec<u16> {
        let mut index_vec: Vec<u16> = Vec::new();
        for range in indices_ranges.iter() {
            let indices_bytes: &[u8] = &main_buffer_data[range.start..range.end];
            let indices_u16: &[u16] = bytemuck::cast_slice::<u8, u16>(indices_bytes);
            index_vec.extend(indices_u16.to_vec());
        }
        index_vec
    }

    pub(super) fn set_relative_indices_offset(
        &mut self,
        index_ranges: &Vec<Range<usize>>,
    ) -> Result<(), InitializationError> {
        // upon creation, this primitive will have stored its offset and length relative to the
        // main byte buffer. Also at this stage, scene_buffer_data has stored a list of ranges that
        // need to be composed into the final index buffer. We need to translate the indices
        // relative to the main buffer to indices relative to a buffer which would contain only the
        // ranges specified in scene_buffer_data.
        let mut relative_buffer_offset = 0;
        for index_range in index_ranges.iter() {
            if self.indices_offset as usize > index_range.end {
                relative_buffer_offset += index_range.len();
            } else {
                relative_buffer_offset += self.indices_offset as usize - index_range.start;
                // paranoid?
                if (self.indices_offset + self.indices_length) as usize > index_range.end {
                    return Err(InitializationError::SceneInitializationError);
                }
                break;
            }
        }

        self.initialized_index_offset_len =
            Some(((relative_buffer_offset / 2) as u32, self.indices_length / 2));
        Ok(())
    }
}
