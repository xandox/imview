use eframe::egui::*;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum DiffMode {
    Full,
    VSplit,
    VColorDiff,
    HSplit,
    HColorDiff,
}
pub struct ImageUIState {
    pub diff_mode: DiffMode,
    pub color_diff_vsplite_gamma: f32,
    pub color_diff_hsplite_gamma: f32,
    pub vsplit_factor: f32,
    pub hsplit_factor: f32,
    scale: Option<f32>,
    view_center: Pos2,
}

impl ImageUIState {
    pub const ZOOM_MIN: f32 = 0.01;
    pub const ZOOM_MAX: f32 = 1.0;

    pub fn new() -> Self {
        Self {
            diff_mode: DiffMode::Full,
            color_diff_vsplite_gamma: 2.2,
            color_diff_hsplite_gamma: 2.2,
            scale: None,
            vsplit_factor: 0.5,
            hsplit_factor: 0.5,
            view_center: Pos2::new(0.5, 0.5),
        }
    }

    pub fn scale(&self) -> f32 {
        self.scale.unwrap_or(1.0)
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.scale = Some(scale);
        self.fix_bounds()
    }

    pub fn set_scale_if_none(&mut self, scale: f32) {
        if self.scale.is_none() {
            self.set_scale(scale)
        }
    }

    pub fn set_scale_diff(&mut self, scale_diff: f32) {
        self.scale = Some(self.scale.unwrap_or(1.0) + scale_diff);
        self.fix_bounds();
    }

    pub fn set_center_diff(&mut self, center_diff: Vec2) {
        self.view_center += center_diff;
        self.fix_bounds();
    }

    fn fix_bounds(&mut self) {
        if self.scale.is_some() {
            self.scale = Some(self.scale.unwrap().clamp(Self::ZOOM_MIN, Self::ZOOM_MAX));
        }
        let s_by_2 = self.scale.unwrap_or(1.0) / 2.0;
        if self.left() < 0.0 {
            self.view_center.x = s_by_2;
        }
        if self.right() > 1.0 {
            self.view_center.x = 1.0 - s_by_2;
        }
        if self.top() < 0.0 {
            self.view_center.y = s_by_2;
        }
        if self.bottom() > 1.0 {
            self.view_center.y = 1.0 - s_by_2;
        }
    }

    fn left(&self) -> f32 {
        self.view_center.x - self.scale() / 2.0
    }

    fn right(&self) -> f32 {
        self.view_center.x + self.scale() / 2.0
    }

    fn top(&self) -> f32 {
        self.view_center.y - self.scale() / 2.0
    }

    fn bottom(&self) -> f32 {
        self.view_center.y + self.scale() / 2.0
    }

    pub fn uv_full(&self) -> Rect {
        let r = Rect::from_min_max(
            pos2(self.left(), self.top()),
            pos2(self.right(), self.bottom()),
        );
        r
    }

    pub fn uv_vsplit(&self, ratio: f32) -> [Rect; 2] {
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

    pub fn uv_hsplit(&self, ratio: f32) -> [Rect; 2] {
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
