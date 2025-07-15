use std::ops::Range;

use gltf::Primitive;

use crate::model::{
    util::{
        get_data_from_binary, get_primitive_data, AttributeType, GltfErrors, InitializationError,
    },
    vertex::ModelVertex,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct GPrimitive {
    position_offset: u32,
    position_length: u32,
    normal_offset_len: Option<(u32, u32)>,
    pub indices_offset_len: Option<(u32, u32)>,
    pub initialized_vertex_offset_len: Option<(u32, u32)>,
    pub initialized_index_offset_len: Option<(u32, u32)>,
    joints_offset_len: Option<(u32, u32)>,
    weights_offset_len: Option<(u32, u32)>,
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
        let normal_offset_len = get_primitive_data(
            normals_accessor.as_ref(),
            AttributeType::Normal,
            buffer_offsets,
        )?;
        let indices_offset_len = get_primitive_data(
            indices_accessor.as_ref(),
            AttributeType::Index,
            buffer_offsets,
        )?;

        let joints_offset_len = get_primitive_data(
            joints0_accessor.as_ref(),
            AttributeType::Joints,
            buffer_offsets,
        )?;

        let weights_offset_len = get_primitive_data(
            weights_accessor.as_ref(),
            AttributeType::Weights,
            buffer_offsets,
        )?;
        Ok(Self {
            position_offset,
            position_length,
            normal_offset_len,
            indices_offset_len,
            initialized_vertex_offset_len: None,
            initialized_index_offset_len: None,
            joints_offset_len,
            weights_offset_len,
        })
    }
    pub(super) fn get_vertex_data(&self, main_buffer_data: &Vec<u8>) -> Vec<ModelVertex> {
        let position_bytes = &main_buffer_data
            [self.position_offset as usize..(self.position_offset + self.position_length) as usize];
        let position_f32: &[f32] = bytemuck::cast_slice(position_bytes);
        let normals_f32 = match self.normal_offset_len {
            Some((offset, len)) => Some(get_data_from_binary::<f32>(offset, len, main_buffer_data)),
            None => None,
        };
        let joints_u8 = match self.joints_offset_len {
            Some((offset, len)) => Some(get_data_from_binary::<u8>(offset, len, main_buffer_data)),
            None => None,
        };
        let weights_normalized = match self.weights_offset_len {
            Some((offset, len)) => {
                let bytes = get_data_from_binary::<f32>(offset, len, main_buffer_data).to_vec();
                let normalized = Self::normalize_f32_to_u8(bytes);
                Some(normalized)
            }
            None => None,
        };
        let vertex_vec: Vec<ModelVertex> = (0..(position_f32.len() / 3))
            .map(|i| {
                let normal = match normals_f32 {
                    Some(n) => &n[i * 3..i * 3 + 3],
                    None => &[0.0, 0.0, 0.0],
                };
                let joints = match joints_u8 {
                    Some(j) => &j[i * 4..i * 4 + 4],
                    None => &[0, 0, 0, 0],
                };
                let weights = match &weights_normalized {
                    Some(w) => &w[i * 4..i * 4 + 4],
                    None => &[0, 0, 0, 0],
                };
                return ModelVertex {
                    position: position_f32[i * 3..i * 3 + 3].try_into().unwrap(),
                    normal: normal.try_into().unwrap(),
                    joints: joints.try_into().unwrap(),
                    weights: weights.try_into().unwrap(),
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
        let offset_len = self.indices_offset_len.unwrap_or((0, 0));
        for index_range in index_ranges.iter() {
            if offset_len.0 as usize > index_range.end {
                relative_buffer_offset += index_range.len();
            } else {
                relative_buffer_offset += offset_len.0 as usize - index_range.start;
                // paranoid?
                if (offset_len.0 + offset_len.1) as usize > index_range.end {
                    return Err(InitializationError::SceneInitializationError);
                }
                break;
            }
        }

        self.initialized_index_offset_len =
            Some(((relative_buffer_offset / 2) as u32, offset_len.1 / 2));
        Ok(())
    }
}
