use crate::{DiffMode, ImageData, ImageUIState};
use arrayvec::ArrayVec;
use eframe::egui::*;

pub struct ImageControls<'a> {
    state: &'a mut ImageUIState,
    data: Option<&'a mut ImageData>,
}

impl<'a> ImageControls<'a> {
    pub fn new(state: &'a mut ImageUIState, data: Option<&'a mut ImageData>) -> Self {
        Self { state, data }
    }

    fn zoom_ui(&mut self, ui: &mut Ui) {
        let slider_min = 100.0 / ImageUIState::ZOOM_MAX;
        let slider_max = 100.0 / ImageUIState::ZOOM_MIN;
        let mut slider_val = 100.0 / self.state.scale();
        ui.horizontal_top(|ui| {
            ui.label("Zoom: ");
            if ui
                .add(
                    widgets::Slider::new(&mut slider_val, slider_min..=slider_max)
                        .logarithmic(true)
                        .fixed_decimals(2),
                )
                .changed()
            {
                self.state.set_scale(100.0 / slider_val);
            }
        });
    }

    fn diff_ui(&mut self, ui: &mut Ui) {
        let data = self.data.as_mut().unwrap();
        if ui
            .radio_value(&mut self.state.diff_mode, DiffMode::Full, "Full image")
            .changed()
        {
            data.switch_to_color_image(ui.ctx());
        }

        if ui
            .radio_value(
                &mut self.state.diff_mode,
                DiffMode::VSplit,
                "Vertical split",
            )
            .changed()
        {
            data.switch_to_color_image(ui.ctx());
        }

        ui.horizontal(|ui| {
            ui.label("Part: ");
            if ui
                .add_enabled(
                    self.state.diff_mode == DiffMode::VSplit,
                    widgets::Slider::new(&mut self.state.vsplit_factor, 0.0..=1.0)
                        .show_value(false),
                )
                .changed()
            {
                data.switch_to_color_image(ui.ctx());
            }
        });

        if ui
            .radio_value(
                &mut self.state.diff_mode,
                DiffMode::VColorDiff,
                "Color difference vertical",
            )
            .changed()
        {
            data.switch_to_vertical_color_diff(ui.ctx(), self.state.color_diff_vsplite_gamma);
        }
        ui.horizontal(|ui| {
            ui.label("Gamma:");
            if ui
                .add_enabled(
                    self.state.diff_mode == DiffMode::VColorDiff,
                    widgets::Slider::new(&mut self.state.color_diff_vsplite_gamma, 1.0..=5.0),
                )
                .changed()
            {
                data.switch_to_vertical_color_diff(ui.ctx(), self.state.color_diff_vsplite_gamma);
            };
        });
        if ui
            .radio_value(
                &mut self.state.diff_mode,
                DiffMode::HSplit,
                "Horiizontal split",
            )
            .changed()
        {
            data.switch_to_color_image(ui.ctx());
        }

        ui.horizontal(|ui| {
            ui.label("Part: ");
            if ui
                .add_enabled(
                    self.state.diff_mode == DiffMode::HSplit,
                    widgets::Slider::new(&mut self.state.hsplit_factor, 0.0..=1.0)
                        .show_value(false),
                )
                .changed()
            {
                data.switch_to_color_image(ui.ctx());
            }
        });
        if ui
            .radio_value(
                &mut self.state.diff_mode,
                DiffMode::HColorDiff,
                "Color difference horizontal",
            )
            .changed()
        {
            data.switch_to_horizontal_color_diff(ui.ctx(), self.state.color_diff_hsplite_gamma);
        }
        ui.horizontal(|ui| {
            ui.label("Gamma:");
            if ui
                .add_enabled(
                    self.state.diff_mode == DiffMode::HColorDiff,
                    widgets::Slider::new(&mut self.state.color_diff_hsplite_gamma, 1.0..=5.0),
                )
                .changed()
            {
                data.switch_to_horizontal_color_diff(ui.ctx(), self.state.color_diff_hsplite_gamma);
            }
        });
    }

    fn view_part_rect(&self, in_rect: Rect) -> ArrayVec<Rect, 2> {
        let uv = self.state.uv_full();
        match self.state.diff_mode {
            DiffMode::Full => {
                let mut r = ArrayVec::new();
                let size = vec2(in_rect.width() * uv.width(), in_rect.height() * uv.height());
                let center = pos2(
                    in_rect.left() + in_rect.width() * uv.center().x,
                    in_rect.top() + in_rect.height() * uv.center().y,
                );
                r.push(Rect::from_center_size(center, size));
                r
            }
            DiffMode::VSplit | DiffMode::VColorDiff => {
                let mut r = ArrayVec::new();
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
            DiffMode::HSplit | DiffMode::HColorDiff => {
                let mut r = ArrayVec::new();
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

    fn preview_ui(&mut self, ui: &mut Ui) {
        let width = ui.available_size_before_wrap().x;
        let data = self.data.as_mut().unwrap();
        let height = data.height() * (width / data.width());
        let resp = ui
            .image(data.color_texture_handle(), vec2(width, height))
            .interact(Sense::drag());
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
                    self.state.set_center_diff(dd);
                }
            }
        }
        if let Some(p) = resp.hover_pos() {
            if rects.iter().any(|r| r.contains(p)) {
                let sd = ui.input().scroll_delta[1];
                if sd != 0.0 {
                    self.state.set_scale_diff(-0.001 * sd);
                }
            }
        }
    }

    fn info_ui(&mut self, ui: &mut Ui) {
        let (w, h) = match self.data.as_ref() {
            Some(d) => (format!("{}", d.width()), format!("{}", d.height())),
            None => ("-".into(), "-".into()),
        };
        ui.horizontal(|ui| {
            ui.label(format!("Size: {}x{}", w, h));
        });
    }

    fn data_load_error(&self, error: &str, ui: &mut Ui) {
        let text = format!("Error loading data: {}", error);
        ui.label(text);
    }

    fn data_is_loading(&self, ui: &mut Ui) {
        ui.label("Loading data...");
        ui.spinner();
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| match self.data.as_ref() {
            None => self.data_is_loading(ui),
            Some(d) => {
                if let Some(em) = d.error_msg.as_ref() {
                    self.data_load_error(em, ui);
                } else {
                    self.zoom_ui(ui);
                    self.diff_ui(ui);
                    self.preview_ui(ui);
                    self.info_ui(ui);
                }
            }
        });
    }
}
