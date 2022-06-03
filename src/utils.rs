use eframe::egui::ColorImage;
use image::RgbaImage;

pub fn make_color_image(image: &RgbaImage) -> ColorImage {
    let w = image.width() as _;
    let h = image.height() as _;
    let size = [w, h];
    let pixels = image.as_flat_samples();
    let color_image = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
    color_image
}
