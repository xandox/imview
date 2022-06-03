use crate::image_ui_state::DiffMode;
use crate::utils::make_color_image;
use eframe::egui::*;
use image::imageops::crop_imm;
use image::RgbaImage;
use std::path::Path;
pub struct ImageData {
    base_name: String,
    image: Option<RgbaImage>,
    width: f32,
    height: f32,
    color_diff_vsplited: Option<RgbaImage>,
    color_diff_hsplited: Option<RgbaImage>,
    texture_handle: Option<TextureHandle>,
    cd_texture_handle: Option<TextureHandle>,
    pub error_msg: Option<String>,
}

impl ImageData {
    pub fn thumbnail(path: &Path, img: RgbaImage, cc: &Context) -> Self {
        let name = format!("{}_thmb", path.display());
        let texture_handle = cc.load_texture(name, make_color_image(&img));
        Self {
            base_name: path.display().to_string(),
            image: None,
            width: img.width() as _,
            height: img.height() as _,
            color_diff_vsplited: None,
            color_diff_hsplited: None,
            texture_handle: Some(texture_handle),
            cd_texture_handle: None,
            error_msg: None,
        }
    }

    pub fn error(err: &dyn std::error::Error) -> Self {
        Self {
            base_name: String::new(),
            image: None,
            width: 0.0,
            height: 0.0,
            color_diff_vsplited: None,
            color_diff_hsplited: None,
            texture_handle: None,
            cd_texture_handle: None,
            error_msg: Some(format!("{}", err)),
        }
    }

    pub fn full_image(path: &Path, img: RgbaImage, cc: &Context) -> Self {
        let name = format!("{}_full", path.display());
        let texture_handle = cc.load_texture(name, make_color_image(&img));
        Self {
            base_name: path.display().to_string(),
            width: img.width() as _,
            height: img.height() as _,
            image: Some(img),
            color_diff_vsplited: None,
            color_diff_hsplited: None,
            texture_handle: Some(texture_handle),
            cd_texture_handle: None,
            error_msg: None,
        }
    }

    pub fn size(&self) -> Vec2 {
        vec2(self.width, self.height)
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn color_texture_handle(&self) -> &TextureHandle {
        self.texture_handle.as_ref().unwrap()
    }

    pub fn color_diff_texture_handle(&self) -> &TextureHandle {
        self.cd_texture_handle.as_ref().unwrap()
    }

    pub fn texture_handle(&self, diff_mode: DiffMode) -> &TextureHandle {
        match diff_mode {
            DiffMode::Full | DiffMode::VSplit | DiffMode::HSplit => self.color_texture_handle(),
            DiffMode::VColorDiff | DiffMode::HColorDiff => self.color_diff_texture_handle(),
        }
    }

    fn create_hdiff_image(&self) -> RgbaImage {
        let w = self.width as _;
        let h = (self.height / 2.0) as _;
        let img = self.image.as_ref().unwrap();
        let left_img = crop_imm(img, 0, 0, w, h).to_image();
        let right_img = crop_imm(img, 0, h, w, h).to_image();
        Self::image_diff(left_img, right_img)
    }

    fn create_vdiff_image(&self) -> RgbaImage {
        let w = (self.width / 2.0) as _;
        let h = self.height as _;
        let img = self.image.as_ref().unwrap();
        let left_img = crop_imm(img, 0, 0, w, h).to_image();
        let right_img = crop_imm(img, w, 0, w, h).to_image();
        Self::image_diff(left_img, right_img)
    }

    fn image_diff(mut one: RgbaImage, two: RgbaImage) -> RgbaImage {
        let (w, h) = one.dimensions();
        for y in 0..h {
            for x in 0..w {
                let op = one.get_pixel_mut(x, y);
                let tp = two.get_pixel(x, y);
                for c in 0..3 {
                    let diff = (op[c] as i32 - tp[c] as i32).abs() as u8;
                    op[c] = diff;
                }
            }
        }
        one
    }

    fn image_gamma(mut img: RgbaImage, gamma: f32) -> RgbaImage {
        let inv_gamma = 1.0 / gamma;
        let (width, height) = img.dimensions();
        for y in 0..height {
            for x in 0..width {
                let p = img.get_pixel_mut(x, y);
                for c in 0..3 {
                    let v = p[c] as f32;
                    let v = (v / 255.0).powf(inv_gamma) * 255.0;
                    let v = v as u8;
                    p[c] = v
                }
            }
        }
        img
    }

    fn create_color_diff_texture(&mut self, cc: &Context, image: RgbaImage) {
        let egui_image = make_color_image(&image);
        self.cd_texture_handle =
            Some(cc.load_texture(format!("{}_color_diff", self.base_name), egui_image));
    }

    pub fn switch_to_horizontal_color_diff(&mut self, ctx: &Context, gamma: f32) {
        if self.color_diff_hsplited.is_none() {
            self.color_diff_hsplited = Some(self.create_hdiff_image())
        }
        let img = Self::image_gamma(self.color_diff_hsplited.as_ref().unwrap().clone(), gamma);
        self.create_color_diff_texture(ctx, img);
    }

    pub fn switch_to_vertical_color_diff(&mut self, ctx: &Context, gamma: f32) {
        if self.color_diff_vsplited.is_none() {
            self.color_diff_vsplited = Some(self.create_vdiff_image())
        }

        let img = Self::image_gamma(self.color_diff_vsplited.as_ref().unwrap().clone(), gamma);
        self.create_color_diff_texture(ctx, img);
    }

    pub fn switch_to_color_image(&mut self, cc: &Context) {
        let egui_image = make_color_image(self.image.as_ref().unwrap());
        self.texture_handle = Some(cc.load_texture(format!("{}_full", self.base_name), egui_image));
    }
}
