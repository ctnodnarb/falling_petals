use anyhow::*;
use image::GenericImageView;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub texture_label: Option<String>,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub sampler_label: Option<String>,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_buffer_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: Option<&str>,
    ) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture_label = label.map(String::from);
        let texture_descriptor = wgpu::TextureDescriptor {
            label: texture_label.as_ref().map(|label| label as &str),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage:
                // Allow texture to be attached as the output of a render pass (it gets written to
                // by the rendering pass)
                wgpu::TextureUsages::RENDER_ATTACHMENT
                // Allow texture to be used as BindingType::Texture in a bind group (allow the
                // texture to be used in shaders)
                | wgpu::TextureUsages::TEXTURE_BINDING
        };
        let texture = device.create_texture(&texture_descriptor);
        // Create a view into the texture (they can have multiple views, e.g. a view of a single 2d
        // slice of a 3d texture).
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // Generate a sampler to fill that field in our struct and in case we ever want to sample
        // the depth texture for some reason.  Often don't really NEED this though.
        let sampler_label = label.map(|texture_label| format!("{} sampler", texture_label));
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: sampler_label.as_ref().map(|x| x as &str),
            // For texture coords outside the range, use the closest texture color on the edge of
            // the texture
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            // Use linear interpolation when magnifying or minifying
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            // If we do decide to render our depth texture, we need to use
            // CompareFunction::LessEqual due to how the sampler_comparison and
            // textureSampleCompare() interacts with the texture() function in GLSL.
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });
        Self {
            texture,
            texture_label,
            view,
            sampler,
            sampler_label,
        }
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> Result<Self> {
        // TODO: It would probably be good to use 16 or 32 bit color instead of rgba8.  Making this
        // change will likely require similar changes in the structures / buffers / uniforms
        // definitions in the shader code and shader definition / setup code.
        let rgba_image = img.to_rgba8();
        let dimensions = img.dimensions();
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture_label = label.map(String::from);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: texture_label.as_ref().map(|label| label as &str),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Most images are stored using sRGB format
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage:
                // Allow the texture to be used in bind groups (so it can be used in shaders)
                wgpu::TextureUsages::TEXTURE_BINDING
                // Allows us to copy data to the texture
                | wgpu::TextureUsages::COPY_DST,
        });
        queue.write_texture(
            // Tell wgpu where and how to copy the texture data
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            // The actual image data to copy into the texture
            &rgba_image,
            // The layout of the image data (rgba_image)
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0),
                rows_per_image: std::num::NonZeroU32::new(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler_label = label.map(|texture_label| format!("{} sampler", texture_label));
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: sampler_label.as_ref().map(|label| label as &str),
            // How to handle texture coords outside the range of the texture
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            // How to interpolate when magnifying / minifying the texture
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            // How to blend between mipmaps
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        Ok(Self {
            texture,
            texture_label,
            view,
            sampler,
            sampler_label,
        })
    }

    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: Option<&str>,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, label)
    }

    pub fn from_descriptor(
        device: &wgpu::Device,
        texture_descriptor: &wgpu::TextureDescriptor,
    ) -> Self {
        let texture_label = texture_descriptor.label.map(String::from);
        let texture = device.create_texture(texture_descriptor);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler_label = texture_label
            .as_ref()
            .map(|texture_label| format!("{} sampler", texture_label));
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: sampler_label.as_ref().map(|label| label as &str),
            // How to handle texture coords outside the range of the texture
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            // How to interpolate when magnifying / minifying the texture
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            // How to blend between mipmaps
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        Self {
            texture,
            texture_label,
            view,
            sampler,
            sampler_label,
        }
    }

    //pub fn from_mandelbrot(device: &wgpu::Device, queue: &wgpu::Queue, size: u32, label: &str) -> Self {
    //    let texels = Self::generate_mandelbrot_texels(size as usize);
    //    todo!("Dont think the below will work since it is in R8uint format");
    //    Self::from_bytes(device, queue, &texels[..], label).unwrap()
    //}

    //fn generate_mandelbrot_texels(size: usize) -> Vec<u8> {
    //    (0..size * size)
    //        .map(|id| {
    //            // get high five for recognizing this ;)
    //            let cx = 3.0 * (id % size) as f32 / (size - 1) as f32 - 2.0;
    //            let cy = 2.0 * (id / size) as f32 / (size - 1) as f32 - 1.0;
    //            let (mut x, mut y, mut count) = (cx, cy, 0);
    //            while count < 0xFF && x * x + y * y < 4.0 {
    //                let old_x = x;
    //                x = x * x - y * y + cx;
    //                y = 2.0 * old_x * y + cy;
    //                count += 1;
    //            }
    //            count
    //        })
    //        .collect()
    //}
}
