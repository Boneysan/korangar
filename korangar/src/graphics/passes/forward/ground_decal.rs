use std::num::{NonZeroU32, NonZeroU64};
use std::sync::Arc;

use bumpalo::Bump;
use bytemuck::{Pod, Zeroable};
use hashbrown::HashMap;
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites,
    CommandEncoder, CompareFunction,
    DepthBiasState, DepthStencilState, Device, FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderStages, StencilState, TextureSampleType,
    TextureView, TextureViewDimension, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, ForwardRenderPassContext, RenderPassContext,
};
use crate::graphics::shader_compiler::ShaderCompiler;
use crate::graphics::{
    BindlessSupport, Buffer, Capabilities, GlobalContext, GroundDecalBlend, GroundDecalInstruction, Prepare, RenderInstruction, Texture,
};

const DRAWER_NAME: &str = "forward ground decal";
const ADDITIVE_DRAWER_NAME: &str = "forward ground decal (additive)";
const INITIAL_INSTRUCTION_SIZE: usize = 64;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct InstanceData {
    /// Four world-space corners (tl, tr, bl, br); `w` is padding.
    corners: [[f32; 4]; 4],
    /// Corner UVs packed two per row: `[0] = (tl.uv, tr.uv)`, `[1] = (bl.uv,
    /// br.uv)`.
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
    /// One pipeline per texture family; they differ only in `BlendState`.
    pipelines: [RenderPipeline; 2],
    draw_count: usize,
    /// Instruction indices in the order they were uploaded: every `Alpha` decal
    /// first, then every `Additive` one, each keeping its original relative
    /// order so overlapping translucent quads still resolve back to front.
    order: Vec<usize>,
    /// Where the additive partition starts inside `order`.
    additive_start: usize,
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
        //
        // Built twice, differing *only* in the blend state, because RO's effect
        // textures come in two families and one pipeline cannot serve both: see
        // `GroundDecalBlend`.
        let pipeline_for = |label: &str, blend: BlendState| device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(label),
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
                        blend: Some(blend),
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

        let pipelines = [
            pipeline_for(DRAWER_NAME, BlendState::ALPHA_BLENDING),
            // `src·α + dst`. The source factor is the alpha rather than One so a
            // unit still fades out: these textures are opaque everywhere, and
            // their brightness has to come from the instruction colour.
            pipeline_for(ADDITIVE_DRAWER_NAME, BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            }),
        ];

        Self {
            bindless_support,
            solid_pixel_texture: global_context.solid_pixel_texture.clone(),
            instance_data_buffer,
            bind_group_layout,
            bind_group,
            pipelines,
            draw_count: 0,
            order: Vec::default(),
            additive_start: 0,
            instance_data: Vec::default(),
            bump: Bump::default(),
            lookup: HashMap::default(),
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        if self.draw_count == 0 {
            return;
        }

        pass.set_bind_group(2, &self.bind_group, &[]);

        // Instances were uploaded partitioned by family, so each family is one
        // contiguous slice and switching pipelines costs two binds a frame.
        let families = [
            (&self.pipelines[0], 0..self.additive_start),
            (&self.pipelines[1], self.additive_start..self.draw_count),
        ];

        for (pipeline, range) in families {
            if range.is_empty() {
                continue;
            }

            pass.set_pipeline(pipeline);

            if self.bindless_support {
                pass.draw(0..6, range.start as u32..range.end as u32);
                continue;
            }

            // Batch contiguous same-texture runs into one instanced draw. Ground
            // effects (Land Protector's 121 tiles) all share a texture, so this is
            // usually a single draw.
            let decal_at = |slot: usize| &draw_data[self.order[slot]];
            let mut run_start = range.start;
            let mut current_texture_id = decal_at(range.start).texture.get_id();
            pass.set_bind_group(3, decal_at(range.start).texture.get_bind_group(), &[]);

            for slot in range.clone() {
                let decal = decal_at(slot);

                if decal.texture.get_id() != current_texture_id {
                    pass.draw(0..6, run_start as u32..slot as u32);
                    current_texture_id = decal.texture.get_id();
                    pass.set_bind_group(3, decal.texture.get_bind_group(), &[]);
                    run_start = slot;
                }
            }

            pass.draw(0..6, run_start as u32..range.end as u32);
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

        // Stable partition by family: `Alpha` first, `Additive` after. Within a
        // family the submission order is preserved, which is what keeps
        // overlapping translucent quads resolving the way they were queued.
        self.order.clear();
        self.order.extend(
            instructions
                .ground_decals
                .iter()
                .enumerate()
                .filter(|(_, instruction)| instruction.blend == GroundDecalBlend::Alpha)
                .map(|(index, _)| index),
        );
        self.additive_start = self.order.len();
        self.order.extend(
            instructions
                .ground_decals
                .iter()
                .enumerate()
                .filter(|(_, instruction)| instruction.blend == GroundDecalBlend::Additive)
                .map(|(index, _)| index),
        );

        if self.bindless_support {
            self.bump.reset();
            self.lookup.clear();
            let mut texture_views = Vec::with_capacity_in(self.draw_count, &self.bump);

            for &index in self.order.iter() {
                let instruction = &instructions.ground_decals[index];
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
            for &index in self.order.iter() {
                self.instance_data.push(Self::instance(&instructions.ground_decals[index], 0));
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
            texture_coordinates: [[uv[0].x, uv[0].y, uv[1].x, uv[1].y], [uv[2].x, uv[2].y, uv[3].x, uv[3].y]],
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
