use crate::model::vertex::ModelVertex;
pub const INDICES: &[u32; 36] = &[
    0, 1, 2, 0, 2, 3, 3, 2, 4, 3, 4, 5, 5, 4, 6, 5, 6, 7, 7, 6, 1, 7, 1, 0, 7, 0, 3, 7, 3, 5, 4, 2,
    1, 4, 2, 6,
];
pub const VERTICES: &[ModelVertex] = &[
    // front face 1
    ModelVertex::new(&[-0.5, 0.5, 0.0]),
    ModelVertex::new(&[-0.5, -0.5, 0.0]),
    ModelVertex::new(&[0.5, -0.5, 0.0]),
    ModelVertex::new(&[0.5, 0.5, 0.0]),
    ModelVertex::new(&[0.5, -0.5, -1.0]),
    ModelVertex::new(&[0.5, 0.5, -1.0]),
    ModelVertex::new(&[-0.5, -0.5, -1.0]),
    ModelVertex::new(&[-0.5, 0.5, -1.0]),
];
