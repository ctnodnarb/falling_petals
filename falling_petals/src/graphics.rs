pub mod camera;
pub mod gpu_types;
pub mod texture;

use crate::configuration::{FallingPetalsConfig, VideoExportConfig};
use crate::state::PetalState;
use camera::Camera;
use cgmath::prelude::*;
use gpu_types::{PositionTextureVertex, VertexBufferEntry};
use std::io::Write;
use texture::Texture;
use wgpu::util::DeviceExt;
use winit::window::Window; // Needed for the device.create_buffer_init() function

/// Cast a sized type to a read-only &[u8] byte array.  Note that the sized type probably should NOT
/// contain any internal indirection / pointers, as this function is generally meant to be used to
/// create a buffer-compatible view of data that needs to be sent somewhere (like the GPU) where
/// those pointer values would be invalid.
unsafe fn sized_type_as_u8_slice<T: Sized>(item: &T) -> &[u8] {
    ::std::slice::from_raw_parts((item as *const T) as *const u8, ::std::mem::size_of::<T>())
}

/// Cast a Vec containing items of a sized type to a read-only &[u8] byte array.  Note that the
/// sized type probably should NOT contain any internal indirection / pointers, as this function is
/// generally meant to be used to create a buffer-compatible view of data that needs to be sent
/// somewhere (like the GPU) where those pointer values would be invalid.
unsafe fn vec_as_u8_slice<T: Sized>(array: &Vec<T>) -> &[u8] {
    ::std::slice::from_raw_parts(
        array.as_ptr() as *const u8,
        array.len() * ::std::mem::size_of::<T>(),
    )
}

/// Index list that defines the tesselation of the 3x3 grid of vertices used to render each petal.
/// In this case, all triangles share the center vertex (it creates fan of 8 triangles around the
/// center vertex).
const TEXTURED_SQUARE_INDICES: &[u16; 24] = &[
    0, 4, 1, //
    1, 4, 2, //
    2, 4, 5, //
    5, 4, 8, //
    8, 4, 7, //
    7, 4, 6, //
    6, 4, 3, //
    3, 4, 0, //
];

enum RenderTarget<'a> {
    Screen(&'a wgpu::TextureView),
    Video,
}

pub struct GraphicsState {
    // GPU handles ---------------------------------------------------------------------------------
    /// Handle to the GPU device
    pub device: wgpu::Device,
    /// Handle to the command queue for the GPU.
    pub queue: wgpu::Queue,

    // Rendering to screen -------------------------------------------------------------------------
    /// The surface to render to (usually that of the window / screen)
    pub surface: wgpu::Surface,
    /// Configuration for the rendering surface
    pub surface_config: wgpu::SurfaceConfiguration,
    /// Current size of the rendering surface
    pub size: winit::dpi::PhysicalSize<u32>,
    /// Depth texture
    pub depth_texture: Texture,
    // Rendering pipeline handle for rendering to the screen
    pub render_pipeline: wgpu::RenderPipeline,

    // Rendering to video --------------------------------------------------------------------------
    pub video_export_state: Option<VideoExportState>,

    // Instance data -------------------------------------------------------------------------------
    /// For each petal, gpu compatible data specifying its location/orientation/scale
    pub petal_pose_data: Vec<gpu_types::Matrix4>,
    /// Handle to buffer for the data specifying each petal's location/orientation/scale
    pub petal_pose_buffer: wgpu::Buffer,
    /// For each petal, the index into which variant it is
    pub petal_variant_index_data: Vec<u32>,
    /// Handle to buffer containing a variant index for each petal
    pub petal_variant_index_buffer: wgpu::Buffer,
    /// For each petal variant, data specifying which portion of which texture to use for that
    /// variant
    pub petal_variant_data: Vec<gpu_types::PetalVariant>,
    /// Handle to buffer containing the texture slice info for each petal variant
    pub petal_variant_buffer: wgpu::Buffer,

    // Other ---------------------------------------------------------------------------------------

    // Objects to control the camera and construct the view/projection matrix.
    pub camera_uniform: gpu_types::Matrix4,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,

    // Textured square used to draw petals
    pub textured_square_vertices: [PositionTextureVertex; 9],
    pub textured_square_vertex_buffer: wgpu::Buffer,
    pub textured_square_index_buffer: wgpu::Buffer,
    pub n_textured_square_indices: u32,
    pub texture_bind_group: wgpu::BindGroup,
}

pub struct VideoExportState {
    /// Config values for video export (if doing video export)
    pub video_config: VideoExportConfig,
    /// Texture to render each video frame to
    pub video_texture: Texture,
    /// Buffer to transfer video output data from GPU to CPU
    pub video_buffer: wgpu::Buffer,
    /// Depth buffer for redering to video
    pub video_depth_texture: Texture,
    /// Rendering pipeline handle for rendering to video
    pub video_render_pipeline: wgpu::RenderPipeline,
    /// JoinHandle for the video encoding thread
    pub video_thread_handle: Option<std::thread::JoinHandle<std::io::Result<()>>>,
    /// Transmitter to send frames to the video encoding thread
    pub video_thread_tx: Option<std::sync::mpsc::SyncSender<Vec<u8>>>,
}

impl GraphicsState {
    pub fn new(
        window: &Window,
        petal_texture_image_paths: &[String],
        petal_variants: Vec<gpu_types::PetalVariant>,
        petal_states: &[PetalState],
        petal_config: &FallingPetalsConfig,
        video_config: VideoExportConfig,
    ) -> Self {
        let size = window.inner_size();

        // -----------------------------------------------------------------------------------------
        log::debug!("WGPU setup");
        let wgpu_instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        //log::debug!("wgpu report:\n{:?}", wgpu_instance.generate_report());
        let surface = unsafe { wgpu_instance.create_surface(window) }.unwrap();
        // The adapter represents the physical instance of your hardware.
        let gpu_adapter =
            pollster::block_on(wgpu_instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }))
            .unwrap();
        //log::debug!("Adapter features:\n{:?}", gpu_adapter.features());
        //log::debug!("Adapter limits:\n{:?}", gpu_adapter.limits());

        // -----------------------------------------------------------------------------------------
        log::debug!("Device and queue setup");

        // The device represents the logical instance that you work with, and that owns all the
        // resources.
        let limits = wgpu::Limits {
            // Request a larger max uniform buffer size so we can have more than ~4000 petals.  It
            // defaults to 65536 (2^16), or 64KB.  Since each petal variant index (a single u32)
            // ends up using up 16 bytes (the minimum uniform buffer array stride), we can only fit
            // indices fo 4096 petals in a uniform buffer with the default limits.
            // ACTUALLY, looks like requesting more than 65536 causes it to fail to obtain a device.
            // Looks like that is the hard limit for my GPU.
            //max_uniform_buffer_binding_size: 4 * 65536,
            ..wgpu::Limits::default()
        };
        let (device, queue) = pollster::block_on(gpu_adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::TEXTURE_BINDING_ARRAY
                    | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
                    | wgpu::Features::BUFFER_BINDING_ARRAY,
                limits,
                label: None,
            },
            None,
        ))
        .unwrap();
        //log::debug!("Device features:\n{:?}", device.features());
        //log::debug!("Device limits:\n{:?}", device.limits());

        // -----------------------------------------------------------------------------------------
        log::debug!("Surface setup");

        // TODO: should I create a SwapChain here too?  Google "wgpu SwapChain".
        let surface_capabilities = surface.get_capabilities(&gpu_adapter);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: surface_capabilities.formats.clone(),
        };

        // -----------------------------------------------------------------------------------------
        log::debug!("Depth texture setup");
        let depth_texture = texture::Texture::create_depth_buffer_texture(
            &device,
            surface_config.width,
            surface_config.height,
            Some("depth texture"),
        );
        let video_depth_texture = texture::Texture::create_depth_buffer_texture(
            &device,
            video_config.width,
            video_config.height,
            Some("video depth texture"),
        );

        // -----------------------------------------------------------------------------------------
        log::debug!("Uniform buffer (for view/projection matrix) setup");
        let camera_uniform: gpu_types::Matrix4 = cgmath::Matrix4::one().into();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera uniform buffer"),
            contents: unsafe { sized_type_as_u8_slice(&camera_uniform) },
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
        // Petal textures
        let mut petal_texture_images = petal_texture_image_paths
            .iter()
            .map(|image_path| image::open(image_path).expect(image_path))
            .collect::<Vec<_>>();
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
                    Some(format!("Petal texture {idx}").as_str()),
                )
                .unwrap()
            })
            .collect();

        // -----------------------------------------------------------------------------------------
        log::debug!("Instance setup");
        let petal_pose_data = petal_states
            .iter()
            .map(|state| gpu_types::Matrix4::from(&state.pose))
            .collect::<Vec<_>>();
        let petal_pose_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance pose buffer"),
            contents: unsafe { vec_as_u8_slice(&petal_pose_data) },
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let mut petal_variant_index_data = petal_states
            .iter()
            .map(|state| state.variant_index)
            .collect::<Vec<_>>();
        // Pad the buffer out so that its size is a multiple of 16 bytes, and is thus compatible
        // with the 16 byte alignment requirement for uniform buffers.  The extra zeros tacked on
        // the end as padding will get transferred to the GPU each frame, but will never be read.
        while petal_variant_index_data.len() % 4 != 0 {
            log::debug!("Adding a byte of padding onto petal_variant_index_data (for alignment).");
            petal_variant_index_data.push(0);
        }
        let petal_variant_index_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Petal variant index buffer"),
                contents: unsafe { vec_as_u8_slice(&petal_variant_index_data) },
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let petal_variant_data = petal_variants;
        let petal_variant_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Petal variant buffer"),
            contents: unsafe { vec_as_u8_slice(&petal_variant_data) },
            // COPY_DST is not needed here because this buffer never gets written to again after its
            // first initialization.
            usage: wgpu::BufferUsages::UNIFORM,
        });

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
                    // Entry at binding 2 for the petal variant info
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Entry at binding 3 for the variant indices for each petal
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: petal_variant_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: petal_variant_index_buffer.as_entire_binding(),
                },
            ],
            label: Some("texture_bind_group"),
        });

        // -----------------------------------------------------------------------------------------
        log::debug!("Render pipeline setup");
        let shader_source_str = include_str!("graphics/shader.wgsl")
            .replace("N_PETAL_VARIANTS", &petal_variant_data.len().to_string())
            .replace(
                "N_VEC4_OF_PETAL_INDICES",
                &((petal_states.len() + 3) / 4).to_string(),
            );
        //log::debug!("Processed shader source:\n{}", &shader_source_str);
        let shader_source = wgpu::ShaderSource::Wgsl(shader_source_str.into());
        let shader_module_descriptor = wgpu::ShaderModuleDescriptor {
            label: Some("Shader module"),
            source: shader_source,
        };
        let shader_module = device.create_shader_module(shader_module_descriptor);
        let render_pipeline = Self::build_render_pipeline(
            &device,
            surface_config.format,
            &shader_module,
            &texture_bind_group_layout,
            &camera_bind_group_layout,
        );
        let video_render_pipeline = Self::build_render_pipeline(
            &device,
            video_config.texture_format,
            &shader_module,
            &texture_bind_group_layout,
            &camera_bind_group_layout,
        );

        // --- Set up vertices used to render each petal -------------------------------------------
        // Apply the petal bend offsets (scaled by their multiplier) to the z coordinates.
        let offsets = petal_config
            .petal_bend_vertex_offsets
            .map(|offset| offset * petal_config.petal_bend_vertex_offset_multiplier);
        let textured_square_vertices = [
            PositionTextureVertex {
                // 0: 0,0 -- Upper left corner
                position: [-1.0, 1.0, offsets[0]],
                texture_coords: [0.0, 0.0],
            },
            PositionTextureVertex {
                // 1: 1,0 -- Top middle
                position: [0.0, 1.0, offsets[1]],
                texture_coords: [0.5, 0.0],
            },
            PositionTextureVertex {
                // 2: 2,0 -- Upper right corner
                position: [1.0, 1.0, offsets[2]],
                texture_coords: [1.0, 0.0],
            },
            PositionTextureVertex {
                // 3: 0,1 -- Left middle
                position: [-1.0, 0.0, offsets[3]],
                texture_coords: [0.0, 0.5],
            },
            PositionTextureVertex {
                // 4: 1,1 -- Middle middle
                position: [0.0, 0.0, offsets[4]],
                texture_coords: [0.5, 0.5],
            },
            PositionTextureVertex {
                // 5: 0,1 -- Middle middle
                position: [1.0, 0.0, offsets[5]],
                texture_coords: [1.0, 0.5],
            },
            PositionTextureVertex {
                // 6: 0,2 -- Lower left corner
                position: [-1.0, -1.0, offsets[6]],
                texture_coords: [0.0, 1.0],
            },
            PositionTextureVertex {
                // 7: 0,1 -- Middle middle
                position: [0.0, -1.0, offsets[7]],
                texture_coords: [0.5, 1.0],
            },
            PositionTextureVertex {
                // 8: 2,2 -- Lower right corner
                position: [1.0, -1.0, offsets[8]],
                texture_coords: [1.0, 1.0],
            },
        ];

        // -----------------------------------------------------------------------------------------
        log::debug!("Textured square vertex & index buffer setup");
        let textured_square_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Textured pentagon vertex buffer"),
                contents: unsafe { sized_type_as_u8_slice(&textured_square_vertices) },
                usage: wgpu::BufferUsages::VERTEX,
            });
        let textured_square_index_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Textured pentagon index buffer"),
                contents: unsafe { sized_type_as_u8_slice(TEXTURED_SQUARE_INDICES) },
                usage: wgpu::BufferUsages::INDEX,
            });
        let n_textured_square_indices = TEXTURED_SQUARE_INDICES.len() as u32;

        // -----------------------------------------------------------------------------------------
        let video_export_state = match video_config.export_enabled {
            false => None,
            true => {
                log::debug!("Set up video output objects");
                let video_texture_descriptor = wgpu::TextureDescriptor {
                    label: Some("video output texture"),
                    size: wgpu::Extent3d {
                        width: video_config.width,
                        height: video_config.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: video_config.texture_format,
                    // COPY_SRC so we can copy the texture contents to a buffer (video_output_buffer),
                    // RENDER_ATTACHMENT so that we can attach the texture to a render pass so it can be
                    // rendered to.
                    usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &surface_capabilities.formats,
                };
                let video_texture = Texture::from_descriptor(&device, &video_texture_descriptor);
                //device.create_texture(&video_texture_descriptor);
                let video_buffer_descriptor = wgpu::BufferDescriptor {
                    label: Some("video output buffer"),
                    size: video_config.frame_size,
                    // COPY_DST so we can copy data into the buffer, MAP_READ so that we can read the
                    // contents of the buffer from the CPU side.
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                };
                let video_buffer = device.create_buffer(&video_buffer_descriptor);

                // -----------------------------------------------------------------------------------------
                log::debug!("Spawn video coding thread");
                // I tried using a std::sync::mpsc::channel() here before, but it seems to accumulate more
                // and more memory for everything I send over it without bound until my RAM fills up and
                // things crash. Maybe this is because frames are getting rendered faster than ffmpeg can
                // encode them?  I'm not sure.  But switching to use a bounded channel
                // (std::sync::mpsc::sync_channel(bound)) fixed the problem so that now my RAM usage remains
                // stable.
                let (video_thread_tx, video_thread_rx) = std::sync::mpsc::sync_channel(1);
                let output_file_clone = video_config.output_file.clone();
                let video_thread_handle = std::thread::spawn(move || {
                    video_thread_fn(
                        video_thread_rx,
                        output_file_clone,
                        video_config.width,
                        video_config.height,
                        video_config.frame_rate,
                    )
                });
                Some(VideoExportState {
                    video_config,
                    video_texture,
                    video_buffer,
                    video_depth_texture,
                    video_render_pipeline,
                    video_thread_handle: Some(video_thread_handle),
                    video_thread_tx: Some(video_thread_tx),
                })
            }
        };

        // -----------------------------------------------------------------------------------------
        log::debug!("Finished graphics setup");
        Self {
            device,
            queue,
            surface,
            surface_config,
            size,

            depth_texture,
            render_pipeline,

            video_export_state,

            petal_pose_data,
            petal_pose_buffer,
            petal_variant_index_data,
            petal_variant_index_buffer,
            petal_variant_data,
            petal_variant_buffer,

            camera_uniform,
            camera_bind_group,
            camera_buffer,

            textured_square_vertices,
            textured_square_vertex_buffer,
            textured_square_index_buffer,
            n_textured_square_indices,
            texture_bind_group,
        }
    }

    fn build_render_pipeline(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        shader_module: &wgpu::ShaderModule,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
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
                PositionTextureVertex::vertex_buffer_layout(),
                gpu_types::Matrix4::vertex_buffer_layout(),
            ],
        };
        // Describes the state of primitve assembly and rasterization in a render pipeline.
        let primitive_state = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None, // Disable culling for petal rendering so we can see front and back
            //cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        };
        let depth_stencil_state = wgpu::DepthStencilState {
            format: texture::Texture::DEPTH_FORMAT,
            depth_write_enabled: true,
            // Draw if new value is less than existing value
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };
        let multisample_state = wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };
        let color_target_state = wgpu::ColorTargetState {
            format: color_format,
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
            depth_stencil: Some(depth_stencil_state),
            multisample: multisample_state,
            fragment: Some(fragment_state),
            multiview: None,
        };
        device.create_render_pipeline(&render_pipeline_descriptor)
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Render to the screen --------------------------------------------------------------------
        // Get the current SurfaceTexture that we will render to.
        let screen_texture = self.surface.get_current_texture()?;
        let screen_texture_view = screen_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let command_encoder = self.render_to_target(RenderTarget::Screen(&screen_texture_view))?;
        self.queue.submit(std::iter::once(command_encoder.finish()));
        screen_texture.present();

        // Render to the video buffer --------------------------------------------------------------
        let mut command_encoder;
        if self.video_export_state.is_some() {
            command_encoder = self.render_to_target(RenderTarget::Video)?;
            if let Some(ref mut video_export_state) = self.video_export_state {
                // Copy the results to the buffer that is readable (mappable) by the CPU
                command_encoder.copy_texture_to_buffer(
                    video_export_state.video_texture.texture.as_image_copy(),
                    wgpu::ImageCopyBuffer {
                        buffer: &video_export_state.video_buffer,
                        layout: wgpu::ImageDataLayout {
                            offset: 0,
                            // TODO: bytes_per_row must be padded to a multiple of
                            // wgpu::COPY_BYTES_PER_ROW_ALIGNMENT, which is 256.  But most modern
                            // standard video resolutions have x resolutions that are multiples of
                            // 64, which satisfies that alignment without any need for padding,
                            // assuming 4 bytes are used per pixel.  It's simpler to assume we're
                            // using one of those resolutions where no padding is needed, since that
                            // also means we don't have to remove the padding from the end of each
                            // row of pixel data before piping the data to ffmpeg (see the "capture"
                            // example in the wgpu repository for an example of how to do that:
                            // https://github.com/gfx-rs/wgpu/tree/master/wgpu/examples/capture).
                            // Thus, for now I'm just making a note in the default configuration
                            // that the x resolution must be a multiple of 64, because otherwise it
                            // will cause a crash for violating COPY_BYTES_PER_ROW_ALIGNMENT.
                            bytes_per_row: Some(
                                std::num::NonZeroU32::new(
                                    video_export_state.video_config.width
                                        * std::mem::size_of::<u32>() as u32,
                                )
                                .unwrap(),
                            ),
                            // A value for rows_per_image is only required if there are multiple images
                            // (i.e. the depth is more than 1).
                            rows_per_image: None, //Some(std::num::NonZeroU32::new(VIDEO_HEIGHT).unwrap()),
                        },
                    },
                    wgpu::Extent3d {
                        width: video_export_state.video_config.width,
                        height: video_export_state.video_config.height,
                        depth_or_array_layers: 1,
                    },
                );
                let video_render_submission_index =
                    self.queue.submit(std::iter::once(command_encoder.finish()));

                let buffer_slice = video_export_state.video_buffer.slice(..);
                let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
                // This queues up the buffer to be mapped, and then calls the FnOnce with a result
                // passed in indicated when it has been mapped and is ready to be read from (or an error
                // has occurred).  The oneshot channel allows me to easily wait until that FnOnce has
                // been called before accessing the buffer's contents.
                buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                    sender.send(result).unwrap()
                });
                // Wait for our submitted commands to render to the video texture to finish executing
                self.device.poll(wgpu::Maintain::WaitForSubmissionIndex(
                    video_render_submission_index,
                ));
                if let Some(Ok(())) = pollster::block_on(receiver.receive()) {
                    let padded_buffer = buffer_slice.get_mapped_range();
                    let frame_pixel_data = padded_buffer.to_owned();
                    video_export_state
                        .video_thread_tx
                        .as_ref()
                        .unwrap()
                        .send(frame_pixel_data)
                        .unwrap();
                    // Must drop any views into the buffer before we unmap it.
                    drop(padded_buffer);
                    video_export_state.video_buffer.unmap();
                } else {
                    log::error!("Buffer failed to map");
                }
            }
        }
        Ok(())
    }

    fn render_to_target(
        &mut self,
        render_target: RenderTarget,
    ) -> Result<wgpu::CommandEncoder, wgpu::SurfaceError> {
        let (color_view, depth_view) = match render_target {
            RenderTarget::Screen(screen_texture_view) => {
                (screen_texture_view, &self.depth_texture.view)
            }
            RenderTarget::Video => {
                if let Some(video_export_state) = self.video_export_state.as_ref() {
                    (
                        &video_export_state.video_texture.view,
                        &video_export_state.video_depth_texture.view,
                    )
                } else {
                    unreachable!();
                }
            }
        };
        let mut command_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        // Render things with textured vertexes ----------------------------------------------------
        let mut textured_vertex_render_pass =
            command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Textured vertex render pass"),
                color_attachments: &[
                    // This is what @location(0) in the fragment shader output targets
                    Some(wgpu::RenderPassColorAttachment {
                        view: color_view,
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
        match render_target {
            RenderTarget::Screen(_) => {
                textured_vertex_render_pass.set_pipeline(&self.render_pipeline)
            }
            RenderTarget::Video => {
                if let Some(video_export_state) = self.video_export_state.as_ref() {
                    textured_vertex_render_pass
                        .set_pipeline(&video_export_state.video_render_pipeline)
                }
            }
        };
        textured_vertex_render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        textured_vertex_render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
        textured_vertex_render_pass
            .set_vertex_buffer(0, self.textured_square_vertex_buffer.slice(..));
        textured_vertex_render_pass.set_vertex_buffer(1, self.petal_pose_buffer.slice(..));
        textured_vertex_render_pass.set_index_buffer(
            self.textured_square_index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        textured_vertex_render_pass.draw_indexed(
            0..self.n_textured_square_indices,
            0,
            0..self.petal_pose_data.len() as _,
        );
        drop(textured_vertex_render_pass);

        Ok(command_encoder)
    }

    /// Update data in the GPU buffers according to the data as currently reflected in the game
    /// state.
    pub fn update(
        &mut self,
        camera: &camera::UprightPerspectiveCamera,
        petal_states: &[crate::state::PetalState],
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
        self.queue.write_buffer(&self.camera_buffer, 0, unsafe {
            sized_type_as_u8_slice(&self.camera_uniform)
        });

        // Update the instance buffer with the current instance poses, and update the petal variant
        // index buffer with the current variant indices (this needs to be updated each frame if
        // the z-sorting changes).
        for ((pose_matrix, variant_index), petal_state) in self
            .petal_pose_data
            .iter_mut()
            .zip(self.petal_variant_index_data.iter_mut())
            .zip(petal_states.iter())
        {
            pose_matrix.matrix = gpu_types::Matrix4::from(&petal_state.pose).matrix;
            *variant_index = petal_state.variant_index;
        }
        self.queue.write_buffer(&self.petal_pose_buffer, 0, unsafe {
            vec_as_u8_slice(&self.petal_pose_data)
        });
        self.queue
            .write_buffer(&self.petal_variant_index_buffer, 0, unsafe {
                vec_as_u8_slice(&self.petal_variant_index_data)
            });
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        log::debug!("Resizing to {:?}", new_size);
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
        self.depth_texture = texture::Texture::create_depth_buffer_texture(
            &self.device,
            self.surface_config.width,
            self.surface_config.height,
            Some("depth texture"),
        );
    }

    /// Return the width/height ratio for the rendering surface.
    pub fn get_aspect_ratio(&self) -> f32 {
        self.surface_config.width as f32 / self.surface_config.height as f32
    }
}

impl Drop for GraphicsState {
    fn drop(&mut self) {
        log::debug!("Dropping GraphicsState");
        if let Some(mut video_export_state) = self.video_export_state.take() {
            let tx = video_export_state.video_thread_tx.take();
            // Close the channel so that the video coding thread will exit normally.
            drop(tx);
            // Wait for the video coding thread to exit normally.
            if let Some(thread_handle) = video_export_state.video_thread_handle.take() {
                thread_handle.join().unwrap().unwrap();
            }
        }
    }
}

fn video_thread_fn(
    receiver: std::sync::mpsc::Receiver<Vec<u8>>,
    output_file: String,
    video_width: u32,
    video_height: u32,
    video_fps: u32,
) -> std::io::Result<()> {
    log::debug!("Video thread starting.");
    let size_str = format!("{video_width}x{video_height}");
    let frame_rate_str = video_fps.to_string();
    let gop_str = (video_fps / 2).to_string();
    // Info on ffmpeg: https://ffmpeg.org/ffmpeg.html
    //   input file(s) --> [demuxer] --> encoded data packets --> [decoder] --> decoded frames
    //     --> [optional filter graph] --> filtered frames --> [encoder] --> encoded data packets
    //     --> [muxer] --> output file(s)
    //   Demuxers/muxers read/write encoded data packets from/to container file formats.
    //   Command-line options generally apply to the next-specified input or output file (so order
    //     relative to the input/output file parameters matters).
    let mut ffmpeg_process = std::process::Command::new("ffmpeg")
        .args([
            // Global options
            "-y", // Overwrite output files if they already exist
            // Input options
            "-f",            // Format
            "rawvideo",      //   raw (no header information, just raw pixel values)
            "-pix_fmt",      // Pixel format
            "bgra",          //   bgra
            "-s",            // Size (resolution)
            &size_str,       //
            "-r",            // Frame rate
            &frame_rate_str, //
            "-i",            // Input file
            "-",             //   Input is coming from stdin
            // Output options
            "-c:v",       // Video codec
            "libx264",    //   The default H.264 codec
            "-preset",    // Preset for video codec
            "slow",       //   Slower gives better compression
            "-crf",       // Quality setting for libx264 codec
            "18",         //   0=51, lower is higher quality, 23 is default
            "-pix_fmt",   // Pixel format
            "yuv420p",    //   yuv420p (recommended by YouTube, needed for many devices)
            "-g",         // Set GOP size (group of pictures / # frames between keyframes)
            &gop_str,     //   YouTube recommends a GOP of half the frame rate
            "-bf",        // Limit consecutive B-frames (bidirectionally predicted frames)
            "2",          //   Youtube recommends a limit of 2 consecutive B-frames.
            "-movflags",  // Muxer flags
            "+faststart", //   YouTube recommends MOOV atom at the beginning of the file
            "-an",        // No audio
            &output_file, // Output file
        ])
        .stdin(std::process::Stdio::piped())
        .spawn()?;
    let mut ffmpeg_stdin = ffmpeg_process.stdin.take().unwrap();
    let mut frame_count = 0;
    while let Ok(message) = receiver.recv() {
        ffmpeg_stdin.write_all(&message).unwrap();
        frame_count += 1;
        if frame_count % video_fps == 0 {
            log::debug!("Video duration = {}s", frame_count / video_fps);
        }
    }
    log::debug!("Flushing out last frames");
    ffmpeg_stdin.flush().unwrap();
    log::debug!("Done flushing frames.  Closing the pipe and waiting for ffmpeg to finish...");
    // Close the pipe to ffmpeg so that ffmpeg will finish and exit
    drop(ffmpeg_stdin);
    // Wait for ffmpeg to finish and exit
    let ffmpeg_output = ffmpeg_process.wait_with_output().unwrap();
    log::debug!("ffmpeg finished! Output: {:?}", ffmpeg_output);
    Ok(())
}
