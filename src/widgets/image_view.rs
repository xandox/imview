use crate::{DiffMode, ImageData, ImageUIState};
use arrayvec::ArrayVec;
use eframe::egui::*;

use crate::widgets::splited_image::SplittedImage;

pub struct ImageView<'a> {
    state: &'a mut ImageUIState,
    data: Option<&'a ImageData>,
}

impl<'a> ImageView<'a> {
    pub fn new(state: &'a mut ImageUIState, data: Option<&'a ImageData>) -> Self {
        Self { state, data }
    }

    fn need_half_width(&self) -> bool {
        self.state.diff_mode == DiffMode::VSplit || self.state.diff_mode == DiffMode::VColorDiff
    }

    fn need_half_height(&self) -> bool {
        self.state.diff_mode == DiffMode::HSplit || self.state.diff_mode == DiffMode::HColorDiff
    }

    fn calc_scale(&self, in_size: Vec2) -> f32 {
        let data = self.data.as_ref().unwrap();
        let width = data.width() * if self.need_half_width() { 0.5 } else { 1.0 };
        let height = data.height() * if self.need_half_height() { 0.5 } else { 1.0 };

        let w_scale = in_size.x / width;
        let h_scale = in_size.y / height;

        let scale = w_scale.min(h_scale).min(1.0);
        scale
    }

    fn display_size(&self, in_size: Vec2) -> ArrayVec<Vec2, 2> {
        let data = self.data.as_ref().unwrap();
        let width = data.width() * if self.need_half_width() { 0.5 } else { 1.0 };
        let height = data.height() * if self.need_half_height() { 0.5 } else { 1.0 };

        let scale = self.calc_scale(in_size);

        let w = width * scale;
        let h = height * scale;

        match self.state.diff_mode {
            DiffMode::Full | DiffMode::VColorDiff | DiffMode::HColorDiff => {
                let mut r = ArrayVec::new();
                r.push(vec2(w, h));
                r
            }
            DiffMode::VSplit => {
                let mut r = ArrayVec::new();
                r.push(vec2(w * self.state.vsplit_factor, h));
                r.push(vec2(w * (1.0 - self.state.vsplit_factor), h));
                r
            }
            DiffMode::HSplit => {
                let mut r = ArrayVec::new();
                r.push(vec2(w, h * self.state.hsplit_factor));
                r.push(vec2(w, h * (1.0 - self.state.hsplit_factor)));
                r
            }
        }
    }

    fn uvs(&self) -> ArrayVec<Rect, 2> {
        match self.state.diff_mode {
            DiffMode::Full | DiffMode::VColorDiff | DiffMode::HColorDiff => {
                let mut r = ArrayVec::new();
                r.push(self.state.uv_full());
                r
            }
            DiffMode::VSplit => ArrayVec::from(self.state.uv_vsplit(self.state.vsplit_factor)),
            DiffMode::HSplit => ArrayVec::from(self.state.uv_hsplit(self.state.hsplit_factor)),
        }
    }

    fn data_exist_ui(&mut self, ui: &mut Ui) {
        let data = self.data.as_ref().unwrap();
        let av_size = ui.available_size_before_wrap();
        self.state.set_scale_if_none(self.calc_scale(av_size));
        let sizes = self.display_size(av_size);
        let uvs = self.uvs();
        let resp = ui.with_layout(
            Layout::centered_and_justified(Direction::LeftToRight),
            |ui| {
                let img = SplittedImage::new(
                    data.texture_handle(self.state.diff_mode),
                    sizes,
                    uvs,
                    self.state.diff_mode,
                );
                ui.add(img);
            },
        );
        let resp = resp.response.interact(Sense::drag());
        if let Some(_hover_pos) = resp.hover_pos() {
            let scroll_delta = ui.input().scroll_delta[1];
            if scroll_delta != 0.0 {
                self.state.set_scale_diff(-0.0001 * scroll_delta)
            }
        }
        if resp.dragged_by(PointerButton::Primary) {
            let dd = resp.drag_delta() * (-self.state.scale() * 0.001);
            self.state.set_center_diff(dd);
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        match self.data {
            None => (),
            Some(_) => self.data_exist_ui(ui),
        }
    }
}
