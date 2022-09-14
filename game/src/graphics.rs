pub mod camera;
pub mod texture;
pub mod vertex;

//use cgmath::prelude::*;
// Needed for image.dimensions(), but apparenly not since I no longer specify no features for the
// image package in Cargo.toml?
//use image::GenericImageView;
use cgmath::prelude::*;
use texture::Texture;
use vertex::{PositionColorVertex, PositionTextureVertex, Vertex};
use wgpu::util::DeviceExt;
use winit::window::Window; // Needed for the device.create_buffer_init() function

// TODO: temp
const COLORED_TRIANGLE_VERTICES: &[PositionColorVertex] = &[
    PositionColorVertex {
        position: [0.0, 0.5, 0.1],
        color: [1.0, 0.0, 0.0],
    },
    PositionColorVertex {
        position: [-0.5, -0.5, 0.1],
        color: [0.0, 1.0, 0.0],
    },
    PositionColorVertex {
        position: [0.5, -0.5, 0.1],
        color: [0.0, 0.0, 1.0],
    },
];
// CPC = colored pentagon center (offset to move it)
const CPC: (f32, f32, f32) = (-0.3, 0.5, 0.0);
const COLORED_PENTAGON_VERTICES: &[PositionColorVertex] = &[
    PositionColorVertex {
        position: [-0.0868241 + CPC.0, 0.49240386 + CPC.1, CPC.2],
        color: [0.5, 0.0, 0.5],
    }, // A
    PositionColorVertex {
        position: [-0.49513406 + CPC.0, 0.06958647 + CPC.1, CPC.2],
        color: [0.5, 0.0, 0.5],
    }, // B
    PositionColorVertex {
        position: [-0.21918549 + CPC.0, -0.44939706 + CPC.1, CPC.2],
        color: [0.5, 0.0, 0.5],
    }, // C
    PositionColorVertex {
        position: [0.35966998 + CPC.0, -0.3473291 + CPC.1, CPC.2],
        color: [0.5, 0.0, 0.5],
    }, // D
    PositionColorVertex {
        position: [0.44147372 + CPC.0, 0.2347359 + CPC.1, CPC.2],
        color: [0.5, 0.0, 0.5],
    }, // E
];
const COLORED_PENTAGON_INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];
// TPC = colored pentagon center (offset to move it)
const TPC: (f32, f32, f32) = (0.3, 0.5, -0.1);
const TEXTURED_PENTAGON_VERTICES: &[PositionTextureVertex] = &[
    PositionTextureVertex {
        position: [-0.0868241 + TPC.0, 0.49240386 + TPC.1, TPC.2],
        texture_coords: [0.4131759, 0.00759614],
    }, // A
    PositionTextureVertex {
        position: [-0.49513406 + TPC.0, 0.06958647 + TPC.1, TPC.2],
        texture_coords: [0.0048659444, 0.43041354],
    }, // B
    PositionTextureVertex {
        position: [-0.21918549 + TPC.0, -0.44939706 + TPC.1, TPC.2],
        texture_coords: [0.28081453, 0.949397],
    }, // C
    PositionTextureVertex {
        position: [0.35966998 + TPC.0, -0.3473291 + TPC.1, TPC.2],
        texture_coords: [0.85967, 0.84732914],
    }, // D
    PositionTextureVertex {
        position: [0.44147372 + TPC.0, 0.2347359 + TPC.1, TPC.2],
        texture_coords: [0.9414737, 0.2652641],
    }, // E
];
const TEXTURED_PENTAGON_INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

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

    /// Depth texture
    pub depth_texture: Option<Texture>,

    // Object to control the camera and construct the view/projection matrix.
    //pub camera: UprightPerspectiveCamera,
    pub camera_uniform: Matrix4Uniform,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,

    // Rendering pipeline handle
    pub colored_vertex_pipeline: wgpu::RenderPipeline,
    pub textured_vertex_pipeline: wgpu::RenderPipeline,

    // Colored triangle
    pub colored_triangle_vertex_buffer: wgpu::Buffer,
    pub n_colored_triangle_vertices: u32,

    // Colored pentagon
    pub colored_pentagon_vertex_buffer: wgpu::Buffer,
    pub colored_pentagon_index_buffer: wgpu::Buffer,
    pub n_colored_pentagon_indices: u32,

    // Textured pentagon
    pub textured_pentagon_vertex_buffer: wgpu::Buffer,
    pub textured_pentagon_index_buffer: wgpu::Buffer,
    pub n_textured_pentagon_indices: u32,
    pub bricks_texture_bind_group: wgpu::BindGroup,
}

impl GraphicsState {
    pub async fn new(window: &Window, enable_depth_buffer: bool) -> Self {
        let size = window.inner_size();

        log::debug!("WGPU setup"); //---------------------------------------------------------------
        let wgpu_instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { wgpu_instance.create_surface(window) };
        // The adapter represents the physical instance of your hardware.
        let gpu_adapter = wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        log::debug!("Device and queue setup"); //---------------------------------------------------
                                               // The device represents the logical instance that you work with, and that owns all the
                                               // resources.
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

        // TODO: should I create a SwapChain here too?  Google "wgpu SwapChain".
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&gpu_adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        log::debug!("Depth texture setup"); //------------------------------------------------------
        let depth_texture = if enable_depth_buffer {
            Some(texture::Texture::create_depth_buffer_texture(
                &device,
                &surface_config,
                Some("depth texture"),
            ))
        } else {
            None
        };

        log::debug!("Uniform buffer (for view/projection matrix) setup"); //------------------------
        let camera_uniform: Matrix4Uniform = cgmath::Matrix4::one().into();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera uniform buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        log::debug!("Camera bind group setup"); //--------------------------------------------------
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    // Put the view-projection matrix at binding 0 (location within the bind group).
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        // Indicates whether this buffer will change size or not.  Can be useful if
                        // we want to store an array of things in our uniform buffer.
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera bind group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        log::debug!("Loading texture"); //----------------------------------------------------------
        let bricks_texture_rgba = include_bytes!("../res/cube-diffuse.jpg");
        let bricks_texture_rgba = image::load_from_memory(bricks_texture_rgba).unwrap();
        let bricks_texture = Texture::from_image(
            &device,
            &queue,
            &bricks_texture_rgba,
            Some("Bricks texture"),
        )
        .unwrap();

        log::debug!("Texture bind group setup"); //-------------------------------------------------
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // Entry at binding 0 for the texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    // Entry at binding 1 for the sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
        let bricks_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&bricks_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&bricks_texture.sampler),
                },
            ],
            label: Some("bricks_texture_bind_group"),
        });

        log::debug!("Render pipeline setup"); //----------------------------------------------------
        let shader_source = wgpu::ShaderSource::Wgsl(include_str!("graphics/shader.wgsl").into());
        let shader_module_descriptor = wgpu::ShaderModuleDescriptor {
            label: Some("Shader module"),
            source: shader_source,
        };
        let shader_module = device.create_shader_module(shader_module_descriptor);
        let colored_vertex_pipeline = Self::build_colored_vertex_pipeline(
            &device,
            &surface_config,
            &shader_module,
            &camera_bind_group_layout,
            depth_texture.as_ref(),
        );
        let textured_vertex_pipeline = Self::build_textured_vertex_pipeline(
            &device,
            &surface_config,
            &shader_module,
            &texture_bind_group_layout,
            &camera_bind_group_layout,
            depth_texture.as_ref(),
        );

        log::debug!("Colored triangle vertex buffer setup"); //-------------------------------------
        let colored_triangle_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Triangle vertex buffer"),
                contents: bytemuck::cast_slice(COLORED_TRIANGLE_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let n_colored_triangle_vertices = COLORED_TRIANGLE_VERTICES.len() as u32;

        log::debug!("Colored pentagon vertex & index buffer setup"); //-----------------------------
        let colored_pentagon_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Colored pentagon vertex buffer"),
                contents: bytemuck::cast_slice(COLORED_PENTAGON_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let colored_pentagon_index_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Colored pentagon index buffer"),
                contents: bytemuck::cast_slice(COLORED_PENTAGON_INDICES),
                usage: wgpu::BufferUsages::INDEX,
            });
        let n_colored_pentagon_indices = COLORED_PENTAGON_INDICES.len() as u32;

        log::debug!("Textured pentagon vertex & index buffer setup"); //-----------------------------
        let textured_pentagon_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Textured pentagon vertex buffer"),
                contents: bytemuck::cast_slice(TEXTURED_PENTAGON_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let textured_pentagon_index_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Textured pentagon index buffer"),
                contents: bytemuck::cast_slice(TEXTURED_PENTAGON_INDICES),
                usage: wgpu::BufferUsages::INDEX,
            });
        let n_textured_pentagon_indices = TEXTURED_PENTAGON_INDICES.len() as u32;

        log::debug!("Finished graphics setup"); //--------------------------------------------------
        Self {
            surface,
            device,
            queue,
            surface_config,
            size,

            depth_texture,

            camera_uniform,
            camera_buffer,
            camera_bind_group,

            colored_vertex_pipeline,
            textured_vertex_pipeline,

            colored_triangle_vertex_buffer,
            n_colored_triangle_vertices,

            colored_pentagon_vertex_buffer,
            colored_pentagon_index_buffer,
            n_colored_pentagon_indices,

            textured_pentagon_vertex_buffer,
            textured_pentagon_index_buffer,
            n_textured_pentagon_indices,
            bricks_texture_bind_group,
        }
    }

    fn build_colored_vertex_pipeline(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        shader_module: &wgpu::ShaderModule,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        depth_texture: Option<&Texture>,
    ) -> wgpu::RenderPipeline {
        let render_pipeline_layout_descriptor = wgpu::PipelineLayoutDescriptor {
            label: Some("Colored vertex pipeline layout"),
            // Layouts of the bind groups that this pipeline uses.  First entry corresponds to set 0
            // in the shader, second entry to set 1, and so on.
            bind_group_layouts: &[camera_bind_group_layout],
            push_constant_ranges: &[],
        };
        let render_pipeline_layout =
            device.create_pipeline_layout(&render_pipeline_layout_descriptor);
        // VertexState describes vertex processing in a rendering pipeline
        let vertex_state = wgpu::VertexState {
            module: shader_module,
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
        let depth_stencil_state = depth_texture.map(|_depth_texture| wgpu::DepthStencilState {
            format: texture::Texture::DEPTH_FORMAT,
            depth_write_enabled: true,
            // Draw if new value is less than existing value
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });
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
            module: shader_module,
            entry_point: "fs_colored_vertex",
            targets: &[Some(color_target_state)],
        };
        let render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: Some("Colored vertex pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: vertex_state,
            primitive: primitive_state,
            depth_stencil: depth_stencil_state,
            multisample: multisample_state,
            fragment: Some(fragment_state),
            multiview: None,
        };
        device.create_render_pipeline(&render_pipeline_descriptor)
    }

    fn build_textured_vertex_pipeline(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        shader_module: &wgpu::ShaderModule,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        depth_texture: Option<&Texture>,
    ) -> wgpu::RenderPipeline {
        let render_pipeline_layout_descriptor = wgpu::PipelineLayoutDescriptor {
            label: Some("Textured vertex pipeline layout"),
            // Layouts of the bind groups that this pipeline uses.  First entry corresponds to set 0
            // in the shader, second entry to set 1, and so on.
            bind_group_layouts: &[texture_bind_group_layout, camera_bind_group_layout],
            push_constant_ranges: &[],
        };
        let render_pipeline_layout =
            device.create_pipeline_layout(&render_pipeline_layout_descriptor);
        // VertexState describes vertex processing in a rendering pipeline
        let vertex_state = wgpu::VertexState {
            module: shader_module,
            entry_point: "vs_textured_vertex",
            // The format of any vertex buffers used with this pipeline
            buffers: &[PositionTextureVertex::vertex_buffer_layout()],
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
        let depth_stencil_state = depth_texture.map(|_depth_texture| wgpu::DepthStencilState {
            format: texture::Texture::DEPTH_FORMAT,
            depth_write_enabled: true,
            // Draw if new value is less than existing value
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });
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
            module: shader_module,
            entry_point: "fs_textured_vertex",
            targets: &[Some(color_target_state)],
        };
        let render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: Some("Textured vertex pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: vertex_state,
            primitive: primitive_state,
            depth_stencil: depth_stencil_state,
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

        // Render things with colored vertexes -----------------------------------------------------
        let mut colored_vertex_render_pass =
            command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Colored vertex render pass"),
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
                depth_stencil_attachment: self.depth_texture.as_ref().map(|depth_texture| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_texture.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: true,
                        }),
                        stencil_ops: None,
                    }
                }),
            });
        colored_vertex_render_pass.set_pipeline(&self.colored_vertex_pipeline);
        colored_vertex_render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        // Draw colored triangle
        colored_vertex_render_pass
            .set_vertex_buffer(0, self.colored_triangle_vertex_buffer.slice(..));
        colored_vertex_render_pass.draw(0..self.n_colored_triangle_vertices, 0..1);
        // Draw colored pentagon
        colored_vertex_render_pass
            .set_vertex_buffer(0, self.colored_pentagon_vertex_buffer.slice(..));
        colored_vertex_render_pass.set_index_buffer(
            self.colored_pentagon_index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        colored_vertex_render_pass.draw_indexed(0..self.n_colored_pentagon_indices, 0, 0..1);
        // Drop render_pass to force the end of a mutable borrow of command_encoder that was started
        // when we called command_encoder.begin_render_pass().  This is needed so we can start
        // another render pass and/or call command_encoder.finish() to create the final command
        // buffer to send to the queue.
        drop(colored_vertex_render_pass);

        // Render things with textured vertexes ----------------------------------------------------
        let mut textured_vertex_render_pass =
            command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Textured vertex render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Don't clear since we already drew some stuff in the last pass.  Instead,
                        // load what has already been drawn from memory.
                        load: wgpu::LoadOp::Load,
                        // Do write new values into the depth buffer
                        store: true,
                    },
                })],
                depth_stencil_attachment: self.depth_texture.as_ref().map(|depth_texture| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_texture.view,
                        depth_ops: Some(wgpu::Operations {
                            // Don't clear since we already drew some stuff in the last pass.  Instead,
                            // load what has already been drawn from memory.
                            load: wgpu::LoadOp::Load,
                            // Do write new values into the depth buffer
                            store: true,
                        }),
                        stencil_ops: None,
                    }
                }),
            });
        textured_vertex_render_pass.set_pipeline(&self.textured_vertex_pipeline);
        textured_vertex_render_pass.set_bind_group(0, &self.bricks_texture_bind_group, &[]);
        textured_vertex_render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
        textured_vertex_render_pass
            .set_vertex_buffer(0, self.textured_pentagon_vertex_buffer.slice(..));
        textured_vertex_render_pass.set_index_buffer(
            self.textured_pentagon_index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        textured_vertex_render_pass.draw_indexed(0..self.n_textured_pentagon_indices, 0, 0..1);
        drop(textured_vertex_render_pass);

        // Create the final command buffer and submit it to the queue ------------------------------
        self.queue.submit(std::iter::once(command_encoder.finish()));
        output_texture.present();
        Ok(())
    }

    /// Update data in the GPU buffers according to the data as currently reflected in the game
    /// state.
    pub fn update(&mut self, camera_uniform: Matrix4Uniform) {
        self.camera_uniform = camera_uniform;
        // TODO: The below is the 3rd option of the 3 listed at the end of this page:
        // https://sotrh.github.io/learn-wgpu/beginner/tutorial6-uniforms/#a-controller-for-our-camera
        // I should probably look into switching it to option 1 (using a staging buffer).
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        log::debug!("Resizing to {:?}", new_size);
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
        if self.depth_texture.is_some() {
            self.depth_texture = Some(texture::Texture::create_depth_buffer_texture(
                &self.device,
                &self.surface_config,
                Some("depth texture"),
            ));
        }
    }

    /// Return the width/height ratio for the rendering surface.
    pub fn get_aspect_ratio(&self) -> f32 {
        self.surface_config.width as f32 / self.surface_config.height as f32
    }
}

/// Struct to store 4x4 matrices in a format that is compatible with being put in buffers sent to
/// the GPU.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Matrix4Uniform {
    matrix: [[f32; 4]; 4],
}

// TODO: Maybe this should be named Matrix4Gpu instead since I think it should be usable for putting
// 4d matrix data into any Gpu buffer (not just uniform buffers)?
impl Matrix4Uniform {
    fn new() -> Self {
        Self {
            matrix: cgmath::Matrix4::identity().into(),
        }
    }
}

impl From<[[f32; 4]; 4]> for Matrix4Uniform {
    fn from(matrix: [[f32; 4]; 4]) -> Self {
        Matrix4Uniform { matrix }
    }
}

impl From<cgmath::Matrix4<f32>> for Matrix4Uniform {
    fn from(matrix: cgmath::Matrix4<f32>) -> Self {
        let matrix: [[f32; 4]; 4] = matrix.into();
        matrix.into()
    }
}
