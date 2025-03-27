use cgmath::Angle;
use code_notes::run;

fn main() {
    let v1 = cgmath::Vector4::<f32>::new(-0.5, 0.5, 0.0, 1.0);
    let v2 = cgmath::Vector4::new(-0.5, -0.5, 0.0, 1.0);
    let v3 = cgmath::Vector4::new(0.5, -0.5, 0.0, 1.0);

    let aspect_ratio = 0.5;
    let c0 = cgmath::Vector4::<f32> {
        x: aspect_ratio,
        y: 0.0,
        z: 0.0,
        w: 0.0,
    };
    let c1 = cgmath::Vector4::<f32> {
        x: 0.0,
        y: 1.0,
        z: 0.0,
        w: 0.0,
    };
    let c2 = cgmath::Vector4::<f32> {
        x: 0.0,
        y: 0.0,
        z: 1.0,
        w: 0.0,
    };
    let c3 = cgmath::Vector4::<f32> {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };
    let fov = cgmath::Rad(std::f32::consts::FRAC_PI_4);
    let scaler_matrix = cgmath::Matrix4::<f32>::from_scale(0.3);
    let rotator_matrix = cgmath::Matrix4::<f32>::from_angle_z(cgmath::Deg(25.0));
    let translate_x = cgmath::Matrix4::<f32>::from_translation(cgmath::Vector3::new(0.6, 0.0, 0.0));
    let translate_z =
        cgmath::Matrix4::<f32>::from_translation(cgmath::Vector3::new(0.0, 0.0, -0.5));

    let camera_matrix = cgmath::Matrix4::<f32>::from_cols(c0, c1, c2, c3);
    println!("tranlation matrix: {:?}", translate_x);
    println!("scaler matrix: {:?}", scaler_matrix);
    println!("rotater_matrix: {:?}", rotator_matrix);

    println!("{:?}", translate_x * v1);
    println!("{:?}", translate_x * v2);
    println!("{:?}", translate_x * v3);
    run();
}
