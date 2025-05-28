use std::sync::Arc;
use winit::window::Window;

use crate::{
    app::app_config::AppConfig,
    model::{model::GlobalTransform, util::load_gltf},
    scene::{
        scene::GScene,
        scene_scaffolds::{BRAIN, BUGGY, BUGGY_BOX, CUBE, DRAGON, FOX, TRUCK, TRUCK_BOX},
    },
};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

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
    //let box_scene = load_gltf("box", &app_config.device, aspect_ratio).unwrap();
    //let truck_scene = load_gltf("milk-truck", &app_config.device, aspect_ratio).unwrap();

    //let offset_x =
    //    cgmath::Matrix4::<f32>::from_translation(cgmath::Vector3::<f32>::new(4.8, 0.5, 0.0));
    //let offset_3 =
    //    cgmath::Matrix4::<f32>::from_translation(cgmath::Vector3::<f32>::new(14.8, 0.5, 0.0));
    //let offset_y: [[f32; 4]; 4] =
    //    cgmath::Matrix4::<f32>::from_translation(cgmath::Vector3::<f32>::new(-5.0, -6.0, 0.0))
    //        .into();
    //let mut gscene = GScene::merge(truck_scene, box_scene).unwrap();
    //gscene.update_global_transform(1, 0, offset_x.into());
    //let mut box_transforms = Vec::<[[f32; 4]; 4]>::new();
    //let mut x = 5.0;
    //let mut z = 1.0;
    //let y = 0.0;
    //for i in 0..3 {
    //    if i % 15 == 0 {
    //        z += 7.5;
    //        x = 1.0
    //    } else {
    //        x += 7.5
    //    }
    //    box_transforms
    //        .push(cgmath::Matrix4::from_translation(cgmath::Vector3::new(x, y, -z)).into());
    //}
    //gscene.add_model_instances(
    //    0,
    //    vec![cgmath::Matrix4::from_translation(cgmath::Vector3::new(-5.0, 0.0, 0.0)).into()],
    //);
    //gscene.add_model_instances(0, box_transforms);
    //gscene.add_model_instances(1, vec![offset_3.into()]);
    //gscene.init(&app_config.device);
    //gscene
    let a = TRUCK.create(&app_config.device, aspect_ratio).unwrap();
    a
}
