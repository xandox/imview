use eframe::egui::*;

use crate::ImageData;

pub struct Thumbnail<'a> {
    image: Option<&'a ImageData>,
    size: f32,
    is_current: bool,
}

impl<'a> Thumbnail<'a> {
    pub fn new(image: Option<&'a ImageData>, size: f32, is_current: bool) -> Self {
        Self {
            image,
            size,
            is_current,
        }
    }
}

impl Widget for Thumbnail<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, resp) = ui.allocate_exact_size(vec2(self.size, self.size), Sense::click());
        if ui.is_rect_visible(rect) {
            ui.ctx().request_repaint();
            ui.allocate_ui_at_rect(rect, |ui| {
                let bg_color = if self.is_current {
                    ui.visuals().extreme_bg_color
                } else {
                    ui.visuals().faint_bg_color
                };
                ui.painter_at(rect)
                    .rect(rect, Rounding::none(), bg_color, Stroke::none());
                match self.image {
                    None => {
                        ui.centered_and_justified(|ui| ui.add(widgets::Spinner::new()));
                    }
                    Some(data) => {
                        if data.error_msg.is_some() {
                            ui.centered_and_justified(|ui| {
                                let text = RichText::new("Loading error").color(Color32::RED);
                                ui.label(text);
                            });
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.image(data.color_texture_handle(), data.size())
                            });
                        }
                    }
                }
            });
        }

        resp
    }
}
