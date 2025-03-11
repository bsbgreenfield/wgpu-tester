use crate::vertex::Vertex;
pub const INDICES: [u32; 3] = [0, 1, 2];
pub const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.5, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
    },
];
