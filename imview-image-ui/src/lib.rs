use arrayvec::ArrayVec as FVec;
pub use eframe::egui::ColorImage;
use eframe::egui::*;
use std::path::PathBuf;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum DiffMode {
    Full,
    VSplit,
    HSplit,
}

struct ImagePart {
    center: Pos2,
    scale: Option<f32>,
}

impl Default for ImagePart {
    fn default() -> Self {
        ImagePart {
            center: Pos2::new(0.5, 0.5),
            scale: None,
        }
    }
}

impl ImagePart {
    const ZOOM_MIN: f32 = 0.01;
    const ZOOM_MAX: f32 = 1.0;

    pub fn set_scale(&mut self, scale: f32) {
        self.scale = Some(scale);
        self.fix_bounds()
    }

    pub fn set_scale_diff(&mut self, scale_diff: f32) {
        self.scale = Some(self.scale.unwrap_or(1.0) + scale_diff);
        self.fix_bounds();
    }

    pub fn set_center_diff(&mut self, center_diff: Vec2) {
        self.center += center_diff;
        self.fix_bounds();
    }

    fn fix_bounds(&mut self) {
        if self.scale.is_some() {
            self.scale = Some(self.scale.unwrap().clamp(Self::ZOOM_MIN, Self::ZOOM_MAX));
        }
        let s_by_2 = self.scale.unwrap_or(1.0) / 2.0;
        if self.left() < 0.0 {
            self.center.x = s_by_2;
        }
        if self.right() > 1.0 {
            self.center.x = 1.0 - s_by_2;
        }
        if self.top() < 0.0 {
            self.center.y = s_by_2;
        }
        if self.bottom() > 1.0 {
            self.center.y = 1.0 - s_by_2;
        }
    }

    fn left(&self) -> f32 {
        self.center.x - self.scale.unwrap_or(1.0) / 2.0
    }

    fn right(&self) -> f32 {
        self.center.x + self.scale.unwrap_or(1.0) / 2.0
    }

    fn top(&self) -> f32 {
        self.center.y - self.scale.unwrap_or(1.0) / 2.0
    }

    fn bottom(&self) -> f32 {
        self.center.y + self.scale.unwrap_or(1.0) / 2.0
    }

    fn uv_full(&self) -> Rect {
        let r = Rect::from_min_max(
            pos2(self.left(), self.top()),
            pos2(self.right(), self.bottom()),
        );
        r
    }

    fn uv_vsplit(&self, ratio: f32) -> [Rect; 2] {
        let s = self.scale.unwrap_or(1.0) / 2.0;
        let lr = Rect::from_min_max(
            pos2(self.left() / 2.0, self.top()),
            pos2(self.right() / 2.0 - (1.0 - ratio) * s, self.bottom()),
        );
        let rr = Rect::from_min_max(
            pos2(self.left() / 2.0 + 0.5 + ratio * s, self.top()),
            pos2(self.right() / 2.0 + 0.5, self.bottom()),
        );
        [lr, rr]
    }

    fn uv_hsplit(&self, ratio: f32) -> [Rect; 2] {
        let s = self.scale.unwrap_or(1.0) / 2.0;
        let lr = Rect::from_min_max(
            pos2(self.left(), self.top() / 2.0),
            pos2(self.right(), self.bottom() / 2.0 - (1.0 - ratio) * s),
        );
        let rr = Rect::from_min_max(
            pos2(self.left(), self.top() / 2.0 + 0.5 + ratio * s),
            pos2(self.right(), self.bottom() / 2.0 + 0.5),
        );
        [lr, rr]
    }
}

struct ImageDiff {
    mode: DiffMode,
    v_factor: f32,
    h_factor: f32,
}

impl Default for ImageDiff {
    fn default() -> Self {
        Self {
            mode: DiffMode::Full,
            v_factor: 0.5,
            h_factor: 0.5,
        }
    }
}

pub struct ImageUI {
    pub filename: PathBuf,
    texture: TextureHandle,
    width: usize,
    height: usize,
    view_part: ImagePart,
    diff: ImageDiff,
}

impl ImageUI {
    pub const ZOOM_MIN: f32 = ImagePart::ZOOM_MIN;
    pub const ZOOM_MAX: f32 = ImagePart::ZOOM_MAX;

    pub fn new(filename: PathBuf, image: ColorImage, cc: &Context) -> Self {
        let name = filename.display().to_string();
        let width = image.width();
        let height = image.height();
        Self {
            filename,
            texture: cc.load_texture(name, image),
            width: width,
            height: height,
            view_part: ImagePart::default(),
            diff: ImageDiff::default(),
        }
    }

    pub fn uv(&self) -> FVec<Rect, 2> {
        match self.diff.mode {
            DiffMode::Full => {
                let mut r = FVec::new();
                r.push(self.view_part.uv_full());
                r
            }
            DiffMode::VSplit => FVec::from(self.view_part.uv_vsplit(self.diff.v_factor)),
            DiffMode::HSplit => FVec::from(self.view_part.uv_hsplit(self.diff.h_factor)),
        }
    }

    pub fn view_part_rect(&self, in_rect: Rect) -> FVec<Rect, 2> {
        let uv = self.view_part.uv_full();
        match self.diff.mode {
            DiffMode::Full => {
                let mut r = FVec::new();
                let size = vec2(in_rect.width() * uv.width(), in_rect.height() * uv.height());
                let center = pos2(
                    in_rect.left() + in_rect.width() * uv.center().x,
                    in_rect.top() + in_rect.height() * uv.center().y,
                );
                r.push(Rect::from_center_size(center, size));
                r
            }
            DiffMode::VSplit => {
                let mut r = FVec::new();
                let size = vec2(
                    in_rect.width() / 2.0 * uv.width(),
                    in_rect.height() * uv.height(),
                );
                let top = in_rect.top() + in_rect.height() * uv.center().y;
                let left = in_rect.width() / 2.0 * uv.center().x;
                let center_l = pos2(in_rect.left() + left, top);
                let center_r = pos2((in_rect.left() + in_rect.right()) / 2.0 + left, top);
                r.push(Rect::from_center_size(center_l, size));
                r.push(Rect::from_center_size(center_r, size));
                r
            }
            DiffMode::HSplit => {
                let mut r = FVec::new();
                let size = vec2(
                    in_rect.width() * uv.width(),
                    in_rect.height() / 2.0 * uv.height(),
                );
                let left = in_rect.left() + in_rect.width() * uv.center().x;
                let top = in_rect.height() / 2.0 * uv.center().y;
                let center_l = pos2(left, in_rect.top() + top);
                let center_r = pos2(left, (in_rect.top() + in_rect.bottom()) / 2.0 + top);
                r.push(Rect::from_center_size(center_l, size));
                r.push(Rect::from_center_size(center_r, size));
                r
            }
        }
    }

    pub fn display_size(&mut self, in_size: Vec2) -> FVec<Vec2, 2> {
        let width = if self.diff.mode == DiffMode::VSplit {
            self.width as f32 / 2.0
        } else {
            self.width as f32
        };
        let height = if self.diff.mode == DiffMode::HSplit {
            self.height as f32 / 2.0
        } else {
            self.height as f32
        };

        let w_scale = in_size.x / width;
        let h_scale = in_size.y / height;

        let scale = w_scale.min(h_scale).min(1.0);

        let w = width * scale;
        let h = height * scale;

        if self.view_part.scale.is_none() {
            self.view_part.scale = Some(scale);
        }

        match self.diff.mode {
            DiffMode::Full => {
                let mut r = FVec::new();
                r.push(vec2(w, h));
                r
            }
            DiffMode::VSplit => {
                let mut r = FVec::new();
                r.push(vec2(w * self.diff.v_factor, h));
                r.push(vec2(w * (1.0 - self.diff.v_factor), h));
                r
            }
            DiffMode::HSplit => {
                let mut r = FVec::new();
                r.push(vec2(w, h * self.diff.h_factor));
                r.push(vec2(w, h * (1.0 - self.diff.h_factor)));
                r
            }
        }
    }

    fn controls_ui_zoom(&mut self, ui: &mut Ui) -> f32 {
        let slider_min = 100.0 / Self::ZOOM_MAX;
        let slider_max = 100.0 / Self::ZOOM_MIN;
        let mut slider_val = 100.0 / self.view_part.scale.unwrap_or(1.0);
        ui.horizontal_top(|ui| {
            ui.label("Zoom:");
            if ui
                .add(
                    widgets::Slider::new(&mut slider_val, slider_min..=slider_max)
                        .logarithmic(true)
                        .fixed_decimals(2),
                )
                .changed()
            {
                self.view_part.set_scale(100.0 / slider_val);
            }
        })
        .response
        .rect
        .width()
    }

    fn controls_ui_split(&mut self, ui: &mut Ui, width: f32) {
        ui.radio_value(&mut self.diff.mode, DiffMode::Full, "Full image");
        ui.radio_value(&mut self.diff.mode, DiffMode::VSplit, "Vertical split");
        ui.spacing_mut().slider_width = width;
        ui.add_enabled(
            self.diff.mode == DiffMode::VSplit,
            widgets::Slider::new(&mut self.diff.v_factor, 0.0..=1.0).show_value(false),
        );
        ui.radio_value(&mut self.diff.mode, DiffMode::HSplit, "Horiizontal split");
        ui.add_enabled(
            self.diff.mode == DiffMode::HSplit,
            widgets::Slider::new(&mut self.diff.h_factor, 0.0..=1.0).show_value(false),
        );
    }

    fn preview_ui(&mut self, ui: &mut Ui, width: f32) -> Response {
        let height = self.height as f32 * (width / self.width as f32);
        let resp = ui.image(&self.texture, vec2(width, height));
        resp
    }

    fn controls_ui_preview(&mut self, ui: &mut Ui, width: f32) {
        let height = self.height as f32 * (width / self.width as f32);
        let resp = self.preview_ui(ui, width).interact(Sense::drag());
        let rect = resp.rect;
        let rects = self.view_part_rect(rect);
        for r in rects.iter() {
            ui.painter_at(rect).rect(
                *r,
                Rounding::none(),
                Color32::TRANSPARENT,
                Stroke::new(1.5, Color32::YELLOW),
            )
        }
        if let Some(p) = resp.interact_pointer_pos() {
            if rects.iter().any(|r| r.contains(p)) {
                if resp.dragged_by(PointerButton::Primary) {
                    let dd = resp.drag_delta();
                    let dd = Vec2::new(dd.x / width, dd.y / height);
                    self.view_part.set_center_diff(dd);
                }
            }
        }
        if let Some(p) = resp.hover_pos() {
            if rects.iter().any(|r| r.contains(p)) {
                let sd = ui.input().scroll_delta[1];
                if sd != 0.0 {
                    self.view_part.set_scale_diff(-0.001 * sd);
                }
            }
        }
    }

    pub fn controls_ui(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            let width = self.controls_ui_zoom(ui);
            self.controls_ui_split(ui, width);
            self.controls_ui_preview(ui, width);
            self.controls_ui_info(ui);
        });
    }

    fn controls_ui_info(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(format!("Size: {}x{}", self.width, self.height));
        });
    }

    pub fn thumbnail_ui(&self, ui: &mut Ui, is_current: bool, height: f32) -> Response {
        let width = self.width as f32 * (height / self.height as f32);
        let resp = ui.image(&self.texture, vec2(width, height));
        if is_current {
            let rect = resp.rect;
            ui.painter_at(rect).rect(
                rect,
                Rounding::none(),
                Color32::TRANSPARENT,
                Stroke::new(1.5, Color32::YELLOW),
            )
        }
        resp.interact(Sense::click())
    }

    pub fn main_ui(&mut self, ui: &mut Ui) -> Response {
        let sizes = self.display_size(ui.available_size_before_wrap());
        let uvs = self.uv();
        let resp = ui.with_layout(
            Layout::centered_and_justified(Direction::LeftToRight),
            |ui| {
                let img = SplittedImage::new(&self.texture, sizes, uvs, self.diff.mode);
                ui.add(img);
            },
        );
        let resp = resp.response.interact(Sense::drag());
        if let Some(_hover_pos) = resp.hover_pos() {
            let scroll_delta = ui.input().scroll_delta[1];
            if scroll_delta != 0.0 {
                self.view_part.set_scale_diff(-0.0001 * scroll_delta)
            }
        }
        if resp.dragged_by(PointerButton::Primary) {
            let dd = resp.drag_delta() * (-self.view_part.scale.unwrap_or(1.0) * 0.001);
            self.view_part.set_center_diff(dd);
        }
        resp
    }
}

#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
#[derive(Clone, Debug)]
pub struct SplittedImage {
    texture_id: TextureId,
    sizes: FVec<Vec2, 2>,
    uvs: FVec<Rect, 2>,
    bg_fill: Color32,
    tint: Color32,
    sense: Sense,
    mode: DiffMode,
}

impl SplittedImage {
    pub fn new(
        texture_id: impl Into<TextureId>,
        sizes: FVec<Vec2, 2>,
        uvs: FVec<Rect, 2>,
        mode: DiffMode,
    ) -> Self {
        Self {
            texture_id: texture_id.into(),
            sizes: sizes,
            uvs: uvs,
            bg_fill: Default::default(),
            tint: Color32::WHITE,
            sense: Sense::hover(),
            mode: mode,
        }
    }

    /// A solid color to put behind the image. Useful for transparent images.
    pub fn bg_fill(mut self, bg_fill: impl Into<Color32>) -> Self {
        self.bg_fill = bg_fill.into();
        self
    }

    /// Multiply image color with this. Default is WHITE (no tint).
    pub fn tint(mut self, tint: impl Into<Color32>) -> Self {
        self.tint = tint.into();
        self
    }

    /// Make the image respond to clicks and/or drags.
    ///
    /// Consider using [`ImageButton`] instead, for an on-hover effect.
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }
}

impl SplittedImage {
    pub fn size(&self) -> Vec2 {
        match self.mode {
            DiffMode::Full => self.sizes[0],
            DiffMode::VSplit => vec2(self.sizes[0].x + self.sizes[1].x, self.sizes[0].y),
            DiffMode::HSplit => vec2(self.sizes[0].x, self.sizes[0].y + self.sizes[1].y),
        }
    }

    pub fn paint_at(&self, ui: &mut Ui, rect: Rect) {
        if ui.is_rect_visible(rect) {
            use epaint::*;
            let Self {
                texture_id,
                sizes: _,
                uvs,
                bg_fill,
                tint,
                sense: _,
                mode: _,
            } = self;

            if *bg_fill != Default::default() {
                let mut mesh = Mesh::default();
                mesh.add_colored_rect(rect, *bg_fill);
                ui.painter().add(Shape::mesh(mesh));
            }

            {
                let rects = self.build_mesh_rects(rect);
                for (rect, uv) in rects.iter().zip(uvs) {
                    let mut mesh = Mesh::with_texture(*texture_id);
                    mesh.add_rect_with_uv(*rect, *uv, *tint);
                    ui.painter().add(Shape::mesh(mesh));
                }
            }
        }
    }

    fn build_mesh_rects(&self, rect: Rect) -> FVec<Rect, 2> {
        let mut result = FVec::new();
        match self.mode {
            DiffMode::Full => {
                result.push(rect);
            }
            DiffMode::VSplit => {
                let top = rect.top();
                let bottom = rect.bottom();
                let l_left = rect.left();
                let l_right = l_left + self.sizes[0].x;
                let r_left = l_left + self.sizes[0].x;
                let r_right = rect.right();
                result.push(Rect::from_min_max(pos2(l_left, top), pos2(l_right, bottom)));
                result.push(Rect::from_min_max(pos2(r_left, top), pos2(r_right, bottom)));
            }
            DiffMode::HSplit => {
                let left = rect.left();
                let right = rect.right();
                let t_top = rect.top();
                let t_bottom = t_top + self.sizes[0].y;
                let b_top = t_top + self.sizes[0].y;
                let b_bottom = rect.bottom();
                result.push(Rect::from_min_max(pos2(left, t_top), pos2(right, t_bottom)));
                result.push(Rect::from_min_max(pos2(left, b_top), pos2(right, b_bottom)));
            }
        }

        result
    }
}

impl Widget for SplittedImage {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(self.size(), self.sense);
        self.paint_at(ui, rect);
        response
    }
}
