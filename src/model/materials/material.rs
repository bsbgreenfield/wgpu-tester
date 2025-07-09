use std::{
    fmt::{self, Debug},
    num::NonZero,
    path::PathBuf,
};

use image::{GenericImageView, ImageBuffer};
use wgpu::{BindingResource, BufferBinding, Extent3d, FilterMode, TextureUsages};

use crate::model::materials::{
    texture::GTexture,
    util::{
        self, address_mode_from_gltf, find_texure_file, get_image_bytes_from_view,
        mag_filter_from_gltf, min_filter_from_gltf,
    },
};

#[allow(unused)]
pub struct MaterialDefinition<'a> {
    /// the index of the material in the eventual Vec<GMaterial> stored in app state.
    /// to be used during the render pass
    pub index: u32,
    /// The GLTF id of this material, stored so that we can avoid duplication of materials
    pub id: usize,
    pub image_source: Option<PathBuf>,
    buffer_bytes: Option<Vec<u8>>,
    pub base_color_factors: [f32; 4],
    texture_descriptor: wgpu::TextureDescriptor<'a>,
    sampler_descriptor: wgpu::SamplerDescriptor<'a>,
    view_descriptor: wgpu::TextureViewDescriptor<'a>,
}

impl<'a> Debug for MaterialDefinition<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::write(
            f,
            format_args!(
                "index: {}, id: {}, image_source: {:?}, base_colors: {:?}",
                self.index, self.id, self.image_source, self.base_color_factors
            ),
        )
    }
}
impl<'a> MaterialDefinition<'a> {
    pub fn white() -> Self {
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
        let sampler_descriptor = wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        };

        Self {
            index: 0,
            id: 9999,
            image_source: None,
            buffer_bytes: None,
            base_color_factors: [1.0, 1.0, 1.0, 1.0],
            texture_descriptor,
            sampler_descriptor,
            view_descriptor: wgpu::TextureViewDescriptor::default(),
        }
    }
    pub fn new(
        material: &gltf::material::Material,
        main_buffer_bytes: &Vec<u8>,
        material_index: usize,
    ) -> Self {
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

        let m = MaterialDefinition {
            index: material_index as u32,
            id: material.index().unwrap_or(0),
            image_source: image_path,
            buffer_bytes: image_bytes,
            sampler_descriptor,
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
            base_color_factors: material.pbr_metallic_roughness().base_color_factor(),
        };
        m
    }
}

pub struct GMaterial {
    pub texture: GTexture,
    image: Option<image::DynamicImage>,
    pub bind_group: wgpu::BindGroup,
}

impl GMaterial {
    pub fn from_material_definition_with_bgl(
        material_def: &mut MaterialDefinition,
        device: &wgpu::Device,
        bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        let no_texture: bool =
            !(material_def.image_source.is_some() || material_def.buffer_bytes.is_some());
        let maybe_image = match no_texture {
            true => None,
            false => match &material_def.image_source {
                Some(image_src) => {
                    Some(util::get_image_from_path(&image_src).expect("image is located in path"))
                }
                None => {
                    let image_bytes = material_def.buffer_bytes.as_ref().unwrap();
                    Some(image::load_from_memory(image_bytes).expect("image data in bytes"))
                }
            },
        };
        if let Some(image) = &maybe_image {
            material_def.texture_descriptor.size = Extent3d {
                width: image.width(),
                height: image.height(),
                depth_or_array_layers: 1,
            };
        }
        let texture = GTexture::new(
            &material_def.texture_descriptor,
            &material_def.sampler_descriptor,
            &material_def.view_descriptor,
            device,
        );

        // create the bind group for the base color
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });
        Self {
            image: maybe_image,
            texture,
            bind_group,
        }
    }

    pub fn write_texture_2d(&self, queue: &wgpu::Queue) {
        match &self.image {
            Some(image) => {
                let w = image.dimensions().0;
                let h = image.dimensions().1;
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.texture.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                        aspect: wgpu::TextureAspect::All,
                    },
                    &image.to_rgba8(),
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
            None => queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.texture.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &[255, 255, 255, 255],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4),
                    rows_per_image: Some(1),
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            ),
        }
    }
}
