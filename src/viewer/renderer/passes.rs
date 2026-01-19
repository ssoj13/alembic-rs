//! Render passes used by the viewer pipeline.

use super::{Renderer, SceneMesh};

impl Renderer {
    pub fn render_shadow_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        if !self.show_shadows {
            return;
        }

        let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow_depth_pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.shadow_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        shadow_pass.set_pipeline(&self.pipelines.shadow_pipeline);
        shadow_pass.set_bind_group(0, &self.shadow_pass_bind_group, &[]);

        for mesh in self.meshes.values() {
            shadow_pass.set_bind_group(1, &mesh.model_bind_group, &[]);
            shadow_pass.set_vertex_buffer(0, mesh.mesh.vertex_buffer.slice(..));
            shadow_pass.set_index_buffer(mesh.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            shadow_pass.draw_indexed(0..mesh.mesh.index_count, 0, 0..1);
        }
    }

    pub fn render_depth_prepass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
        meshes: &[&SceneMesh],
        use_depth_prepass: bool,
    ) {
        if !use_depth_prepass {
            return;
        }

        let prepass_pipeline = if self.double_sided {
            &self.pipelines.depth_prepass_pipeline_double_sided
        } else {
            &self.pipelines.depth_prepass_pipeline
        };

        let mut prepass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("depth_prepass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        prepass.set_pipeline(prepass_pipeline);
        prepass.set_bind_group(0, &self.camera_light_bind_group, &[]);
        prepass.set_bind_group(3, &self.shadow_bind_group, &[]);
        prepass.set_bind_group(4, &self.env_map.bind_group, &[]);
        for mesh in meshes {
            prepass.set_bind_group(1, &mesh.material_bind_group, &[]);
            prepass.set_bind_group(2, &mesh.model_bind_group, &[]);
            prepass.set_vertex_buffer(0, mesh.mesh.vertex_buffer.slice(..));
            prepass.set_index_buffer(mesh.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            prepass.draw_indexed(0..mesh.mesh.index_count, 0, 0..1);
        }
    }

    pub fn render_gbuffer_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
        meshes: &[&SceneMesh],
        use_depth_prepass: bool,
    ) {

        let gbuffer = match &self.gbuffer {
            Some(gbuffer) => gbuffer,
            None => return,
        };

        let gbuffer_pipeline = if self.double_sided {
            &self.pipelines.gbuffer_pipeline_double_sided
        } else {
            &self.pipelines.gbuffer_pipeline
        };

        let gbuffer_depth_load = if use_depth_prepass {
            wgpu::LoadOp::Load
        } else {
            wgpu::LoadOp::Clear(1.0)
        };

        let mut gbuffer_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gbuffer_pass"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &gbuffer.albedo_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &gbuffer.normals_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &gbuffer.occlusion_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: gbuffer_depth_load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        gbuffer_pass.set_pipeline(gbuffer_pipeline);
        gbuffer_pass.set_bind_group(0, &self.camera_light_bind_group, &[]);
        gbuffer_pass.set_bind_group(3, &self.shadow_bind_group, &[]);
        gbuffer_pass.set_bind_group(4, &self.env_map.bind_group, &[]);

        for mesh in meshes {
            gbuffer_pass.set_bind_group(1, &mesh.material_bind_group, &[]);
            gbuffer_pass.set_bind_group(2, &mesh.model_bind_group, &[]);
            gbuffer_pass.set_vertex_buffer(0, mesh.mesh.vertex_buffer.slice(..));
            gbuffer_pass.set_index_buffer(mesh.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            gbuffer_pass.draw_indexed(0..mesh.mesh.index_count, 0, 0..1);
        }
    }

    pub fn render_ssao_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
        use_ssao: bool,
    ) {
        if !use_ssao {
            return;
        }

        let gbuffer = match &self.gbuffer {
            Some(gbuffer) => gbuffer,
            None => return,
        };
        if self.ssao_targets.is_none() {
            return;
        }

        let ssao_params = super::resources::SsaoParams {
            strength: [self.ssao_strength, 0.0, 0.0, 0.0],
        };
        self.queue.write_buffer(&self.ssao_params_buffer, 0, bytemuck::bytes_of(&ssao_params));
        self.ssao_bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssao_bind_group"),
            layout: &self.postfx.ssao_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&gbuffer.normals_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.postfx.ssao_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.ssao_params_buffer.as_entire_binding(),
                },
            ],
        }));

        let mut ssao_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ssao_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &gbuffer.occlusion_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if let Some(ssao_bind_group) = &self.ssao_bind_group {
            ssao_pass.set_pipeline(&self.postfx.ssao_pipeline);
            ssao_pass.set_bind_group(0, ssao_bind_group, &[]);
            ssao_pass.draw(0..3, 0..1);
        }
    }

    pub fn render_opaque_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
        color_target_view: &wgpu::TextureView,
        meshes: &[&SceneMesh],
        opaque_pipeline: &wgpu::RenderPipeline,
        xray_pipeline: Option<&wgpu::RenderPipeline>,
        opaque_depth_load: wgpu::LoadOp<f32>,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("opaque_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: self.background_color[0] as f64,
                        g: self.background_color[1] as f64,
                        b: self.background_color[2] as f64,
                        a: self.background_color[3] as f64,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: opaque_depth_load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if self.has_environment() && self.hdr_visible {
            render_pass.set_pipeline(&self.skybox_pipeline);
            render_pass.set_bind_group(0, &self.skybox_camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.env_map.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.skybox_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.skybox_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.skybox_index_count, 0, 0..1);
        }

        render_pass.set_bind_group(0, &self.camera_light_bind_group, &[]);
        render_pass.set_bind_group(3, &self.shadow_bind_group, &[]);
        render_pass.set_bind_group(4, &self.env_map.bind_group, &[]);

        if self.show_grid {
            if let Some(grid) = &self.grid_mesh {
                render_pass.set_pipeline(&self.pipelines.line_pipeline);
                render_pass.set_bind_group(1, &self.grid_material, &[]);
                render_pass.set_bind_group(2, &self.grid_model, &[]);
                render_pass.set_vertex_buffer(0, grid.vertex_buffer.slice(..));
                render_pass.set_index_buffer(grid.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..grid.index_count, 0, 0..1);
            }
        }

        if let Some(xray_pipeline) = xray_pipeline {
            render_pass.set_pipeline(xray_pipeline);
        } else {
            render_pass.set_pipeline(opaque_pipeline);
        }
        for mesh in meshes {
            render_pass.set_bind_group(1, &mesh.material_bind_group, &[]);
            render_pass.set_bind_group(2, &mesh.model_bind_group, &[]);
            render_pass.set_vertex_buffer(0, mesh.mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..mesh.mesh.index_count, 0, 0..1);
        }

        if !self.curves.is_empty() {
            render_pass.set_pipeline(&self.pipelines.line_pipeline);
            for curve in self.curves.values() {
                render_pass.set_bind_group(1, &curve.material_bind_group, &[]);
                render_pass.set_bind_group(2, &curve.model_bind_group, &[]);
                render_pass.set_vertex_buffer(0, curve.mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(curve.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..curve.mesh.index_count, 0, 0..1);
            }
        }

        if !self.points.is_empty() {
            render_pass.set_pipeline(&self.pipelines.point_pipeline);
            for pts in self.points.values() {
                render_pass.set_bind_group(1, &pts.material_bind_group, &[]);
                render_pass.set_bind_group(2, &pts.model_bind_group, &[]);
                render_pass.set_vertex_buffer(0, pts.vertex_buffer.slice(..));
                render_pass.draw(0..pts.vertex_count, 0..1);
            }
        }
    }

    pub fn render_composite_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {

        let (gbuffer, targets) = match (&self.gbuffer, &self.ssao_targets) {
            (Some(gbuffer), Some(targets)) => (gbuffer, targets),
            _ => return,
        };

        self.composite_bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composite_bind_group"),
            layout: &self.postfx.composite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&targets.color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&gbuffer.occlusion_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.postfx.composite_sampler),
                },
            ],
        }));

        let mut composite_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("composite_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if let Some(composite_bind_group) = &self.composite_bind_group {
            composite_pass.set_pipeline(&self.postfx.composite_pipeline);
            composite_pass.set_bind_group(0, composite_bind_group, &[]);
            composite_pass.draw(0..3, 0..1);
        }
    }
}
