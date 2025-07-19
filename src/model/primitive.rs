use std::ops::Range;

use gltf::Primitive;

use crate::{
    model::{
        util::{
            copy_binary_data_from_gltf, get_index_offset_len, AttributeType, GltfErrors,
            InitializationError,
        },
        vertex::ModelVertex,
    },
    scene::scene::PrimitiveData,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct GPrimitive {
    pub initialized_vertex_offset_len: Option<(u32, u32)>,
    pub initialized_index_offset_len: Option<(u32, u32)>,
}
impl GPrimitive {
    pub fn new() -> Self {
        Self {
            initialized_vertex_offset_len: None,
            initialized_index_offset_len: None,
        }
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
        data: &PrimitiveData,
        index_ranges: &Vec<Range<usize>>,
    ) -> Result<(), InitializationError> {
        // upon creation, this primitive will have stored its offset and length relative to the
        // main byte buffer. Also at this stage, scene_buffer_data has stored a list of ranges that
        // need to be composed into the final index buffer. We need to translate the indices
        // relative to the main buffer to indices relative to a buffer which would contain only the
        // ranges specified in scene_buffer_data.
        let mut relative_buffer_offset = 0;
        let offset = data.indices_offset;
        let len = data.indices_len;
        for index_range in index_ranges.iter() {
            if offset > index_range.end {
                relative_buffer_offset += index_range.len();
            } else {
                relative_buffer_offset += offset - index_range.start;
                // paranoid?
                if (offset + len) as usize > index_range.end {
                    return Err(InitializationError::SceneInitializationError);
                }
                break;
            }
        }

        self.initialized_index_offset_len =
            Some(((relative_buffer_offset / 2) as u32, len as u32 / 2));
        Ok(())
    }
}

impl PrimitiveData {
    pub(super) fn from_data(
        mesh_id: usize,
        primitive: Primitive,
        buffer_offsets: &Vec<u64>,
        binary_data: &Vec<u8>,
    ) -> Result<Self, GltfErrors> {
        let (_, position_accessor) = primitive
            .attributes()
            .find(|a| a.0 == gltf::Semantic::Positions)
            .unwrap();

        let (_, maybe_normals_accessor) = match primitive
            .attributes()
            .find(|a| a.0 == gltf::Semantic::Normals)
        {
            Some(normals) => (normals.0, Some(normals.1)),
            None => (gltf::Semantic::Normals, None),
        };
        let maybe_indices_accessor = primitive.indices();
        let (_, maybe_joints0_accessor) = match primitive
            .attributes()
            .find(|a| a.0 == gltf::Semantic::Joints(0))
        {
            Some(joints) => (joints.0, Some(joints.1)),
            None => (gltf::Semantic::Joints(0), None),
        };
        let (_, maybe_weights_accessor) = match primitive
            .attributes()
            .find(|a| a.0 == gltf::Semantic::Weights(0))
        {
            Some(weights) => (weights.0, Some(weights.1)),
            None => (gltf::Semantic::Weights(0), None),
        };
        let positions = copy_binary_data_from_gltf(
            &position_accessor,
            AttributeType::Position,
            buffer_offsets,
            binary_data,
        )?;
        let (indices_offset, indices_len) =
            get_index_offset_len(maybe_indices_accessor.as_ref(), buffer_offsets)?
                .unwrap_or((0, 0));
        let mut normals = None;
        let mut joints = None;
        let mut weights = None;
        if let Some(normals_accesor) = maybe_normals_accessor {
            normals = Some(copy_binary_data_from_gltf(
                &normals_accesor,
                AttributeType::Normal,
                buffer_offsets,
                binary_data,
            )?);
        }
        if let Some(joints_accesor) = maybe_joints0_accessor {
            joints = Some(copy_binary_data_from_gltf(
                &joints_accesor,
                AttributeType::Joints,
                buffer_offsets,
                binary_data,
            )?);
        }
        if let Some(weights_accesor) = maybe_weights_accessor {
            weights = Some(copy_binary_data_from_gltf(
                &weights_accesor,
                AttributeType::Weights,
                buffer_offsets,
                binary_data,
            )?);
        }
        Ok(Self {
            mesh_id,
            positions,
            indices_offset,
            indices_len,
            normals,
            joints,
            weights,
        })
    }
    pub(super) fn get_vertex_data(&self) -> Vec<ModelVertex> {
        let position_f32: &[f32] = bytemuck::cast_slice(&self.positions);
        let normals_f32: Option<Vec<f32>> = match &self.normals {
            Some(normals) => Some(bytemuck::cast_slice(normals).to_vec()),
            None => None,
        };
        let joints_u16: Option<Vec<u16>> = match &self.joints {
            Some(joints) => Some(bytemuck::cast_slice(&joints).to_vec()),
            None => None,
        };
        let weights_f32: Option<Vec<f32>> = match &self.weights {
            Some(weights) => Some(bytemuck::cast_slice(&weights).to_vec()),
            None => None,
        };
        let weights_normalized = if let Some(w) = weights_f32 {
            Some(Self::normalize_f32_to_u8(w.to_vec()))
        } else {
            None
        };
        let vertex_vec: Vec<ModelVertex> = (0..(position_f32.len() / 3))
            .map(|i| {
                let normal = match &normals_f32 {
                    Some(n) => n[i * 3..i * 3 + 3].try_into().unwrap(),
                    None => [0.0, 0.0, 0.0],
                };
                let joints = match &joints_u16 {
                    Some(j) => j[i * 4..i * 4 + 4].try_into().unwrap(),
                    None => [0, 0, 0, 0],
                };
                let weights = match &weights_normalized {
                    Some(w) => &w[i * 4..i * 4 + 4],
                    None => &[0, 0, 0, 0],
                };
                println!("POSITION {:?}", &position_f32[i * 3..i * 3 + 3]);
                println!(
                    "JOINTS {:?}",
                    &[
                        joints[0] as u8,
                        joints[1] as u8,
                        joints[2] as u8,
                        joints[3] as u8,
                    ]
                );
                println!("WEIGHTS {:?}\n\n", weights);

                return ModelVertex {
                    position: position_f32[i * 3..i * 3 + 3].try_into().unwrap(),
                    normal: normal.try_into().unwrap(),
                    joints: [
                        joints[0] as u8,
                        joints[1] as u8,
                        joints[2] as u8,
                        joints[3] as u8,
                    ],
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
                assert!(x <= 1.0 && x >= 0.0);
                let scaled = (x * 255.0).round();
                scaled.clamp(0.0, 255.0) as u8
            })
            .collect()
    }
}
