use std::num::{NonZeroU32, NonZeroU64};
use std::sync::Arc;

use bumpalo::Bump;
use bytemuck::{Pod, Zeroable};
use hashbrown::HashMap;
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, BlendState, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder, CompareFunction,
    DepthBiasState, DepthStencilState, Device, FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderStages, StencilState, TextureSampleType,
    TextureView, TextureViewDimension, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, ForwardRenderPassContext, RenderPassContext,
};
use crate::graphics::shader_compiler::ShaderCompiler;
use crate::graphics::{BindlessSupport, Buffer, Capabilities, GlobalContext, GroundDecalInstruction, Prepare, RenderInstruction, Texture};

const DRAWER_NAME: &str = "forward ground decal";
const INITIAL_INSTRUCTION_SIZE: usize = 64;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct InstanceData {
    /// Four world-space corners (tl, tr, bl, br); `w` is padding.
    corners: [[f32; 4]; 4],
    /// Corner UVs packed two per row: `[0] = (tl.uv, tr.uv)`, `[1] = (bl.uv, br.uv)`.
    texture_coordinates: [[f32; 4]; 2],
    color: [f32; 4],
    texture_index: i32,
    padding: [u32; 3],
}

pub(crate) struct ForwardGroundDecalDrawer {
    bindless_support: bool,
    solid_pixel_texture: Arc<Texture>,
    instance_data_buffer: Buffer<InstanceData>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
    draw_count: usize,
    instance_data: Vec<InstanceData>,
    bump: Bump,
    lookup: HashMap<u64, i32>,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::Three }, { DepthAttachmentCount::One }> for ForwardGroundDecalDrawer {
    type Context = ForwardRenderPassContext;
    type DrawData<'data> = &'data [GroundDecalInstruction];

    fn new(
        capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        shader_compiler: &ShaderCompiler,
        global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let bindless_support = capabilities.bindless_support() == BindlessSupport::Full;

        let shader_module = if bindless_support {
            shader_compiler.create_shader_module("forward", "ground_decal_bindless")
        } else {
            shader_compiler.create_shader_module("forward", "ground_decal")
        };

        let instance_data_buffer = Buffer::with_capacity(
            device,
            format!("{DRAWER_NAME} instance data"),
            BufferUsages::COPY_DST | BufferUsages::STORAGE,
            (size_of::<InstanceData>() * INITIAL_INSTRUCTION_SIZE) as _,
        );

        let bind_group_layout = if bindless_support {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(DRAWER_NAME),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(size_of::<InstanceData>() as _),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: NonZeroU32::new(capabilities.get_max_texture_binding_array_count()),
                    },
                ],
            })
        } else {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(DRAWER_NAME),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(size_of::<InstanceData>() as _),
                    },
                    count: None,
                }],
            })
        };

        let bind_group = if bindless_support {
            Self::create_bind_group_bindless(device, &bind_group_layout, &instance_data_buffer, &[global_context
                .solid_pixel_texture
                .get_texture_view()])
        } else {
            Self::create_bind_group(device, &bind_group_layout, &instance_data_buffer)
        };

        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);

        let bind_group_layouts: &[Option<&BindGroupLayout>] = if bindless_support {
            &[
                Some(pass_bind_group_layouts[0]),
                Some(pass_bind_group_layouts[1]),
                Some(&bind_group_layout),
            ]
        } else {
            &[
                Some(pass_bind_group_layouts[0]),
                Some(pass_bind_group_layouts[1]),
                Some(&bind_group_layout),
                Some(Texture::bind_group_layout(device)),
            ]
        };

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts,
            immediate_size: 0,
        });

        let color_attachment_formats = render_pass_context.color_attachment_formats();

        // Only the main color buffer is written; the two WBOIT buffers are left
        // untouched (masked empty), so the tile blends straight onto the scene.
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
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
                targets: &[
                    Some(ColorTargetState {
                        format: color_attachment_formats[0],
                        blend: Some(BlendState::ALPHA_BLENDING),
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format: color_attachment_formats[1],
                        blend: None,
                        write_mask: ColorWrites::empty(),
                    }),
                    Some(ColorTargetState {
                        format: color_attachment_formats[2],
                        blend: None,
                        write_mask: ColorWrites::empty(),
                    }),
                ],
            }),
            // Flat ground tile viewed from above — no back-face culling so winding
            // never matters.
            primitive: PrimitiveState::default(),
            multisample: MultisampleState {
                count: global_context.msaa.sample_count(),
                ..Default::default()
            },
            // Depth-test against the scene (reverse-Z: Greater = closer) so terrain
            // occludes the tile, but do not write depth — the tile is translucent
            // and entities drawn afterwards must still compose over it.
            depth_stencil: Some(DepthStencilState {
                format: render_pass_context.depth_attachment_output_format()[0],
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::Greater),
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            cache: None,
            multiview_mask: None,
        });

        Self {
            bindless_support,
            solid_pixel_texture: global_context.solid_pixel_texture.clone(),
            instance_data_buffer,
            bind_group_layout,
            bind_group,
            pipeline,
            draw_count: 0,
            instance_data: Vec::default(),
            bump: Bump::default(),
            lookup: HashMap::default(),
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        if self.draw_count == 0 {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(2, &self.bind_group, &[]);

        if self.bindless_support {
            pass.draw(0..6, 0..self.draw_count as u32);
        } else {
            // Batch contiguous same-texture runs into one instanced draw. Ground
            // effects (Land Protector's 121 tiles) all share a texture, so this is
            // usually a single draw.
            let decals = &draw_data[0..self.draw_count];
            let mut run_start = 0usize;
            let mut current_texture_id = decals[0].texture.get_id();
            pass.set_bind_group(3, decals[0].texture.get_bind_group(), &[]);

            for (index, decal) in decals.iter().enumerate() {
                if decal.texture.get_id() != current_texture_id {
                    pass.draw(0..6, run_start as u32..index as u32);
                    current_texture_id = decal.texture.get_id();
                    pass.set_bind_group(3, decal.texture.get_bind_group(), &[]);
                    run_start = index;
                }
            }

            pass.draw(0..6, run_start as u32..self.draw_count as u32);
        }
    }
}

impl Prepare for ForwardGroundDecalDrawer {
    fn prepare(&mut self, device: &Device, instructions: &RenderInstruction) {
        self.draw_count = instructions.ground_decals.len();

        if self.draw_count == 0 {
            return;
        }

        self.instance_data.clear();

        if self.bindless_support {
            self.bump.reset();
            self.lookup.clear();
            let mut texture_views = Vec::with_capacity_in(self.draw_count, &self.bump);

            for instruction in instructions.ground_decals.iter() {
                let mut texture_index = texture_views.len() as i32;
                let id = instruction.texture.get_id();

                if let Some(existing) = self.lookup.get(&id) {
                    texture_index = *existing;
                } else {
                    self.lookup.insert(id, texture_index);
                    texture_views.push(instruction.texture.get_texture_view());
                }

                self.instance_data.push(Self::instance(instruction, texture_index));
            }

            if texture_views.is_empty() {
                texture_views.push(self.solid_pixel_texture.get_texture_view());
            }

            self.instance_data_buffer.reserve(device, self.instance_data.len());
            self.bind_group = Self::create_bind_group_bindless(device, &self.bind_group_layout, &self.instance_data_buffer, &texture_views)
        } else {
            for instruction in instructions.ground_decals.iter() {
                self.instance_data.push(Self::instance(instruction, 0));
            }

            self.instance_data_buffer.reserve(device, self.instance_data.len());
            self.bind_group = Self::create_bind_group(device, &self.bind_group_layout, &self.instance_data_buffer)
        }
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        self.instance_data_buffer
            .write(device, staging_belt, command_encoder, &self.instance_data);
    }
}

impl ForwardGroundDecalDrawer {
    fn instance(instruction: &GroundDecalInstruction, texture_index: i32) -> InstanceData {
        let corner = |index: usize| {
            let point = instruction.corners[index];
            [point.x, point.y, point.z, 0.0]
        };
        let uv = instruction.texture_coordinates;

        InstanceData {
            corners: [corner(0), corner(1), corner(2), corner(3)],
            texture_coordinates: [
                [uv[0].x, uv[0].y, uv[1].x, uv[1].y],
                [uv[2].x, uv[2].y, uv[3].x, uv[3].y],
            ],
            color: instruction.color.components_linear(),
            texture_index,
            padding: Default::default(),
        }
    }

    fn create_bind_group_bindless(
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        instance_data_buffer: &Buffer<InstanceData>,
        texture_views: &[&TextureView],
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(DRAWER_NAME),
            layout: bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: instance_data_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureViewArray(texture_views),
                },
            ],
        })
    }

    fn create_bind_group(device: &Device, bind_group_layout: &BindGroupLayout, instance_data_buffer: &Buffer<InstanceData>) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(DRAWER_NAME),
            layout: bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: instance_data_buffer.as_entire_binding(),
            }],
        })
    }
}
