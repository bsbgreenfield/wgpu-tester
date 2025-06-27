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
}

impl GPrimitive {
    pub(super) fn new(primitive: Primitive) -> Result<Self, GltfErrors> {
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
        let indices_accessor = primitive.indices().unwrap();

        let (position_offset, position_length) =
            get_primitive_data(Some(&position_accessor), AttributeType::Position)?.ok_or(
                GltfErrors::VericesError(String::from("could not extract position data")),
            )?;
        let (normal_offset, normal_length) =
            get_primitive_data(normals_accessor.as_ref(), AttributeType::Normal)?.unwrap_or((0, 0));
        let (indices_offset, indices_length) =
            get_primitive_data(Some(&indices_accessor), AttributeType::Index)?.ok_or(
                GltfErrors::IndicesError(String::from("could not extract index data")),
            )?;

        Ok(Self {
            position_offset,
            position_length,
            normal_offset,
            normal_length,
            indices_offset,
            indices_length,
            initialized_vertex_offset_len: None,
            initialized_index_offset_len: None,
        })
    }
    pub(super) fn get_vertex_data(&self, main_buffer_data: &Vec<u8>) -> Vec<ModelVertex> {
        let position_bytes = &main_buffer_data
            [self.position_offset as usize..(self.position_offset + self.position_length) as usize];
        let position_f32: &[f32] = bytemuck::cast_slice(position_bytes);
        let normal_bytes = &main_buffer_data
            [self.normal_offset as usize..(self.normal_offset + self.normal_length) as usize];
        let normals_f32: &[f32] = bytemuck::cast_slice(normal_bytes);
        assert_eq!(normals_f32.len(), position_f32.len());
        let vertex_vec: Vec<ModelVertex> = (0..(position_f32.len() / 3))
            .map(|i| ModelVertex {
                position: position_f32[i * 3..i * 3 + 3].try_into().unwrap(),
                normal: normals_f32[i * 3..i * 3 + 3].try_into().unwrap(),
            })
            .collect();
        vertex_vec
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
