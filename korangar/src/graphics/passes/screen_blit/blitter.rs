use bytemuck::bytes_of;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
    BufferBindingType, BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor,
    ShaderStages, TextureSampleType, TextureViewDimension, VertexState,
};

use crate::graphics::passes::screen_blit::ScreenBlitRenderPassContext;
use crate::graphics::passes::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer};
use crate::graphics::shader_compiler::ShaderCompiler;
use crate::graphics::{AttachmentTexture, Capabilities, GlobalContext};

const DRAWER_NAME: &str = "screen blit blitter";

pub(crate) struct ScreenBlitBlitterDrawer {
    pipeline: RenderPipeline,
    grade_buffer: Buffer,
    grade_bind_group: BindGroup,
}

impl Drawer<{ BindGroupCount::None }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for ScreenBlitBlitterDrawer {
    type Context = ScreenBlitRenderPassContext;
    type DrawData<'data> = &'data AttachmentTexture;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        queue: &Queue,
        shader_compiler: &ShaderCompiler,
        global_context: &GlobalContext,
        _render_pass_context: &Self::Context,
    ) -> Self {
        let surface_texture_format = global_context.surface_texture_format;

        let shader_module = match surface_texture_format.is_srgb() {
            true => shader_compiler.create_shader_module("screen_blit", "blitter_srgb"),
            false => shader_compiler.create_shader_module("screen_blit", "blitter"),
        };

        let label = format!("{DRAWER_NAME} {surface_texture_format:?}");

        let texture_bind_group_layout = AttachmentTexture::bind_group_layout(
            device,
            TextureViewDimension::D2,
            TextureSampleType::Float { filterable: true },
            false,
        );

        let grade_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("screen blit color grade"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let grade_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("screen blit color grade"),
            size: 16,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&grade_buffer, 0, bytes_of(&[1.0f32, 1.0, 1.0, 1.0]));

        let grade_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("screen blit color grade"),
            layout: &grade_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: grade_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(&label),
            bind_group_layouts: &[Some(&texture_bind_group_layout), Some(&grade_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(&label),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: surface_texture_format,
                    blend: None,
                    write_mask: ColorWrites::default(),
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });

        Self {
            pipeline,
            grade_buffer,
            grade_bind_group,
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, draw_data.get_bind_group(), &[]);
        pass.set_bind_group(1, &self.grade_bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl ScreenBlitBlitterDrawer {
    /// xyz multiply the linear color. w is saturation. Written before the blit
    /// pass.
    pub(crate) fn prepare(&self, queue: &Queue, grade: [f32; 4]) {
        queue.write_buffer(&self.grade_buffer, 0, bytes_of(&grade));
    }
}
