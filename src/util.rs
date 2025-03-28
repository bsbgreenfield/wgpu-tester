use std::{ops::Deref, sync::Arc};
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::{object::Object, vertex::Vertex};

use crate::app::app_config::AppConfig;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

pub async fn setup_config<'a>(window: Arc<Window>) -> AppConfig<'a> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        ..Default::default()
    });
    let surface = instance.create_surface(Arc::clone(&window)).unwrap();

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        )
        .await
        .unwrap();

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(surface_caps.formats[0]);

    let size = window.inner_size();
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: Vec::new(),
        desired_maximum_frame_latency: 2,
    };

    AppConfig {
        size,
        surface,
        device,
        queue,
        config,
    }
}
pub fn create_vertex_bind_group<B>(
    buffer_data: B,
    device: &wgpu::Device,
    label: Option<&str>,
    buf_label: Option<&str>,
    buffer_usage: wgpu::BufferUsages,
    binding_type: wgpu::BindingType,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup)
where
    B: Copy + Clone + bytemuck::Zeroable + bytemuck::Pod,
{
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: buf_label,
        contents: bytemuck::cast_slice(&[buffer_data]),
        usage: buffer_usage,
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: binding_type,
            count: None,
        }],
        label,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
        label,
    });

    (bind_group_layout, bind_group)
}

pub fn create_objects(
    vertices: Vec<&[Vertex]>,
    indices: Vec<&[u32]>,
    device: &wgpu::Device,
) -> Vec<Object> {
    let mut objects = Vec::with_capacity(vertices.len());
    for data in vertices.iter().zip(indices.iter()) {
        // let mut vertices_t: Vec<Vertex> = Vec::with_capacity(3);
        // // transform each vertex by dividing by its z value
        // for v in data.0.iter() {
        //     let x = v.position[0] / v.position[2];
        //     let y = v.position[1] / v.position[2];
        //     vertices_t.push(Vertex {
        //         position: [x, y, 1.0],
        //     });
        // }
        let o = Object::from_vertices(data.0, data.1, device);
        objects.push(o);
    }
    objects
}
