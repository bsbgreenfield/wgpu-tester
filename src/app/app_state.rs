use super::app_config::AppConfig;
use crate::object::{ObjectTransform, ToRawMatrix};
use crate::scene::{MyScene, Scene};
use crate::util;
use crate::vertex::Vertex;
use std::sync::Arc;
use winit::window::Window;

pub struct AppState<'a> {
    app_config: AppConfig<'a>,
    render_pipeline: wgpu::RenderPipeline,
    scene: Box<dyn Scene>,
    bind_groups: Vec<wgpu::BindGroup>,
}

impl<'a> AppState<'a> {
    pub async fn new(window: Arc<Window>) -> Self {
        let app_config: AppConfig = util::setup_config(window).await;
        let shader = app_config
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shader.wgsl").into()),
            });

        let aspect_ratio = (app_config.size.height / app_config.size.width) as f32;
        let mut scene = MyScene::new(&app_config.device, aspect_ratio);
        scene.setup(&app_config.device);
        let camera_buffer = scene
            .camera
            .get_buffer(scene.camera_uniform, &app_config.device);

        let (camera_bind_group_layout, camera_bind_group) =
            crate::scene::get_camera_bind_group(&camera_buffer, &app_config.device);

        let bind_groups = vec![camera_bind_group];

        let render_pipeline_layout =
            app_config
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[&camera_bind_group_layout],
                    push_constant_ranges: &[],
                });
        let render_pipeline =
            app_config
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Render Pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[Vertex::desc(), ObjectTransform::desc()],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: app_config.config.format,
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
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                });

        Self {
            app_config,
            render_pipeline,
            scene: Box::new(scene),
            bind_groups,
        }
    }

    pub fn update(&mut self) {
        //    let rot = cgmath::Matrix4::from_angle_y(cgmath::Deg(-0.4));
        //    self.instance_data.apply_transforms(&[rot]);
        //    let data = self.instance_data.as_raw_data();
        //    self.app_config.queue.write_buffer(
        //        &self.instance_data.instance_transform_buffer,
        //        0,
        //        bytemuck::cast_slice(&data),
        //    );
    }

    pub fn draw(&self) -> Result<(), wgpu::SurfaceError> {
        let output = self.app_config.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.app_config
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
            // set instance buffer
            render_pass.set_vertex_buffer(
                1,
                self.scene
                    .get_instances()
                    .instance_transform_buffer
                    .slice(..),
            );
            render_pass.set_pipeline(&self.render_pipeline);

            // set all bind groups
            for (idx, bind_group) in self.bind_groups.iter().enumerate() {
                render_pass.set_bind_group(idx as u32, Some(bind_group), &[]);
            }

            // for each object, draw as many instances as there are defined in
            // self.instance data for that particular index

            use crate::object::DrawObject;
            for (idx, object) in self.scene.get_objects().iter().enumerate() {
                let num_instances = self.scene.get_instances().object_instances[idx].num_instances;
                let offset = self.scene.get_instances().object_instances[idx].offset_val;
                render_pass.draw_object_instanced(object, offset..num_instances + offset);
            }
        }
        self.app_config
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.app_config.resize(new_size);
    }
}
