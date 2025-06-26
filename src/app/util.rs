use std::sync::Arc;
use winit::window::Window;

use crate::scene::scene_scaffolds::BOX_ANIMATED;
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

pub(super) fn get_scene(app_config: &AppConfig, aspect_ratio: f32) -> GScene {
    BOX_ANIMATED
        .create(&app_config.device, aspect_ratio)
        .unwrap()
}
