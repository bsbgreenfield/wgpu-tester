use super::app_config::AppConfig;
use super::util;
use crate::model::model::{GDrawModel, LocalTransform};
use crate::model::vertex::*;
use crate::scene::scene::GScene;
use std::sync::Arc;
use wgpu::{BindGroupEntry, BindGroupLayoutEntry};
use winit::window::Window;
pub struct InputController {
    pub key_d_down: bool,
    pub key_w_down: bool,
    pub key_a_down: bool,
    pub key_s_down: bool,
    pub key_q_down: bool,
    pub key_e_down: bool,
    pub key_1_down: bool,
    pub key_2_down: bool,
}
impl InputController {
    pub fn new() -> Self {
        Self {
            key_a_down: false,
            key_d_down: false,
            key_s_down: false,
            key_w_down: false,
            key_q_down: false,
            key_e_down: false,
            key_1_down: false,
            key_2_down: false,
        }
    }
}

pub enum UpdateResult {
    UpdateError,
}

pub struct AppState<'a> {
    pub app_config: AppConfig<'a>,
    render_pipeline: wgpu::RenderPipeline,
    pub gscene: GScene,
    bind_groups: Vec<wgpu::BindGroup>,
    pub input_controller: InputController,
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
        let aspect_ratio = (app_config.size.width / app_config.size.height) as f32;
        let gscene = util::get_scene(&app_config, aspect_ratio);
        let (camera_bind_group_layout, camera_bind_group) =
            gscene.get_camera_bind_group(&app_config.device);
        let (global_instance_bind_group_layout, global_instance_bind_group) =
            AppState::setup_global_instance_bind_group(&app_config, &gscene);
        let bind_groups = vec![camera_bind_group, global_instance_bind_group];
        let render_pipeline_layout =
            app_config
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[
                        &camera_bind_group_layout,
                        &global_instance_bind_group_layout,
                    ],
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
                        buffers: &[ModelVertex::desc(), LocalTransform::desc()],
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
            gscene,
            bind_groups,
            input_controller: InputController::new(),
        }
    }

    fn setup_global_instance_bind_group(
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

    fn process_input(&mut self) {
        let speed: f32 = self.gscene.get_speed();
        if self.input_controller.key_a_down {
            self.gscene.update_camera_pos(-speed, 0.0, 0.0);
        }
        if self.input_controller.key_d_down {
            self.gscene.update_camera_pos(speed, 0.0, 0.0);
        }
        if self.input_controller.key_s_down {
            self.gscene.update_camera_pos(0.0, 0.0, speed);
        }
        if self.input_controller.key_w_down {
            self.gscene.update_camera_pos(0.0, 0.0, -speed);
        }
        if self.input_controller.key_1_down {
            self.gscene.initialize_animation(0, 0, 0);
            self.input_controller.key_e_down = false;
        }
        if self.input_controller.key_2_down {
            self.gscene.initialize_animation(1, 1, 0);
            self.input_controller.key_e_down = false;
        }
        // if self.input_controller.key_q_down {
        //     self.scene
        //         .update_camera_rot(cgmath::point3(-speed, 0.0, 0.0));
        // }
        // if self.input_controller.key_e_down {
        //     self.scene
        //         .update_camera_rot(cgmath::point3(speed, 0.0, 0.0));
        // }
        self.app_config.queue.write_buffer(
            self.gscene.get_camera_buf(),
            0,
            bytemuck::cast_slice(&self.gscene.get_camera_uniform_data()),
        );
    }

    pub(super) fn update(&mut self) -> Result<(), UpdateResult> {
        self.process_input();
        let time = std::time::SystemTime::now();
        let timestamp = time.duration_since(std::time::UNIX_EPOCH).unwrap();
        if self.gscene.get_animation_frame(timestamp) {
            unsafe {
                self.app_config.queue.write_buffer(
                    self.gscene
                        .get_local_transform_buffer()
                        .as_ref()
                        .unwrap_unchecked(),
                    0,
                    bytemuck::cast_slice(self.gscene.get_local_transform_data()),
                );
            }
        }
        //let rot = cgmath::Matrix4::from_angle_y(cgmath::Deg(0.3));
        //self.gscene.update_global_transform_x(0, rot.into());
        //unsafe {
        //    self.app_config.queue.write_buffer(
        //        self.gscene
        //            .get_global_transform_buffer()
        //            .as_ref()
        //            .unwrap_unchecked(),
        //        0,
        //        bytemuck::cast_slice(&self.gscene.get_global_transform_data()),
        //    );
        //}
        Ok(())
    }

    pub(super) fn draw(&self) -> Result<(), wgpu::SurfaceError> {
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

            // set all bind groups
            for (idx, bind_group) in self.bind_groups.iter().enumerate() {
                render_pass.set_bind_group(idx as u32, Some(bind_group), &[]);
            }

            render_pass.set_pipeline(&self.render_pipeline);
            //if self.scene.draw_scene(&mut render_pass).is_err() {
            //    panic!("error");
            //}
            render_pass.set_vertex_buffer(
                0,
                self.gscene.get_vertex_buffer().as_ref().unwrap().slice(..),
            );
            if self.gscene.get_index_buffer().as_ref().unwrap().size() > 0 {
                render_pass.set_index_buffer(
                    self.gscene.get_index_buffer().as_ref().unwrap().slice(..),
                    wgpu::IndexFormat::Uint16,
                );
            }
            render_pass.set_vertex_buffer(
                1,
                self.gscene
                    .get_local_transform_buffer()
                    .as_ref()
                    .expect("local transform data should be initialized")
                    .slice(..),
            );
            render_pass.draw_scene(&self.gscene);
        }
        self.app_config
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub(super) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.app_config.resize(new_size);
    }
}
