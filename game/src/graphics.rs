pub mod camera;
pub mod gpu_types;
pub mod texture;

use crate::game::PetalState;
use gpu_types::{PositionTextureVertex, VertexBufferEntry};

use camera::Camera;
use cgmath::prelude::*;
use std::io::Write;
use texture::Texture;
use wgpu::util::DeviceExt;
use winit::window::Window; // Needed for the device.create_buffer_init() function

const VIDEO_WIDTH: u32 = 1920;
const VIDEO_HEIGHT: u32 = 1080;
const VIDEO_FRAME_RATE: u32 = 60;
const VIDEO_PIXEL_COUNT: u32 = VIDEO_WIDTH * VIDEO_HEIGHT;
// One u32 per pixel for Bgra8unorm
const VIDEO_FRAME_SIZE: u64 = std::mem::size_of::<u32>() as u64 * VIDEO_PIXEL_COUNT as u64;
const VIDEO_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

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

// Define the vertices for a square that will be used to render each petal (after being transformed
// by that petal's Pose).
// TSC = Textured Square Center
const TSC: (f32, f32, f32) = (0.0, 0.0, 0.0); //(0.3, 0.5, 0.2);
const TEXTURED_SQUARE_VERTICES: &[PositionTextureVertex; 4] = &[
    PositionTextureVertex {
        // Upper left corner
        position: [-1.0 + TSC.0, 1.0 + TSC.1, TSC.2],
        texture_coords: [0.0, 0.0],
    },
    PositionTextureVertex {
        // Lower left corner
        position: [-1.0 + TSC.0, -1.0 + TSC.1, TSC.2],
        texture_coords: [0.0, 1.0],
    },
    PositionTextureVertex {
        // Lower right corner
        position: [1.0 + TSC.0, -1.0 + TSC.1, TSC.2],
        texture_coords: [1.0, 1.0],
    },
    PositionTextureVertex {
        // Upper right corner
        position: [1.0 + TSC.0, 1.0 + TSC.1, TSC.2],
        texture_coords: [1.0, 0.0],
    },
];
const TEXTURED_SQUARE_INDICES: &[u16; 6] = &[0, 1, 2, 0, 2, 3];

enum RenderTarget<'a> {
    Screen(&'a wgpu::TextureView),
    Video,
}

pub struct GraphicsState {
    // TODO: Go through all the members of this struct and determine if they all actually need to be
    // public.  Make them private where appropriate.

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

    // Object to control the camera and construct the view/projection matrix.
    pub camera_uniform: gpu_types::Matrix4,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,

    // Textured square used to draw petals
    pub textured_square_vertex_buffer: wgpu::Buffer,
    pub textured_square_index_buffer: wgpu::Buffer,
    pub n_textured_square_indices: u32,
    pub texture_bind_group: wgpu::BindGroup,
}

impl GraphicsState {
    pub fn new(
        window: &Window,
        petal_texture_image_paths: &[&str],
        petal_variants: Vec<gpu_types::PetalVariant>,
        petal_states: &[PetalState],
    ) -> Self {
        let size = window.inner_size();

        // -----------------------------------------------------------------------------------------
        log::debug!("WGPU setup");
        let wgpu_instance = wgpu::Instance::new(wgpu::Backends::all());
        //log::debug!("wgpu report:\n{:?}", wgpu_instance.generate_report());
        let surface = unsafe { wgpu_instance.create_surface(window) };
        // The adapter represents the physical instance of your hardware.
        let gpu_adapter =
            pollster::block_on(wgpu_instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }))
            .unwrap();
        log::debug!("Adapter features:\n{:?}", gpu_adapter.features());
        log::debug!("Adapter limits:\n{:?}", gpu_adapter.limits());

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
            //max_uniform_buffer_binding_size: 4 * 65536,
            ..wgpu::Limits::default()
        };
        let (device, queue) = pollster::block_on(gpu_adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::TEXTURE_BINDING_ARRAY
                    | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
                    | wgpu::Features::BUFFER_BINDING_ARRAY,
                //features: wgpu::Features::empty(),
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
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&gpu_adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
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
            VIDEO_WIDTH,
            VIDEO_HEIGHT,
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
                    Some(format!("Petal texture {}", idx).as_str()),
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
        let petal_variant_index_data = petal_states
            .iter()
            .map(|state| state.variant_index)
            .collect::<Vec<_>>();
        let petal_variant_index_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Petal variant index buffer"),
                contents: unsafe { vec_as_u8_slice(&petal_variant_index_data) },
                // TODO: Do I need COPY_DST for buffers if I'm not going to write to them again after
                // the initial initialization?
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let petal_variant_data = petal_variants;
        let petal_variant_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Petal variant buffer"),
            contents: unsafe { vec_as_u8_slice(&petal_variant_data) },
            // TODO: Do I need COPY_DST for buffers if I'm not going to write to them again after
            // the initial initialization?
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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
                        count: None, //core::num::NonZeroU32::new(petal_variant_data.len() as u32),
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
                        count: None, //core::num::NonZeroU32::new(petal_poses.len() as u32),
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
            .replace("N_PETALS", &petal_states.len().to_string())
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
            VIDEO_TEXTURE_FORMAT,
            &shader_module,
            &texture_bind_group_layout,
            &camera_bind_group_layout,
        );

        // -----------------------------------------------------------------------------------------
        log::debug!("Textured square vertex & index buffer setup");
        let textured_square_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Textured pentagon vertex buffer"),
                contents: unsafe { sized_type_as_u8_slice(TEXTURED_SQUARE_VERTICES) },
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
        log::debug!("Set up video output objects");
        let video_texture_descriptor = wgpu::TextureDescriptor {
            label: Some("video output texture"),
            size: wgpu::Extent3d {
                width: VIDEO_WIDTH,
                height: VIDEO_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: VIDEO_TEXTURE_FORMAT,
            // COPY_SRC so we can copy the texture contents to a buffer (video_output_buffer),
            // RENDER_ATTACHMENT so that we can attach the texture to a render pass so it can be
            // rendered to.
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
        };
        let video_texture = Texture::from_descriptor(&device, &video_texture_descriptor);
        //device.create_texture(&video_texture_descriptor);
        let video_buffer_descriptor = wgpu::BufferDescriptor {
            label: Some("video output buffer"),
            size: VIDEO_FRAME_SIZE,
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
        let video_thread_handle = std::thread::spawn(move || video_thread_fn(video_thread_rx));

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

            video_texture,
            video_buffer,
            video_depth_texture,
            video_render_pipeline,
            video_thread_handle: Some(video_thread_handle),
            video_thread_tx: Some(video_thread_tx),

            petal_pose_data,
            petal_pose_buffer,
            petal_variant_index_data,
            petal_variant_index_buffer,
            petal_variant_data,
            petal_variant_buffer,

            camera_uniform,
            camera_bind_group,
            camera_buffer,

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
            cull_mode: None, // Disable culling for petal rendering
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
        let mut command_encoder = self.render_to_target(RenderTarget::Video)?;
        // Copy the results to the buffer that is readable (mappable) by the CPU
        command_encoder.copy_texture_to_buffer(
            self.video_texture.texture.as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer: &self.video_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    // TODO: looks like bytes_per_row must be padded to a multiple of
                    // wgpu::COPY_BYTES_PER_ROW_ALIGNMENT, which is 256.  But apparently this is
                    // still working even though I'm not ensuring that.
                    bytes_per_row: Some(
                        std::num::NonZeroU32::new(VIDEO_WIDTH * std::mem::size_of::<u32>() as u32)
                            .unwrap(),
                    ),
                    // A value for rows_per_image is only required if there are multiple images
                    // (i.e. the depth is more than 1).
                    rows_per_image: None, //Some(std::num::NonZeroU32::new(VIDEO_HEIGHT).unwrap()),
                },
            },
            wgpu::Extent3d {
                width: VIDEO_WIDTH,
                height: VIDEO_HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        let video_render_submission_index =
            self.queue.submit(std::iter::once(command_encoder.finish()));

        let buffer_slice = self.video_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        // This queus up the buffer to be mapped, and then calls the FnOnce with a result passed in
        // indicated when it has been mapped and is ready to be read from (or an error has
        // occurred).  The oneshot channel allows me to easily wait until that FnOnce has been
        // called before accessing the buffer's contents.
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
            self.video_thread_tx
                .as_ref()
                .unwrap()
                .send(frame_pixel_data)
                .unwrap();
            // Must drop any views into the buffer before we unmap it.
            drop(padded_buffer);
            self.video_buffer.unmap();
        } else {
            log::error!("Buffer failed to map");
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
            RenderTarget::Video => (&self.video_texture.view, &self.video_depth_texture.view),
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
                textured_vertex_render_pass.set_pipeline(&self.video_render_pipeline)
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
        petal_states: &[crate::game::PetalState],
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

        // Update the instance buffer with the current instance poses.
        for (pose_matrix, petal_state) in self.petal_pose_data.iter_mut().zip(petal_states.iter()) {
            pose_matrix.matrix = gpu_types::Matrix4::from(&petal_state.pose).matrix;
        }

        self.queue.write_buffer(&self.petal_pose_buffer, 0, unsafe {
            vec_as_u8_slice(&self.petal_pose_data)
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
        let tx = self.video_thread_tx.take();
        // Close the channel so that the video coding thread will exit normally.
        drop(tx);
        // Wait for the video coding thread to exit normally.
        if let Some(thread_handle) = self.video_thread_handle.take() {
            thread_handle.join().unwrap().unwrap();
        }
    }
}

fn video_thread_fn(receiver: std::sync::mpsc::Receiver<Vec<u8>>) -> std::io::Result<()> {
    log::debug!("Video thread starting.");
    let size_str = format!("{}x{}", VIDEO_WIDTH, VIDEO_HEIGHT);
    let frame_rate_str = VIDEO_FRAME_RATE.to_string();
    let mut ffmpeg_process = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "bgra",
            "-s",
            &size_str,
            "-r",
            &frame_rate_str,
            "-i",
            "-",
            "-c:v",
            "libx264",
            "-an",
            "output_video.h264",
        ])
        .stdin(std::process::Stdio::piped())
        .spawn()?;
    let mut ffmpeg_stdin = ffmpeg_process.stdin.take().unwrap();
    let mut frame_count = 0;
    while let Ok(message) = receiver.recv() {
        ffmpeg_stdin.write_all(&message).unwrap();
        frame_count += 1;
        if frame_count % VIDEO_FRAME_RATE == 0 {
            log::debug!("Video duration = {}s", frame_count / VIDEO_FRAME_RATE);
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

// From the wgpu closure example... but it seems like things are working without me making sure that
// the buffer is properly aligned?
//struct BufferDimensions {
//    width: usize,
//    height: usize,
//    unpadded_bytes_per_row: usize,
//    padded_bytes_per_row: usize,
//}
//
//impl BufferDimensions {
//    fn new(width: usize, height: usize) -> Self {
//        let bytes_per_pixel = std::mem::size_of::<u32>();
//        let unpadded_bytes_per_row = width * bytes_per_pixel;
//        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
//        let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
//        let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
//        Self {
//            width,
//            height,
//            unpadded_bytes_per_row,
//            padded_bytes_per_row,
//        }
//    }
//}
