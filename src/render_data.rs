//! Everything needed to draw the grid: geometry buffers, the cell-size uniform,
//! and the render pipeline. Built once at startup. [`crate::graphics::GraphicsContext::render()`] uses this data.

use wgpu::{BufferUsages, util::DeviceExt};

use crate::{
    colormap::{ColorEncoding, make_colormap},
    config::AppConfig,
    graphics::RenderUniform,
    vertex::{CELL_VERTICES, INDICES, Instance, Vertex, get_instances},
};

/// GPU resources for drawing the grid.
pub struct RenderData {
    /// Compiled `render.wgsl` plus all the fixed drawing settings.
    pub pipeline: wgpu::RenderPipeline,
    /// The 4 corners of one square ([`CELL_VERTICES`]).
    pub vertex_buffer: wgpu::Buffer,
    /// The 6 indices that make the square from two triangles ([`INDICES`]).
    pub index_buffer: wgpu::Buffer,
    /// One centre position per cell ([`get_instances`]).
    pub instance_buffer: wgpu::Buffer,
    /// `@group(0)` in `render.wgsl`: cell size uniform (binding 0) and age colormap (binding 1).
    pub render_uniform_bind_group: wgpu::BindGroup,
}

impl RenderData {
    /// Build the render pipeline and its buffers.
    ///
    /// `game_state_render_bind_group_layout` is the grid layout from
    /// [`crate::game_data::GameData::get_render_bind_group_layout`]
    pub fn new(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        game_state_render_bind_group_layout: &wgpu::BindGroupLayout,
        config: &AppConfig,
    ) -> anyhow::Result<Self> {
        // Geometry buffers. The VERTEX / INDEX usage flags let the render pass
        // read them as vertex or index data.
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            usage: wgpu::BufferUsages::VERTEX,
            contents: bytemuck::cast_slice(CELL_VERTICES),
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cell Index buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: BufferUsages::INDEX,
        });

        // Instance buffer storing the center position of each cell in clip space. Also a VERTEX buffer; `Instance::desc()`
        // is what tells the pipeline to step through it once per square instead.
        let instances = get_instances(config.rows, config.cols, config.gap_size, config.cell_size);

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Cell size uniform buffer, plus its layout and bind group (`@group(0)`).
        let uniform = RenderUniform::new(config.cell_size);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Render Uniform Buffer"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Age → colour table (`@group(0) @binding(1)`). sRGB surfaces need linear colours.
        let encoding = if surface_config.format.is_srgb() {
            ColorEncoding::Linear
        } else {
            ColorEncoding::Srgb
        };
        let colormap = make_colormap(config.color_steps, encoding);
        let colormap_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Age Colormap Buffer"),
            contents: bytemuck::cast_slice(&colormap),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let render_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render Uniform Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // colormap: read-only, only the vertex shader needs it
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let render_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Uniform Bind Group"),
            layout: &render_uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: colormap_buffer.as_entire_binding(),
                },
            ],
        });

        // Pipeline layout: group 0 = cell size uniform, group 1 = grid state.
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&render_uniform_bind_group_layout),
                    Some(game_state_render_bind_group_layout),
                ],
                immediate_size: 0,
            });

        // One WGSL file holds both the vertex shader (`vs_main`) and the fragment shader (`fs_main`).
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Render shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/render.wgsl").into()),
        });

        // Render pipeline: every setting for how to turn vertices into pixels.
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            // Vertex stage: runs once per corner and works out where the corner lands on screen.
            // `buffers` lists the vertex buffer layouts for slot 0 (corners) and slot 1 (instances).
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(Vertex::desc()), Some(Instance::desc())],
            },
            // How to build shapes from the vertices.
            primitive: wgpu::PrimitiveState {
                // Every 3 indices form one triangle.
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                // Anticlockwise triangles face the camera. Back faces are culled (skipped).
                // Since we're in 2D I guess we want every primitive to be front facing.
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                // Fill triangles (rather than drawing only their edges or points).
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            // 2D, so no depth buffer.
            depth_stencil: None,
            // One sample per pixel: no anti-aliasing.
            // Seems to be the default for simple projects
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // Fragment stage: runs once per covered pixel and picks its colour.
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                // Write into a texture with the surface's format. Alpha blending mixes our
                // colour with the background by its alpha (cell colours are opaque).
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        Ok(Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            render_uniform_bind_group,
        })
    }
}
