use wgpu::{Extent3d, FilterMode, TextureUsages};

pub struct GTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
}

impl GTexture {
    pub fn new(
        t: &wgpu::TextureDescriptor,
        s: &wgpu::SamplerDescriptor,
        v: &wgpu::TextureViewDescriptor,
        bgl: &wgpu::BindGroupLayout,
        device: &wgpu::Device,
    ) -> Self {
        let texture = device.create_texture(t);
        let sampler = device.create_sampler(s);
        let view = texture.create_view(v);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            texture,
            view,
            sampler,
            bind_group,
        }
    }

    #[allow(unused)]
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
        todo!()
    }
}
