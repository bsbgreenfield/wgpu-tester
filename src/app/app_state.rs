use super::{app_config::AppConfig, util};
use crate::object::{Object, ObjectTransform, ToRawMatrix};
use crate::scene::{InstanceData, ObjectInstances, Scene};
use crate::vertex::Vertex;
use cgmath::SquareMatrix;
use std::sync::Arc;
use winit::window::Window;

pub struct AppState<'a> {
    app_config: AppConfig<'a>,
    render_pipeline: wgpu::RenderPipeline,
    scene: Scene,
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
        let (ctr_bind_group_layout, ctr_bind_group) = util::create_vertex_bind_group(
            super::app::CtrUniform::new(),
            &app_config.device,
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
            app_config
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[&ctr_bind_group_layout],
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
                        // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                        // or Features::POLYGON_MODE_POINT
                        polygon_mode: wgpu::PolygonMode::Fill,
                        // Requires Features::DEPTH_CLIP_CONTROL
                        unclipped_depth: false,
                        // Requires Features::CONSERVATIVE_RASTERIZATION
                        conservative: false,
                    },
                });

        // create an object using its local coordinates
        // object contains its vertex and index buffers
        use crate::constants::*;
        let object = Object::from_vertices(VERTICES, &INDICES, &app_config.device);
        let object_2 = Object::from_vertices(VERTICES_2, &INDICES_2, &app_config.device);
        let objects = vec![object, object_2];

        let mut object_instances_list = Vec::<ObjectInstances>::with_capacity(objects.len());

        for obj_idx in 0..objects.len() {
            let object_instances: ObjectInstances = Self::get_object_instances(obj_idx);
            object_instances_list.push(object_instances);
        }

        let instance_data = InstanceData::new(object_instances_list, &app_config.device);

        let bind_groups = vec![ctr_bind_group];
        let scene = Scene {
            objects,
            instance_data,
        };
        Self {
            app_config,
            render_pipeline,
            scene,
            bind_groups,
        }
    }

    pub fn get_object_instances(obj_idx: usize) -> ObjectInstances {
        match obj_idx {
            0 => {
                let t: ObjectTransform = ObjectTransform {
                    transform_matrix: cgmath::Matrix4::identity(),
                };

                let t2: ObjectTransform = ObjectTransform {
                    transform_matrix: cgmath::Matrix4::from_translation(
                        cgmath::Vector3::<f32>::new(0.5, 0.4, 0.0),
                    ),
                };

                ObjectInstances::from_transforms(vec![t, t2], 0)
            }
            1 => {
                let tt =
                    cgmath::Matrix4::from_translation(cgmath::Vector3::<f32>::new(-0.3, 0.6, 0.0));
                let ss = cgmath::Matrix4::from_scale(0.3);
                let s = ObjectTransform {
                    transform_matrix: tt * ss,
                };

                ObjectInstances::from_transforms(vec![s], 2)
            }
            _ => panic!("no such object defined as index {}", obj_idx),
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
                self.scene.instance_data.instance_transform_buffer.slice(..),
            );
            render_pass.set_pipeline(&self.render_pipeline);

            // set all bind groups
            for (idx, bind_group) in self.bind_groups.iter().enumerate() {
                render_pass.set_bind_group(idx as u32, Some(bind_group), &[]);
            }

            // for each object, draw as many instances as there are defined in
            // self.instance data for that particular index

            use crate::object::DrawObject;
            for (idx, object) in self.scene.objects.iter().enumerate() {
                let num_instances = self.scene.instance_data.object_instances[idx].num_instances;
                let offset = self.scene.instance_data.object_instances[idx].offset_val;
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
