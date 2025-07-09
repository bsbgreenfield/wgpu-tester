use std::sync::Arc;
use wgpu::{BindGroupEntry, BindGroupLayoutEntry, BufferBindingType};
use winit::window::Window;

#[allow(unused_imports)]
use crate::scene::scene_scaffolds::{BOX_ANIMATED, BUGGY, FOX, MONKEY, POLLY};
#[allow(unused_imports)]
use crate::{
    app::app_config::AppConfig,
    scene::{
        scene::GScene,
        scene_scaffolds::{BRAIN, CUBE, TRUCK},
    },
};

pub(super) async fn setup_config<'a>(window: Arc<Window>) -> AppConfig<'a> {
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
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
        })
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
pub(super) fn create_diffuse_bgl(app_config: &AppConfig) -> wgpu::BindGroupLayout {
    app_config
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("diffuse bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
}

pub(super) fn create_base_color_bgl(app_config: &AppConfig) -> wgpu::BindGroupLayout {
    app_config
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("base color bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
}
/// add this materials base color to the vec whic his to be used as the data for a buffer that
/// stores all base colors needed for the scene. Return the index within this buffer, which will
/// be used as the offset for the base color bind group for this material
pub(super) fn add_base_color(base_color_vec: &mut Vec<[f32; 4]>, base_color: [f32; 4]) -> usize {
    if base_color == [1.0, 1.0, 1.0, 1.0] {
        return 0; // the default base color, always stored at offset 0.
    }
    for (idx, color) in base_color_vec.iter().enumerate().skip(1) {
        if color == &base_color[0..4] {
            return idx + 1;
        }
    }
    base_color_vec.push(base_color);
    return base_color_vec.len() - 1;
}

pub(super) fn setup_global_instance_bind_group(
    app_config: &AppConfig,
    scene: &GScene,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    let global_instance_bind_group_layout =
        app_config
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Global bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
    let global_instance_bind_group =
        app_config
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &global_instance_bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 1,
                    resource: scene
                        .get_global_buf()
                        .expect("should be initialized")
                        .as_entire_binding(),
                }],
                label: Some("Global bind group"),
            });
    (
        global_instance_bind_group_layout,
        global_instance_bind_group,
    )
}

pub(super) fn get_scene<'a>(device: &wgpu::Device, aspect_ratio: f32) -> GScene<'a> {
    FOX.create(device, aspect_ratio).unwrap()
}
