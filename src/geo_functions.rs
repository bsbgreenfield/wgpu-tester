type Mat4x4 = [[f32; 4]; 4];

pub fn rotate_45() -> Mat4x4 {
    const fval: f32 = std::f32::consts::PI / 4.0;
    let cos_45 = fval.cos();
    let sin_45 = fval.sin();
    let ret: Mat4x4 = [
        [cos_45, -1.0 * sin_45, 0.0, 0.0],
        [sin_45, cos_45, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    transpose4x4(ret)
}

pub fn rotate_about_z(theta: f32) -> Mat4x4 {
    let fval: f32 = theta;
    let cos_45 = fval.cos();
    let sin_45 = fval.sin();
    let ret: Mat4x4 = [
        [cos_45, -1.0 * sin_45, 0.0, 0.0],
        [sin_45, cos_45, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    transpose4x4(ret)
}

pub fn rotate_about_y(theta: f32) -> Mat4x4 {
    let fval: f32 = theta;
    let cos = fval.cos();
    let sin = fval.sin();
    let ret: Mat4x4 = [
        [cos, 0.0, -sin, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [sin, 0.0, cos, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    transpose4x4(ret)
}

pub const fn identity() -> Mat4x4 {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

pub fn transpose4x4(mut source: Mat4x4) -> Mat4x4 {
    let copy: Mat4x4 = source.clone();
    for i in 0..4 {
        for j in 0..4 {
            source[j][i] = copy[i][j];
        }
    }

    source
}
