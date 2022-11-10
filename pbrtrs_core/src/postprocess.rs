#[cfg(feature = "enable_oidn")]
mod oidn_impl {
    use image::Rgb32FImage;
    use oidn::RayTracing;

    pub fn denoise(image: &mut Rgb32FImage) {
        let device = oidn::Device::new();
        RayTracing::new(&device)
            .srgb(false)
            .image_dimensions(image.width() as usize, image.height() as usize)
            .hdr(true)
            .filter_in_place(image)
            .unwrap();

        if let Err(e) = device.get_error() {
            println!("Error denoising image: {}", e.1);
        }
    }
}

#[cfg(feature = "enable_oidn")]
pub use oidn_impl::*;
