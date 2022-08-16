pub mod texture;
pub mod vertex;

//use cgmath::prelude::*;
use vertex::{PositionColorVertex, PositionTextureVertex, Vertex};
use wgpu::util::DeviceExt;
use winit::window::Window; // Needed for the device.create_buffer_init() function

// TODO: temp
const COLORED_TRIANGLE_VERTICES: &[PositionColorVertex] = &[
    PositionColorVertex {
        position: [0.0, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    PositionColorVertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    PositionColorVertex {
        position: [0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    },
];
const COLORED_PENTAGON_VERTICES: &[PositionColorVertex] = &[
    PositionColorVertex {
        position: [-0.0868241, 0.49240386, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // A
    PositionColorVertex {
        position: [-0.49513406, 0.06958647, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // B
    PositionColorVertex {
        position: [-0.21918549, -0.44939706, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // C
    PositionColorVertex {
        position: [0.35966998, -0.3473291, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // D
    PositionColorVertex {
        position: [0.44147372, 0.2347359, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // E
];
const COLORED_PENTAGON_INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

pub struct GraphicsState {
    /// The surface to render to (usually that of the window / screen)
    pub surface: wgpu::Surface,
    /// Configuration for the rendering surface
    pub surface_config: wgpu::SurfaceConfiguration,
    /// Handle to the GPU device
    pub device: wgpu::Device,
    /// Handle to the command queue for the GPU.
    pub queue: wgpu::Queue,
    /// Current size of the rendering surface
    pub size: winit::dpi::PhysicalSize<u32>,
    /// Rendering pipeline handle
    pub render_pipeline: wgpu::RenderPipeline,

    /// Vertex buffer
    pub vertex_buffer: wgpu::Buffer,
    /// Index buffer
    pub index_buffer: wgpu::Buffer,
    /// Number of indices in the index buffer
    pub n_indices: u32,
}

impl GraphicsState {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        log::debug!("WGPU setup"); //---------------------------------------------------------------
        let wgpu_instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { wgpu_instance.create_surface(window) };
        let gpu_adapter = wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        log::debug!("Device and queue setup"); //---------------------------------------------------
        let (device, queue) = gpu_adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        log::debug!("Surface setup"); //------------------------------------------------------------
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&gpu_adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        log::debug!("Render pipeline setup"); //----------------------------------------------------
        let render_pipeline = Self::build_colored_vertex_pipeline(&device, &surface_config);
        //let render_pipeline_layout_descriptor = wgpu::PipelineLayoutDescriptor {
        //    label: Some("Render pipeline layout"),
        //    // Layouts of the bind groups that this pipeline uses.  First entry corresponds to set 0
        //    // in the shader, second entry to set 1, and so on.
        //    bind_group_layouts: &[],
        //    // Set of pus constant ranges (?) that this pipeline uses.
        //    push_constant_ranges: &[],
        //};
        //let render_pipeline_layout =
        //    device.create_pipeline_layout(&render_pipeline_layout_descriptor);
        //let shader_source = wgpu::ShaderSource::Wgsl(include_str!("graphics/shader.wgsl").into());
        //let shader_module_descriptor = wgpu::ShaderModuleDescriptor {
        //    label: Some("Shader module"),
        //    source: shader_source,
        //};
        //let shader_module = device.create_shader_module(shader_module_descriptor);
        //// VertexState describes vertex processing in a rendering pipeline
        //let vertex_state = wgpu::VertexState {
        //    module: &shader_module,
        //    entry_point: "vs_colored_triangle",
        //    // The format of any vertex buffers used with this pipeline
        //    buffers: &[PositionColorVertex::vertex_buffer_layout()],
        //};
        //// Describes the state of primitve assembly and rasterization in a render pipeline.
        //let primitive_state = wgpu::PrimitiveState {
        //    topology: wgpu::PrimitiveTopology::TriangleList,
        //    strip_index_format: None,
        //    front_face: wgpu::FrontFace::Ccw,
        //    cull_mode: Some(wgpu::Face::Back),
        //    unclipped_depth: false,
        //    polygon_mode: wgpu::PolygonMode::Fill,
        //    conservative: false,
        //};
        ////let depth_stencil_state = wgpu::DepthStencilState {
        ////    format: texture::Texture::DEPTH_FORMAT,
        ////    depth_write_enabled: true,
        ////    // Draw if new value is less than existing value
        ////    depth_compare: wgpu::CompareFunction::Less,
        ////    stencil: wgpu::StencilState::default(),
        ////    bias: wgpu::DepthBiasState::default(),
        ////};
        //let multisample_state = wgpu::MultisampleState {
        //    count: 1,
        //    mask: !0,
        //    alpha_to_coverage_enabled: false,
        //};
        //let color_target_state = wgpu::ColorTargetState {
        //    format: surface_config.format,
        //    blend: Some(wgpu::BlendState::REPLACE),
        //    // Mask that enables / disables writes to different color/alpha channels
        //    write_mask: wgpu::ColorWrites::ALL,
        //};
        //let fragment_state = wgpu::FragmentState {
        //    module: &shader_module,
        //    entry_point: "fs_colored_triangle",
        //    targets: &[Some(color_target_state)],
        //};
        //let render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
        //    label: Some("Render pipeline"),
        //    layout: Some(&render_pipeline_layout),
        //    vertex: vertex_state,
        //    primitive: primitive_state,
        //    depth_stencil: None, //Some(depth_stencil_state),
        //    multisample: multisample_state,
        //    fragment: Some(fragment_state),
        //    multiview: None,
        //};
        //let render_pipeline = device.create_render_pipeline(&render_pipeline_descriptor);

        //log::debug!("Colored triangle vertex buffer setup"); //-------------------------------------
        //let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //    label: Some("Triangle vertex buffer"),
        //    contents: bytemuck::cast_slice(COLORED_TRIANGLE_VERTICES),
        //    usage: wgpu::BufferUsages::VERTEX,
        //});
        //let n_vertices = COLORED_TRIANGLE_VERTICES.len() as u32;
        log::debug!("Colored pentagon vertex & index buffer setup"); //-------------------------------------
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pentagon vertex buffer"),
            contents: bytemuck::cast_slice(COLORED_PENTAGON_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pentagon index buffer"),
            contents: bytemuck::cast_slice(COLORED_PENTAGON_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let n_indices = COLORED_PENTAGON_INDICES.len() as u32;

        Self {
            surface,
            device,
            queue,
            surface_config,
            size,
            render_pipeline,

            vertex_buffer,
            index_buffer,
            n_indices,
        }
    }

    fn build_colored_vertex_pipeline(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> wgpu::RenderPipeline {
        let render_pipeline_layout_descriptor = wgpu::PipelineLayoutDescriptor {
            label: Some("Colored vertex pipeline"),
            // Layouts of the bind groups that this pipeline uses.  First entry corresponds to set 0
            // in the shader, second entry to set 1, and so on.
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        };
        let render_pipeline_layout =
            device.create_pipeline_layout(&render_pipeline_layout_descriptor);
        let shader_source = wgpu::ShaderSource::Wgsl(include_str!("graphics/shader.wgsl").into());
        let shader_module_descriptor = wgpu::ShaderModuleDescriptor {
            label: Some("Shader module"),
            source: shader_source,
        };
        let shader_module = device.create_shader_module(shader_module_descriptor);
        // VertexState describes vertex processing in a rendering pipeline
        let vertex_state = wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_colored_vertex",
            // The format of any vertex buffers used with this pipeline
            buffers: &[PositionColorVertex::vertex_buffer_layout()],
        };
        // Describes the state of primitve assembly and rasterization in a render pipeline.
        let primitive_state = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        };
        //let depth_stencil_state = wgpu::DepthStencilState {
        //    format: texture::Texture::DEPTH_FORMAT,
        //    depth_write_enabled: true,
        //    // Draw if new value is less than existing value
        //    depth_compare: wgpu::CompareFunction::Less,
        //    stencil: wgpu::StencilState::default(),
        //    bias: wgpu::DepthBiasState::default(),
        //};
        let multisample_state = wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };
        let color_target_state = wgpu::ColorTargetState {
            format: surface_config.format,
            blend: Some(wgpu::BlendState::REPLACE),
            // Mask that enables / disables writes to different color/alpha channels
            write_mask: wgpu::ColorWrites::ALL,
        };
        let fragment_state = wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_colored_vertex",
            targets: &[Some(color_target_state)],
        };
        let render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: Some("Render pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: vertex_state,
            primitive: primitive_state,
            depth_stencil: None, //Some(depth_stencil_state),
            multisample: multisample_state,
            fragment: Some(fragment_state),
            multiview: None,
        };
        device.create_render_pipeline(&render_pipeline_descriptor)
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Get the current SurfaceTexture that we will render to.
        let output_texture = self.surface.get_current_texture()?;
        let output_texture_view = output_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut command_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        // Render colored triangle
        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Colored triangle render pass"),
            color_attachments: &[
                // This is what @location(0) in the fragment shader output targets
                Some(wgpu::RenderPassColorAttachment {
                    view: &output_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }),
            ],
            depth_stencil_attachment: None,
        });
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        //render_pass.draw(0..self.n_vertices, 0..1);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.n_indices, 0, 0..1);

        // Drop render_pass to force the end of a mutable borrow of command_encoder that was started
        // when we called command_encoder.begin_render_pass().  This is needed so we can call
        // command_encoder.finish().
        drop(render_pass);
        self.queue.submit(std::iter::once(command_encoder.finish()));
        output_texture.present();
        Ok(())
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        log::debug!("Resizing to {:?}", new_size);
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }
}
