use std::path::PathBuf;

use wgpu::{Extent3d, FilterMode, TextureUsages};

use crate::model::materials::util::find_texure_file;

pub struct GTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: Option<wgpu::BindGroup>,
}

impl GTexture {
    pub fn from_texture(
        device: &wgpu::Device,
        texture: &gltf::Texture,
    ) -> Result<Self, image::ImageError> {
        let g_image = texture.source();
        let image = match g_image.source() {
            gltf::image::Source::View { view, mime_type } => {
                todo!();
            }
            gltf::image::Source::Uri { uri, mime_type } => {
                let path_buf: PathBuf = find_texure_file(uri)?;
                let bytes = std::fs::read(path_buf)?;
                image::load_from_memory(&bytes)?
            }
        };
        Ok(Self::from_image(
            device,
            &texture.sampler(),
            texture,
            &image,
        ))
    }

    pub fn from_diffuse_factors(factors: &[f32; 4], device: &wgpu::Device) -> Self {
        let t = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: Extent3d::default(),
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let s = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });
        let v = t.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture: t,
            view: v,
            sampler: s,
            bind_group: None,
        }
    }

    fn from_image(
        device: &wgpu::Device,
        sampler: &gltf::texture::Sampler,
        texture: &gltf::texture::Texture,
        image: &image::DynamicImage,
    ) -> Self {
        let gpu_texture = Self::create_texure(
            texture,
            device,
            Extent3d {
                width: image.width(),
                height: image.height(),
                depth_or_array_layers: 1,
            },
        );
        let sampler = Self::create_sampler(sampler, device);
        let view = gpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture: gpu_texture,
            view,
            sampler,
            bind_group: None,
        }
    }
    fn create_texure(
        txt: &gltf::texture::Texture,
        device: &wgpu::Device,
        size: Extent3d,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: txt.name(),
            size,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    pub fn create_sampler(
        sampler: &gltf::texture::Sampler,
        device: &wgpu::Device,
    ) -> wgpu::Sampler {
        let min_filter = Self::min_filter_from_gltf(sampler.min_filter());

        //TODO: allow for mipmaps
        device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sampler"),
            address_mode_u: Self::address_mode_from_gltf(sampler.wrap_s()),
            address_mode_v: Self::address_mode_from_gltf(sampler.wrap_t()),
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: Self::mag_filter_from_gltf(sampler.mag_filter()),
            min_filter: min_filter.0,
            mipmap_filter: min_filter.1.unwrap_or(wgpu::FilterMode::Nearest),
            ..Default::default()
        })
    }
}
