use std::path::PathBuf;

use image::GenericImageView;
use wgpu::{Extent3d, FilterMode, TextureUsages};

use crate::model::materials::{
    texture::GTexture,
    util::{
        self, address_mode_from_gltf, find_texure_file, get_image_bytes_from_view,
        mag_filter_from_gltf, min_filter_from_gltf,
    },
};

#[allow(unused)]
pub struct MaterialDefinition<'a> {
    pub image_source: Option<PathBuf>,
    buffer_bytes: Option<Vec<u8>>,
    base_color_factors: [f32; 4],
    texture_descriptor: wgpu::TextureDescriptor<'a>,
    sampler_descripor: wgpu::SamplerDescriptor<'a>,
    view_descriptor: wgpu::TextureViewDescriptor<'a>,
}
impl<'a> MaterialDefinition<'a> {
    pub fn new(material: &gltf::material::Material, main_buffer_bytes: &Vec<u8>) -> Self {
        let texture_descriptor: wgpu::TextureDescriptor = wgpu::TextureDescriptor {
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
                    label: None,
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
                label: None,
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: FilterMode::Nearest,
                min_filter: FilterMode::Nearest,
                mipmap_filter: FilterMode::Nearest,
                ..Default::default()
            },
        };

        let mut image_path: Option<PathBuf> = None;
        let mut image_bytes: Option<Vec<u8>> = None;
        if let Some(bct) = material.pbr_metallic_roughness().base_color_texture() {
            let bct_texture = bct.texture();
            match bct_texture.source().source() {
                gltf::image::Source::View { view, mime_type: _ } => {
                    image_bytes = Some(get_image_bytes_from_view(&view, main_buffer_bytes));
                }
                gltf::image::Source::Uri { uri, mime_type: _ } => {
                    println!("searching for image {:?}", uri);
                    image_path = find_texure_file(uri).ok();
                }
            }
        }
        let base_colors = material.pbr_metallic_roughness().base_color_factor();
        let m = MaterialDefinition {
            image_source: image_path,
            buffer_bytes: image_bytes,
            sampler_descripor: sampler_descriptor,
            texture_descriptor,
            view_descriptor: wgpu::TextureViewDescriptor {
                label: None,
                format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
                dimension: Some(wgpu::TextureViewDimension::D2),
                array_layer_count: None,
                base_mip_level: 0,
                base_array_layer: 0,
                mip_level_count: None,
                usage: Some(TextureUsages::TEXTURE_BINDING),
                aspect: wgpu::TextureAspect::All,
            },
            base_color_factors: base_colors,
        };
        m
    }
}

pub struct GMaterial {
    pub texture: GTexture,
    image: image::DynamicImage,
}

impl GMaterial {
    pub fn from_material_definition_with_bgl(
        material_def: &mut MaterialDefinition,
        device: &wgpu::Device,
        bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        assert!(material_def.image_source.is_some() || material_def.buffer_bytes.is_some());
        let image: image::DynamicImage = match &material_def.image_source {
            Some(image_src) => {
                util::get_image_from_path(&image_src).expect("image is located in path")
            }
            None => {
                let image_bytes = material_def.buffer_bytes.as_ref().unwrap();
                image::load_from_memory(image_bytes).expect("image data in bytes")
            }
        };
        material_def.texture_descriptor.size = Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        };
        let texture = GTexture::new(
            &material_def.texture_descriptor,
            &material_def.sampler_descripor,
            &material_def.view_descriptor,
            bgl,
            device,
        );
        return Self { texture, image };
    }

    pub fn write_texture_2d(&self, queue: &wgpu::Queue) {
        let w = self.image.dimensions().0;
        let h = self.image.dimensions().1;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &self.image.to_rgba8(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    }
}
