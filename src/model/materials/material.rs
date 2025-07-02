use std::path::PathBuf;

use wgpu::{Extent3d, FilterMode, TextureUsages};

use crate::model::materials::{
    texture::GTexture,
    util::{address_mode_from_gltf, find_texure_file, mag_filter_from_gltf, min_filter_from_gltf},
};

pub struct MaterialDefinition<'a> {
    image_source: Option<PathBuf>,
    buffer_bytes: Option<Vec<u8>>,
    base_color_factors: [f32; 4],
    texture_descriptor: wgpu::TextureDescriptor<'a>,
    sampler_descripor: wgpu::SamplerDescriptor<'a>,
    view_descriptor: wgpu::TextureViewDescriptor<'a>,
}
impl<'a> MaterialDefinition<'a> {
    fn new(material: &gltf::material::Material) -> Self {
        let mut texture_descriptor: wgpu::TextureDescriptor = wgpu::TextureDescriptor {
            label: None,
            size: Extent3d::default(),
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        };

        let maybe_texture = material.pbr_metallic_roughness().base_color_texture();

        let sampler_descriptor = match maybe_texture {
            Some(tex) => {
                let sampler = tex.texture().sampler();
                let min_filter = min_filter_from_gltf(sampler.min_filter());
                wgpu::SamplerDescriptor {
                    label: Some("sampler"),
                    address_mode_u: address_mode_from_gltf(sampler.wrap_s()),
                    address_mode_v: address_mode_from_gltf(sampler.wrap_t()),
                    address_mode_w: wgpu::AddressMode::Repeat,
                    mag_filter: mag_filter_from_gltf(sampler.mag_filter()),
                    min_filter: min_filter.0,
                    mipmap_filter: min_filter.1.unwrap_or(wgpu::FilterMode::Nearest),
                    ..Default::default()
                }
            }
            None => wgpu::SamplerDescriptor {
                label: Some("sampler"),
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: FilterMode::Nearest,
                min_filter: FilterMode::Nearest,
                mipmap_filter: FilterMode::Nearest,
                ..Default::default()
            },
        };

        match material.pbr_metallic_roughness().base_color_texture() {
            Some(bct) => {
                let bct_texture = bct.texture();
                match bct_texture.source().source() {
                    gltf::image::Source::View { view, mime_type } => {
                        todo!()
                    }
                    gltf::image::Source::Uri { uri, mime_type } => {
                        texture_descriptor.label = bct_texture.name();
                        let m = MaterialDefinition {
                            image_source: Some(find_texure_file(uri).unwrap()), // TODO: propgogate
                            // errors
                            buffer_bytes: None,
                            sampler_descripor: sampler_descriptor,
                        };
                    }
                }
            }
        }
    }
}

pub struct GMaterial {
    texture: GTexture,
    id: usize,
}

impl GMaterial {
    pub fn new(
        material: &gltf::Material,
        device: &wgpu::Device,
    ) -> Result<Self, image::ImageError> {
        let tex = match material.pbr_metallic_roughness().base_color_texture() {
            Some(bct) => GTexture::from_texture(device, &bct.texture())?,
            None => GTexture::from_diffuse_factors(
                &material.pbr_metallic_roughness().base_color_factor(),
                device,
            ),
        };
        Ok(Self {
            texture: tex,
            id: material.index().unwrap_or(9999),
        })
    }
}
