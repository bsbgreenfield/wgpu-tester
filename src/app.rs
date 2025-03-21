use crate::{
    constants::{INDICES, VERTICES},
    geo_functions,
    object::{self, Object, ObjectTransform, ToRawMatrix},
    vertex,
};
use cgmath::{InnerSpace, Rotation3, Zero};
use std::{clone, sync::Arc};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalSize, Position, Size},
    event::*,
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{self, Window},
};
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CtrUniform {
    trans: [[f32; 4]; 4],
}

impl CtrUniform {
    fn new() -> Self {
        Self {
            trans: (OPENGL_TO_WGPU_MATRIX).into(),
        }
    }
}

fn create_vertex_bind_group<B>(
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

#[derive(Default)]
pub struct App<'a> {
    pub window: Option<Arc<Window>>,
    app_state: Option<AppState<'a>>,
    surface_configured: bool,
}

impl<'a> App<'a> {
    fn update_state(&mut self) {
        self.app_state.as_mut().unwrap().update();
    }
}

pub struct AppState<'a> {
    pub config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    objects: Vec<Object>,
    instance_transform_buffer: wgpu::Buffer,
    bind_groups: Vec<wgpu::BindGroup>,
}

impl<'a> AppState<'a> {
    pub async fn new(window: Arc<Window>) -> Self {
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let (ctr_bind_group_layout, ctr_bind_group) = create_vertex_bind_group(
            CtrUniform::new(),
            &device,
            Some("ctr bind group"),
            Some("ctr buffer"),
            wgpu::BufferUsages::UNIFORM,
            wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        );

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&ctr_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex::Vertex::desc(), object::ObjectTransform::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::Zero,
                            operation: wgpu::BlendOperation::Min,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::all(),
                })],
                compilation_options: Default::default(),
            }),
            depth_stencil: None,
            cache: None,
            multiview: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
        });

        let instance_data = vec![geo_functions::identity()];

        // store the matrix transforms per object instance to be used by the shader
        let instance_transform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let object = Object::from_vertices(VERTICES, &INDICES, &device);
        let objects = vec![object];

        let bind_groups = vec![ctr_bind_group];
        Self {
            config,
            size,
            surface,
            device,
            queue,
            render_pipeline,
            objects,
            instance_transform_buffer,
            bind_groups,
        }
    }

    pub fn update(&mut self) {}

    pub fn draw(&self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.7,
                            g: 0.7,
                            b: 0.5,
                            a: 0.2,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_vertex_buffer(1, self.instance_transform_buffer.slice(..));
            render_pass.set_pipeline(&self.render_pipeline);

            for (idx, bind_group) in self.bind_groups.iter().enumerate() {
                render_pass.set_bind_group(idx as u32, Some(bind_group), &[]);
            }

            use crate::object::DrawObject;
            render_pass.draw_object_instanced(self.objects.first().unwrap(), 0..1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes().with_inner_size(PhysicalSize::new(1500, 1500)),
                    )
                    .unwrap(),
            );
            let app_state = pollster::block_on(AppState::new(window.clone()));
            self.app_state = Some(app_state);
            self.window = Some(window.clone());
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::KeyL),
                        ..
                    },
                ..
            } => {
                if self.window.is_some() {
                    println!("{:?}", self.window.as_ref().unwrap());
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                self.surface_configured = true;
                self.app_state.as_mut().unwrap().resize(physical_size);
            }
            WindowEvent::RedrawRequested => {
                if !self.surface_configured {
                    return;
                }
                self.window.as_ref().unwrap().request_redraw();
                match self.app_state.as_ref().unwrap().draw() {
                    Ok(_) => {}
                    Err(_) => {
                        event_loop.exit();
                    }
                }
            }
            _ => (),
        }
    }
}
