use super::app_config::AppConfig;
use super::util;
use crate::app::util::{create_diffuse_bgl, setup_config, setup_global_instance_bind_group};
use crate::model::materials::material::{GMaterial, MaterialDefinition};
use crate::model::materials::texture::GTexture;
use crate::model::model::{GDrawModel, LocalTransform};
use crate::model::vertex::*;
use crate::scene::camera::get_camera_color_bg;
use crate::scene::scene::GScene;
use std::sync::Arc;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
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
    pub gscene: GScene<'a>,
    bind_groups: Vec<wgpu::BindGroup>,
    joint_bind_group: wgpu::BindGroup,
    pub input_controller: InputController,
    pub materials: Vec<GMaterial>,
    depth_texture: GTexture,
}

impl<'a> AppState<'a> {
    pub async fn new(window: Arc<Window>) -> Self {
        let app_config: AppConfig = setup_config(window).await;
        let shader = app_config
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shader.wgsl").into()),
            });
        let aspect_ratio = (app_config.size.width / app_config.size.height) as f32;
        let sampler_texture_bgl = create_diffuse_bgl(&app_config);
        let mut gscene = util::get_scene(&app_config.device, aspect_ratio);
        let camera_color_bind_group_layout = gscene.get_camera_bind_group(&app_config.device);

        let (global_instance_bind_group_layout, global_instance_bind_group) =
            setup_global_instance_bind_group(&app_config, &gscene);

        let (joint_bgl, joint_bind_group) = AppState::setup_joint_bind_group(&app_config, &gscene);

        let render_pipeline_layout =
            app_config
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[
                        &camera_color_bind_group_layout,
                        &global_instance_bind_group_layout,
                        &joint_bgl,
                        &sampler_texture_bgl,
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
                                color: wgpu::BlendComponent::REPLACE,
                                alpha: wgpu::BlendComponent::REPLACE,
                            }),
                            write_mask: wgpu::ColorWrites::all(),
                        })],
                        compilation_options: Default::default(),
                    }),
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
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
        let mut materials = Vec::<GMaterial>::new();
        let mut base_color_vec: Vec<[f32; 4]> = vec![[1.0; 4]];

        println!("{:?}", gscene.material_definitions.len());
        // prepare base color buffer data
        for m_def in gscene.material_definitions.iter() {
            base_color_vec.push(m_def.base_color_factors);
        }

        // add default material to the vec in the first slot
        materials.push(GMaterial::from_material_definition_with_bgl(
            &mut MaterialDefinition::white(),
            &app_config.device,
            &sampler_texture_bgl,
        ));
        for m_def in gscene.material_definitions.iter_mut() {
            materials.push(GMaterial::from_material_definition_with_bgl(
                m_def,
                &app_config.device,
                &sampler_texture_bgl,
            ));
        }
        for material in materials.iter() {
            material.write_texture_2d(&app_config.queue);
        }

        let base_color_buffer = app_config.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("base color buffer"),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            contents: bytemuck::cast_slice(&base_color_vec),
        });

        let camera_color_bind_group = get_camera_color_bg(
            gscene.get_camera_buf(),
            &base_color_buffer,
            &camera_color_bind_group_layout,
            &app_config.device,
        );

        let depth_texture = GTexture::create_depth_texture(&app_config.device, &app_config.config);

        let bind_groups = vec![camera_color_bind_group, global_instance_bind_group];
        Self {
            materials,
            app_config,
            render_pipeline,
            gscene,
            depth_texture,
            bind_groups,
            joint_bind_group,
            input_controller: InputController::new(),
        }
    }

    fn setup_joint_bind_group(
        app_config: &AppConfig,
        scene: &GScene,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let joint_bind_group_layout =
            app_config
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("joint transform bgl"),
                    entries: &[BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
        let joint_bind_group = app_config
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &joint_bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 2,
                    resource: scene
                        .get_joint_buf()
                        .expect("should be initialized")
                        .as_entire_binding(),
                }],
                label: Some("Joint bind group"),
            });

        (joint_bind_group_layout, joint_bind_group)
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
            self.input_controller.key_1_down = false;
        }
        if self.input_controller.key_2_down {
            self.gscene.initialize_animation(0, 0, 1);
            self.input_controller.key_2_down = false;
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
                self.app_config.queue.write_buffer(
                    self.gscene.get_joint_buf_unchecked(),
                    0,
                    bytemuck::cast_slice(self.gscene.get_joint_transform_data()),
                );
            }
        }
        // let rot = cgmath::Matrix4::from_angle_y(cgmath::Deg(0.4));
        // self.gscene.update_global_transform_x(0, rot.into());
        // unsafe {
        //     self.app_config.queue.write_buffer(
        //         self.gscene
        //             .get_global_transform_buffer()
        //             .as_ref()
        //             .unwrap_unchecked(),
        //         0,
        //         bytemuck::cast_slice(&self.gscene.get_global_transform_data()),
        //     );
        // }
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // set all bind groups
            for (idx, bind_group) in self.bind_groups.iter().enumerate() {
                render_pass.set_bind_group(idx as u32, Some(bind_group), &[]);
            }
            render_pass.set_bind_group(2, &self.joint_bind_group, &[]);
            render_pass.set_bind_group(3, &self.materials[0].bind_group, &[]);

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
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw_scene(&self.gscene, &self.materials);
        }
        self.app_config
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub(super) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.app_config.resize(new_size);
        self.depth_texture =
            GTexture::create_depth_texture(&self.app_config.device, &self.app_config.config);
    }
}
