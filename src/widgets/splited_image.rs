use crate::DiffMode;
use arrayvec::ArrayVec;
use eframe::egui::*;

#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
#[derive(Clone, Debug)]
pub struct SplittedImage {
    texture_id: TextureId,
    sizes: ArrayVec<Vec2, 2>,
    uvs: ArrayVec<Rect, 2>,
    bg_fill: Color32,
    tint: Color32,
    sense: Sense,
    mode: DiffMode,
}

impl SplittedImage {
    pub fn new(
        texture_id: impl Into<TextureId>,
        sizes: ArrayVec<Vec2, 2>,
        uvs: ArrayVec<Rect, 2>,
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
    #[allow(dead_code)]
    pub fn bg_fill(mut self, bg_fill: impl Into<Color32>) -> Self {
        self.bg_fill = bg_fill.into();
        self
    }

    /// Multiply image color with this. Default is WHITE (no tint).
    #[allow(dead_code)]
    pub fn tint(mut self, tint: impl Into<Color32>) -> Self {
        self.tint = tint.into();
        self
    }

    /// Make the image respond to clicks and/or drags.
    ///
    /// Consider using [`ImageButton`] instead, for an on-hover effect.
    #[allow(dead_code)]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }
}

impl SplittedImage {
    pub fn size(&self) -> Vec2 {
        match self.mode {
            DiffMode::Full | DiffMode::VColorDiff | DiffMode::HColorDiff => self.sizes[0],
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

    fn build_mesh_rects(&self, rect: Rect) -> ArrayVec<Rect, 2> {
        let mut result = ArrayVec::new();
        match self.mode {
            DiffMode::Full | DiffMode::HColorDiff | DiffMode::VColorDiff => {
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
