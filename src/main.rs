mod image_ui;
mod loader;

use clap::Parser;
use eframe::egui;
use egui_extras::{Size, StripBuilder};
use image_ui::ImageUI;
use loader::{spawn_loader, LoadedImage};
use log::debug;
use simple_logger::SimpleLogger;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct CliArguments {
    #[clap(min_values(1))]
    path: Vec<PathBuf>,
}

fn run_background_loader(args: CliArguments, ctx: egui::Context) -> Receiver<LoadedImage> {
    let paths = args.path;
    let updater = move || ctx.request_repaint();
    if paths.len() == 0 {
        spawn_loader(None, None, updater).unwrap()
    } else if paths.len() == 1 && paths[0].is_dir() {
        spawn_loader(None, Some(paths[0].clone()), updater).unwrap()
    } else {
        spawn_loader(Some(paths), None, updater).unwrap()
    }
}

fn main() {
    SimpleLogger::new().init().unwrap();
    let args = CliArguments::parse();
    let mut options = eframe::NativeOptions::default();
    options.initial_window_size = Some(egui::Vec2::new(800 as _, 600 as _));
    options.maximized = true;
    eframe::run_native(
        "iMView",
        options,
        Box::new(|cc| {
            let img_recv = run_background_loader(args, cc.egui_ctx.clone());
            let app = IMViewApp::new(img_recv);
            Box::new(app)
        }),
    );
}

struct IMViewApp {
    images: Vec<ImageUI>,
    current_image: Option<usize>,
    img_recv: Receiver<LoadedImage>,
}

impl IMViewApp {
    fn new(img_recv: Receiver<LoadedImage>) -> Self {
        Self {
            images: Vec::new(),
            current_image: None,
            img_recv,
        }
    }
}

impl eframe::App for IMViewApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(ci) = self.current_image {
            let title = format!("iMView - {}", self.images[ci].filename.display());
            frame.set_window_title(&title);
            egui::CentralPanel::default().show(ctx, |ui| {
                StripBuilder::new(ui)
                    .size(Size::remainder().at_least(100.0)) // top cell
                    .size(Size::exact(150.0)) // bottom cell
                    .vertical(|mut strip| {
                        strip.strip(|builder| {
                            builder
                                .size(Size::exact(300.0))
                                .size(Size::remainder())
                                .horizontal(|mut strip| {
                                    strip.cell(|ui| {
                                        self.images[ci].controls_ui(ui);
                                    });
                                    strip.cell(|ui| {
                                        self.images[ci].main_ui(ui);
                                    });
                                });
                        });
                        strip.cell(|ui| {
                            let height = ui.available_size_before_wrap().y;
                            egui::containers::ScrollArea::horizontal().show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    for (i, img) in self.images.iter().enumerate() {
                                        let is_current =
                                            self.current_image.map_or(false, |ci| ci == i);
                                        if img.thumbnail_ui(ui, is_current, height).clicked() {
                                            self.current_image = Some(i);
                                        }
                                    }
                                });
                            });
                        });
                    });
            });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| ui.label("Loading images..."));
        }

        while let Ok(ld_img) = self.img_recv.try_recv() {
            debug!("Got image: {:?}", ld_img.path);
            if ld_img.image.is_some() {
                self.images.push(ImageUI::new(
                    ld_img.path.clone(),
                    ld_img.image.unwrap(),
                    ctx,
                ));
                if self.current_image.is_none() {
                    self.current_image = Some(self.images.len() - 1);
                }
            }
        }
    }
}
