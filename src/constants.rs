use crate::vertex::Vertex;
pub const INDICES: [u32; 36] = [
    0, 1, 2, 0, 2, 3, 3, 2, 4, 3, 4, 5, 5, 4, 6, 5, 6, 7, 7, 6, 1, 7, 1, 0, 7, 0, 3, 7, 3, 5, 4, 2,
    1, 4, 2, 6,
];
pub const VERTICES: &[Vertex] = &[
    // front face 1
    Vertex {
        position: [-0.5, 0.5, 0.0], // 0
    },
    Vertex {
        position: [-0.5, -0.5, 0.0], // 1
    },
    Vertex {
        position: [0.5, -0.5, 0.0], // 2
    },
    Vertex {
        position: [0.5, 0.5, 0.0], // 3
    },
    Vertex {
        position: [0.5, -0.5, -1.0], // 4
    },
    Vertex {
        position: [0.5, 0.5, -1.0], // 5
    },
    Vertex {
        position: [-0.5, -0.5, -1.0], // 6
    },
    Vertex {
        position: [-0.5, 0.5, -1.0], // 7
    },
];

pub const INDICES_2: [u32; 6] = [0, 1, 2, 0, 2, 3];
pub const VERTICES_2: &[Vertex] = &[
    // front face 1
    Vertex {
        position: [-0.5, 0.5, 0.0], // 0
    },
    Vertex {
        position: [-0.5, -0.5, 0.0], // 1
    },
    Vertex {
        position: [0.5, -0.5, 0.0], // 2
    },
    Vertex {
        position: [0.5, 0.5, 0.0], // 3
    },
];
