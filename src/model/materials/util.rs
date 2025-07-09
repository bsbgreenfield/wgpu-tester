use std::path::PathBuf;

use gltf::buffer::View;

pub(super) fn get_image_bytes_from_view(view: &View, main_buffer_data: &Vec<u8>) -> Vec<u8> {
    main_buffer_data[view.offset()..view.offset() + view.length()].to_vec()
}

pub(super) fn find_texure_file(uri: &str) -> Result<PathBuf, std::io::Error> {
    let res_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("res")
        .join(uri);

    println!("{res_dir:?}");
    if !res_dir.exists() {
        eprintln!("res directory does not exist: {:?}", res_dir);
        return Err(std::io::ErrorKind::NotFound.into());
    } else {
        return Ok(res_dir);
    }
}

pub(super) fn get_image_from_path(path: &PathBuf) -> Result<image::DynamicImage, std::io::Error> {
    let bytes = std::fs::read(path)?;
    let diffuse_image = image::load_from_memory(&bytes).unwrap();
    Ok(diffuse_image)
}

pub(super) fn address_mode_from_gltf(wrap_mode: gltf::texture::WrappingMode) -> wgpu::AddressMode {
    match wrap_mode {
        gltf::texture::WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        gltf::texture::WrappingMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
        gltf::texture::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
    }
}
pub(super) fn mag_filter_from_gltf(
    filter_mode: Option<gltf::texture::MagFilter>,
) -> wgpu::FilterMode {
    match filter_mode {
        Some(fm) => match fm {
            gltf::texture::MagFilter::Nearest => wgpu::FilterMode::Nearest,
            gltf::texture::MagFilter::Linear => wgpu::FilterMode::Linear,
        },
        None => wgpu::FilterMode::Nearest,
    }
}
pub(super) fn min_filter_from_gltf(
    filter_mode: Option<gltf::texture::MinFilter>,
) -> (wgpu::FilterMode, Option<wgpu::FilterMode>) {
    match filter_mode {
        Some(fm) => match fm {
            gltf::texture::MinFilter::Nearest => (wgpu::FilterMode::Nearest, None),
            gltf::texture::MinFilter::Linear => (wgpu::FilterMode::Linear, None),
            gltf::texture::MinFilter::NearestMipmapNearest => {
                (wgpu::FilterMode::Nearest, Some(wgpu::FilterMode::Nearest))
            }
            gltf::texture::MinFilter::LinearMipmapNearest => {
                (wgpu::FilterMode::Linear, Some(wgpu::FilterMode::Nearest))
            }
            gltf::texture::MinFilter::NearestMipmapLinear => {
                (wgpu::FilterMode::Nearest, Some(wgpu::FilterMode::Linear))
            }
            gltf::texture::MinFilter::LinearMipmapLinear => {
                (wgpu::FilterMode::Linear, Some(wgpu::FilterMode::Linear))
            }
        },
        None => (wgpu::FilterMode::Linear, None),
    }
}
