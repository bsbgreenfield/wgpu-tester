use std::vec;

use crate::{scene::scene::SceneScaffold, vertex::Vertex};
pub const INDICES: [u32; 3] = [0, 1, 2];
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
    // Vertex {
    //     position: [0.5, 0.5, 0.0], // 3
    // },
    // Vertex {
    //     position: [0.5, 0.5, 0.5], // 4
    // },
    // Vertex {
    //     position: [0.5, -0.5, 0.5], // 5
    // },
    // back face
    // Vertex {
    //     position: [0.5, 0.5, 0.5], // 4
    // },
    // Vertex {
    //     position: [0.5, -0.5, 0.5], // 5
    // },
    // Vertex {
    //     position: [-0.5, -0.5, 0.5], // 6
    // },
    // Vertex {
    //     position: [-0.5, 0.5, 0.5], // 7
    // },
    // right side

    // index 3
    // index 2
    // index 6
    // index 3
    // index 6
    // index 7
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
