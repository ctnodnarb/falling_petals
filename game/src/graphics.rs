pub mod camera;
pub mod gpu_types;
pub mod texture;

// Needed for image.dimensions(), but apparenly not since I no longer specify no features for the
// image package in Cargo.toml?
//use image::GenericImageView;
use camera::Camera;
use cgmath::prelude::*;
use gpu_types::{
    PositionColorVertex, PositionTextureIndexVertex, PositionTextureVertex, VertexBufferEntry,
};
use noise::{NoiseFn, Seedable};
use texture::Texture;
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
const CPC: (f32, f32, f32) = (-0.3, 0.5, -0.1);
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
// TPC = textured pentagon center (offset to move it)
const TPC: (f32, f32, f32) = (0.0, 0.0, 0.0); //(0.3, 0.5, 0.2);
const TEXTURED_PENTAGON_VERTICES: &[PositionTextureIndexVertex] = &[
    PositionTextureIndexVertex {
        position: [-0.0868241 + TPC.0, 0.49240386 + TPC.1, TPC.2],
        texture_coords: [0.4131759, 0.00759614],
        index: 0,
    }, // A
    PositionTextureIndexVertex {
        position: [-0.49513406 + TPC.0, 0.06958647 + TPC.1, TPC.2],
        texture_coords: [0.0048659444, 0.43041354],
        index: 0,
    }, // B
    PositionTextureIndexVertex {
        position: [-0.21918549 + TPC.0, -0.44939706 + TPC.1, TPC.2],
        texture_coords: [0.28081453, 0.949397],
        index: 1,
    }, // C
    PositionTextureIndexVertex {
        position: [0.35966998 + TPC.0, -0.3473291 + TPC.1, TPC.2],
        texture_coords: [0.85967, 0.84732914],
        index: 1,
    }, // D
    PositionTextureIndexVertex {
        position: [0.44147372 + TPC.0, 0.2347359 + TPC.1, TPC.2],
        texture_coords: [0.9414737, 0.2652641],
        index: 1,
    }, // E
];
const TEXTURED_PENTAGON_INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

pub struct GraphicsState {
    // TODO: Go through all the members of this struct and determine if they all actually need to be
    // public.  Make them private where appropriate.
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
    pub camera_uniform: gpu_types::Matrix4,
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
    pub texture_bind_group: wgpu::BindGroup,

    // Instanced objects
    pub instance_pose_data: Vec<gpu_types::Matrix4>,
    pub instance_pose_buffer: wgpu::Buffer,
}

impl GraphicsState {
    pub async fn new(
        window: &Window,
        petal_poses: &[crate::game::Pose],
        enable_depth_buffer: bool,
    ) -> Self {
        let size = window.inner_size();

        // -----------------------------------------------------------------------------------------
        log::debug!("WGPU setup");
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

        // -----------------------------------------------------------------------------------------
        log::debug!("Device and queue setup");

        // The device represents the logical instance that you work with, and that owns all the
        // resources.
        let (device, queue) = gpu_adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::TEXTURE_BINDING_ARRAY | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                    //features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        // -----------------------------------------------------------------------------------------
        log::debug!("Surface setup");

        // TODO: should I create a SwapChain here too?  Google "wgpu SwapChain".
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&gpu_adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        // -----------------------------------------------------------------------------------------
        log::debug!("Depth texture setup");
        let depth_texture = if enable_depth_buffer {
            Some(texture::Texture::create_depth_buffer_texture(
                &device,
                &surface_config,
                Some("depth texture"),
            ))
        } else {
            None
        };

        // -----------------------------------------------------------------------------------------
        log::debug!("Uniform buffer (for view/projection matrix) setup");
        let camera_uniform: gpu_types::Matrix4 = cgmath::Matrix4::one().into();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera uniform buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // -----------------------------------------------------------------------------------------
        log::debug!("Camera bind group setup");
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

        // -----------------------------------------------------------------------------------------
        log::debug!("Loading textures");
        //const NOISE_COORD_SCALE: u32 = 5;
        //const TEXTURE_DIMENSION: u32 = 1024;
        //let r_generator = noise::Fbm::default().set_seed(rand::random());
        //let g_generator = noise::SuperSimplex::default().set_seed(rand::random());
        //let b_generator = noise::RidgedMulti::default().set_seed(rand::random());
        //let a_generator = noise::Worley::default().set_seed(rand::random());
        //let mut procedural_texture_rgba = image::Rgba32FImage::from_fn(
        //    TEXTURE_DIMENSION,
        //    TEXTURE_DIMENSION,
        //    |x, y| -> image::Rgba<f32> {
        //        image::Rgba::<f32>([
        //            (r_generator.get([
        //                f64::from(NOISE_COORD_SCALE * x) / f64::from(TEXTURE_DIMENSION),
        //                f64::from(NOISE_COORD_SCALE * y) / f64::from(TEXTURE_DIMENSION),
        //            ]) * 0.5
        //                + 0.5)
        //                .clamp(0.0, 1.0) as f32,
        //            (g_generator.get([
        //                f64::from(NOISE_COORD_SCALE * x) / f64::from(TEXTURE_DIMENSION),
        //                f64::from(NOISE_COORD_SCALE * y) / f64::from(TEXTURE_DIMENSION),
        //            ]) * 0.5
        //                + 0.5)
        //                .clamp(0.0, 1.0) as f32,
        //            (b_generator.get([
        //                f64::from(NOISE_COORD_SCALE * x) / f64::from(TEXTURE_DIMENSION),
        //                f64::from(NOISE_COORD_SCALE * y) / f64::from(TEXTURE_DIMENSION),
        //            ]) * 0.5
        //                + 0.5)
        //                .clamp(0.0, 1.0) as f32,
        //            (a_generator.get([
        //                f64::from(NOISE_COORD_SCALE * x) / f64::from(TEXTURE_DIMENSION),
        //                f64::from(NOISE_COORD_SCALE * y) / f64::from(TEXTURE_DIMENSION),
        //            ]) * 0.5
        //                + 0.5)
        //                .clamp(0.0, 1.0) as f32,
        //        ])
        //    },
        //);
        //// Pre-multiply RGB values by their alpha values (since we're using PREMULTIPLIED_ALPHA
        //// mode).
        //for pixel in procedural_texture_rgba.pixels_mut() {
        //    pixel[0] *= pixel[3];
        //    pixel[1] *= pixel[3];
        //    pixel[2] *= pixel[3];
        //}
        //let procedural_texture = Texture::from_image(
        //    &device,
        //    &queue,
        //    &procedural_texture_rgba.into(),
        //    Some("Bricks texture"),
        //)
        //.unwrap();

        //let bricks_texture_rgba = include_bytes!("../res/cube-diffuse.jpg");
        //let bricks_texture_rgba = image::load_from_memory(bricks_texture_rgba).unwrap();
        //let bricks_texture = Texture::from_image(
        //    &device,
        //    &queue,
        //    &bricks_texture_rgba,
        //    Some("Bricks texture"),
        //)
        //.unwrap();

        // Petal textures
        let mut petal_texture_images = vec![
            image::load_from_memory(include_bytes!("../res/pink_petals.png")).unwrap(),
            image::load_from_memory(include_bytes!("../res/cube-diffuse.jpg")).unwrap(),
        ];
        // TODO: There's probably a better way to do this, e.g. defining the list of textures to be
        // loaded up at the GameState level so we know the correct number there, and then passing
        // their paths in to the GraphicsState initialization.  Then I wouldn't need the
        // N_PETAL_VARIANTS constant at all and instead could just use the length of the array.
        assert_eq!(
            petal_texture_images.len(),
            crate::game::N_PETAL_VARIANTS,
            "The N_PETAL_VARIANTS constant must match the number of petal textures loaded",
        );

        // Pre-mulitpy alpha values since we're using PREMULTIPLIED_ALPHA_BLENDING mode.
        for petal_texture_image in &mut petal_texture_images {
            match petal_texture_image {
                image::DynamicImage::ImageRgba32F(image_32f) => {
                    for pixel in image_32f.pixels_mut() {
                        pixel[0] *= pixel[3];
                        pixel[1] *= pixel[3];
                        pixel[2] *= pixel[3];
                    }
                }
                image::DynamicImage::ImageRgba8(image_8u) => {
                    for pixel in image_8u.pixels_mut() {
                        pixel[0] = (u16::from(pixel[0]) * u16::from(pixel[3]) / 255) as u8;
                        pixel[1] = (u16::from(pixel[1]) * u16::from(pixel[3]) / 255) as u8;
                        pixel[2] = (u16::from(pixel[2]) * u16::from(pixel[3]) / 255) as u8;
                    }
                }
                // No alpha channel, so no pre-multiplication necessary.  Everything is opaque.
                image::DynamicImage::ImageRgb8(_) | image::DynamicImage::ImageRgb32F(_) => {}
                image => log::error!(
                    "Unhandled image format for alpha premultiplication: {:?}",
                    image,
                ),
            }
        }
        let petal_textures: Vec<Texture> = petal_texture_images
            .iter()
            .enumerate()
            .map(|(idx, petal_texture_image)| {
                Texture::from_image(
                    &device,
                    &queue,
                    petal_texture_image,
                    Some(format!("Petal texture {}", idx).as_str()),
                )
                .unwrap()
            })
            .collect();

        // -----------------------------------------------------------------------------------------
        log::debug!("Texture bind group setup");
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // Entry at binding 0 for the texture array
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: core::num::NonZeroU32::new(petal_textures.len() as u32),
                    },
                    // Entry at binding 1 for the sampler array
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: core::num::NonZeroU32::new(petal_textures.len() as u32),
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(
                        &petal_textures
                            .iter()
                            .map(|tex| &tex.view)
                            .collect::<Vec<_>>(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::SamplerArray(
                        &petal_textures
                            .iter()
                            .map(|tex| &tex.sampler)
                            .collect::<Vec<_>>(),
                    ),
                },
            ],
            label: Some("texture_bind_group"),
        });

        // -----------------------------------------------------------------------------------------
        log::debug!("Instance setup");
        let instance_pose_data = petal_poses
            .iter()
            .map(gpu_types::Matrix4::from)
            .collect::<Vec<_>>();
        let instance_pose_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance pose buffer"),
            contents: bytemuck::cast_slice(&instance_pose_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // -----------------------------------------------------------------------------------------
        log::debug!("Render pipeline setup");
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

        // -----------------------------------------------------------------------------------------
        log::debug!("Colored triangle vertex buffer setup");
        let colored_triangle_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Triangle vertex buffer"),
                contents: bytemuck::cast_slice(COLORED_TRIANGLE_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let n_colored_triangle_vertices = COLORED_TRIANGLE_VERTICES.len() as u32;

        // -----------------------------------------------------------------------------------------
        log::debug!("Colored pentagon vertex & index buffer setup");
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

        // -----------------------------------------------------------------------------------------
        log::debug!("Textured pentagon vertex & index buffer setup");
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

        // -----------------------------------------------------------------------------------------
        log::debug!("Finished graphics setup");
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
            texture_bind_group,

            instance_pose_data,
            instance_pose_buffer,
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
            // Better alpha blending mode, but requires the color channels to be pre-multiplied by
            // the alpha channel.
            //blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            //blend: Some(wgpu::BlendState::ALPHA_BLENDING), // Enable alpha blending
            blend: Some(wgpu::BlendState::REPLACE), // No alpha blending
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
            buffers: &[
                // TODO:  I realized that putting texture indices inside the vertex buffer is not
                // going to work since it uses the same vertices for every instance.  I need a way
                // to include it with the instance data (currently just a Matrix4 constructed from
                // a Pose struct for each instance).
                PositionTextureIndexVertex::vertex_buffer_layout(),
                gpu_types::Matrix4::vertex_buffer_layout(),
            ],
        };
        // Describes the state of primitve assembly and rasterization in a render pipeline.
        let primitive_state = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None, // Disable culling for petal rendering
            //cull_mode: Some(wgpu::Face::Back),
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
            // Better alpha blending mode, but requires the color channels to be pre-multiplied by
            // the alpha channel.
            blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            //blend: Some(wgpu::BlendState::ALPHA_BLENDING), // Enable alpha blending
            //blend: Some(wgpu::BlendState::REPLACE), // No alpha blending
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
                                r: 0.0, //0.1,
                                g: 0.0, //0.2,
                                b: 0.0, //0.3,
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
        textured_vertex_render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        textured_vertex_render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
        textured_vertex_render_pass
            .set_vertex_buffer(0, self.textured_pentagon_vertex_buffer.slice(..));
        //textured_vertex_render_pass.set_index_buffer(
        //    self.textured_pentagon_index_buffer.slice(..),
        //    wgpu::IndexFormat::Uint16,
        //);
        //textured_vertex_render_pass.draw_indexed(0..self.n_textured_pentagon_indices, 0, 0..1);
        textured_vertex_render_pass.set_vertex_buffer(1, self.instance_pose_buffer.slice(..));
        textured_vertex_render_pass.set_index_buffer(
            self.textured_pentagon_index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        textured_vertex_render_pass.draw_indexed(
            0..self.n_textured_pentagon_indices,
            0,
            0..self.instance_pose_data.len() as _,
        );
        drop(textured_vertex_render_pass);

        // Create the final command buffer and submit it to the queue ------------------------------
        self.queue.submit(std::iter::once(command_encoder.finish()));
        output_texture.present();
        Ok(())
    }

    /// Update data in the GPU buffers according to the data as currently reflected in the game
    /// state.
    pub fn update(
        &mut self,
        camera: &camera::UprightPerspectiveCamera,
        petal_poses: &[crate::game::Pose],
    ) {
        self.camera_uniform = camera.get_view_projection_matrix().into();
        // TODO: The below is the 3rd option of the 3 listed at the end of this page:
        // https://sotrh.github.io/learn-wgpu/beginner/tutorial6-uniforms/#a-controller-for-our-camera
        // I should probably look into switching it to option 1 (using a staging buffer).
        // After having read more later, it sounds like write_buffer is actually quite performant,
        // and using a staging buffer would probably only be slightly better performance-wise
        // (see e.g. https://github.com/gfx-rs/wgpu/discussions/1438).  It looks like
        // wgpu::util::StagingBelt is probably the correct object / way to do a staging buffer in
        // wgpu.
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        // Update the instance buffer with the current instance poses.
        for (pose_matrix, pose) in self.instance_pose_data.iter_mut().zip(petal_poses.iter()) {
            //let mat: gpu_types::Matrix4 = pose.into();
            //pose_matrix.matrix = mat.matrix;
            pose_matrix.matrix = gpu_types::Matrix4::from(pose).matrix;
        }
        //// Alternate method of updating the instance buffer with current instance poses, but
        //// involved constructing a new Vec object (which I'm assuming is less efficient?).
        //self.instance_pose_data = petal_poses
        //    .iter()
        //    .map(gpu_types::Matrix4::from)
        //    .collect::<Vec<_>>();

        self.queue.write_buffer(
            &self.instance_pose_buffer,
            0,
            bytemuck::cast_slice(&self.instance_pose_data),
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
