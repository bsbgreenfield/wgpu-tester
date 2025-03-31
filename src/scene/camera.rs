use wgpu::util::DeviceExt;
pub struct Camera {
    fov: f32,
    aspect_ratio: f32,
    zfar: f32,
    znear: f32,
    eye_pos: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
}
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

impl Camera {
    pub fn new(fov: f32, aspect_ratio: f32, znear: f32, zfar: f32) -> Self {
        Self {
            fov,
            aspect_ratio,
            zfar,
            znear,
            eye_pos: cgmath::Point3 {
                x: 0.0,
                y: 1.0,
                z: 10.0,
            },
            up: cgmath::Vector3::unit_y(),
            target: cgmath::Point3::new(0.0, 0.0, 0.0),
        }
    }
    pub fn perspective_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye_pos, self.target, self.up);
        let proj = cgmath::perspective(
            cgmath::Rad(self.fov),
            self.aspect_ratio,
            self.znear,
            self.zfar,
        );
        println!("{:?}", view);
        println!("{:?}", proj);

        proj * view
    }

    pub fn get_buffer(&self, camera_uniform: CameraUniform, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}
impl CameraUniform {
    pub fn new(camera: &Camera) -> Self {
        Self {
            view_proj: (OPENGL_TO_WGPU_MATRIX * camera.perspective_matrix()).into(),
        }
    }
}

pub fn get_camera_bind_group(
    camera_buffer: &wgpu::Buffer,
    device: &wgpu::Device,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    let camera_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Camera bind group layout"),
        });

    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
        label: Some("camera bind group"),
    });

    (camera_bind_group_layout, camera_bind_group)
}
pub fn get_camera_default(aspect_ratio: f32) -> (Camera, CameraUniform) {
    let camera = Camera::new(std::f32::consts::FRAC_PI_4, aspect_ratio, 0.1, 100.0);
    let camera_uniform: CameraUniform = CameraUniform::new(&camera);
    (camera, camera_uniform)
}
