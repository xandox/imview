mod filesystem;
mod image_data;
mod image_ui_state;
mod utils;
mod widgets;

use image_data::ImageData;
use image_ui_state::{DiffMode, ImageUIState};

use cached::{Cached, SizedCache};
use clap::Parser;
use eframe::egui::{self, Context};
use egui_extras::{Size, StripBuilder};
use filesystem::{FileSystem, FileSystemEvent};
use log::{trace, warn};
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use std::path::PathBuf;
use widgets::{ImageControls, ImageView, Thumbnail};

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct CliArguments {
    #[clap(min_values(1))]
    path: Vec<PathBuf>,
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
            let egui_ctx = cc.egui_ctx.clone();
            let fs = FileSystem::start(args.path, move || egui_ctx.request_repaint());
            let app = IMViewApp::new(fs.unwrap(), cc.egui_ctx.clone());
            Box::new(app)
        }),
    );
}

struct IMViewApp {
    cc: Context,
    file_system: FileSystem,
    current_image: Option<PathBuf>,
    image_files: Vec<PathBuf>,
    image_states: HashMap<PathBuf, ImageUIState>,
    thumbnails_cache: HashMap<PathBuf, ImageData>,
    full_images_cache: SizedCache<PathBuf, ImageData>,
}

const THUMBNAIL_SIZE: u32 = 150;

impl IMViewApp {
    fn new(fs: FileSystem, cc: Context) -> Self {
        Self {
            cc: cc,
            file_system: fs,
            current_image: None,
            image_files: Vec::new(),
            image_states: HashMap::new(),
            thumbnails_cache: HashMap::new(),
            full_images_cache: SizedCache::with_size(10),
        }
    }

    fn process_fs_events(&mut self) {
        let mut was_file_events = false;
        while let Ok(event) = self.file_system.receiver.try_recv() {
            match event {
                FileSystemEvent::FileEvent(event) => {
                    was_file_events = true;
                    self.process_file_event(event);
                }
                FileSystemEvent::OperationEvent(event) => self.process_operation_event(event),
            }
        }
        if was_file_events {
            self.image_files.sort();
            if self.current_image.is_none() && self.image_files.len() >= 1 {
                self.current_image = Some(self.image_files[0].clone());
                self.file_system.read_file(&self.image_files[0])
            }
            if self.image_files.len() == 0 {
                self.current_image = None;
            }
        }
    }

    fn process_file_event(&mut self, event: filesystem::FileEvent) {
        match event {
            filesystem::FileEvent::Added(path) => {
                trace!("File added: {:?}", path);
                self.add_file(path);
            }
            filesystem::FileEvent::Removed(path) => {
                trace!("File removed: {:?}", path);
                self.remove_file(path);
            }
            filesystem::FileEvent::Modified(path) => {
                trace!("File modified: {:?}", path);
                self.invalidate_file_data(path);
            }
            filesystem::FileEvent::Renamed(old_path, new_path) => {
                trace!("File renamed: {:?} -> {:?}", old_path, new_path);
                self.rename_file(old_path, new_path);
            }
        }
    }

    fn add_file(&mut self, path: PathBuf) {
        self.image_files.push(path.clone());
        self.image_states.insert(path.clone(), ImageUIState::new());
        self.file_system.read_thumbnail(&path, THUMBNAIL_SIZE)
    }

    fn remove_file(&mut self, path: PathBuf) {
        self.image_files.retain(|p| p != &path);
        self.image_states.remove(&path);
        self.thumbnails_cache.remove(&path);
        self.full_images_cache.cache_remove(&path);
    }

    fn invalidate_file_data(&mut self, path: PathBuf) {
        self.thumbnails_cache.remove(&path);
        self.full_images_cache.cache_remove(&path);
        self.file_system.read_thumbnail(&path, THUMBNAIL_SIZE);
    }

    fn rename_file(&mut self, old_path: PathBuf, new_path: PathBuf) {
        let index = self
            .image_files
            .iter()
            .position(|p| p == &old_path)
            .unwrap();
        self.image_files[index] = new_path.clone();
        let state = self.image_states.remove(&old_path).unwrap();
        self.image_states.insert(new_path.clone(), state);
        if let Some(data) = self.thumbnails_cache.remove(&old_path) {
            self.thumbnails_cache.insert(new_path.clone(), data);
        }
        if let Some(data) = self.full_images_cache.cache_remove(&old_path) {
            self.full_images_cache.cache_set(new_path.clone(), data);
        }
    }

    fn process_operation_event(&mut self, event: filesystem::OperationEvent) {
        match event {
            filesystem::OperationEvent::ThumbnailLoaded((path, img)) => {
                if img.is_err() {
                    let err = img.err().unwrap();
                    warn!("Failed to load thumbnail for {}: {}", path.display(), err);
                    let data = ImageData::error(&err);
                    self.thumbnails_cache.insert(path, data);
                } else {
                    trace!("Thumbnail loaded: {}", path.display());
                    let img = img.unwrap();
                    let data = ImageData::thumbnail(&path, img, &self.cc);
                    self.thumbnails_cache.insert(path, data);
                }
            }
            filesystem::OperationEvent::ImageLoaded((path, img)) => {
                if img.is_err() {
                    let err = img.err().unwrap();
                    warn!("Failed to load image for {}: {}", path.display(), err);
                    let data = ImageData::error(&err);
                    self.full_images_cache.cache_set(path, data);
                } else {
                    let img = img.unwrap();
                    trace!("Image loaded: {}", path.display());
                    let data = ImageData::full_image(&path, img, &self.cc);
                    self.full_images_cache.cache_set(path, data);
                }
            }
        }
    }
}

impl eframe::App for IMViewApp {
    fn on_exit_event(&mut self) -> bool {
        trace!("Closing application");
        self.file_system.shutdown();
        true
    }
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.process_fs_events();

        if let Some(ci) = self.current_image.clone() {
            let title = format!("iMView - {}", ci.display());
            if self.full_images_cache.cache_get(&ci).is_none() {
                self.file_system.read_file(&ci);
            }
            frame.set_window_title(&title);
            egui::CentralPanel::default().show(ctx, |ui| {
                let thumbs_height = ui.spacing().item_spacing.y
                    + ui.spacing().scroll_bar_width
                    + THUMBNAIL_SIZE as f32;
                StripBuilder::new(ui)
                    .size(Size::remainder().at_least(100.0)) // top cell
                    .size(Size::exact(thumbs_height)) // bottom cell
                    .vertical(|mut strip| {
                        strip.strip(|builder| {
                            builder
                                .size(Size::exact(300.0))
                                .size(Size::remainder())
                                .horizontal(|mut strip| {
                                    strip.cell(|ui| {
                                        ImageControls::new(
                                            self.image_states.get_mut(&ci).unwrap(),
                                            self.full_images_cache.cache_get_mut(&ci),
                                        )
                                        .ui(ui);
                                    });
                                    strip.cell(|ui| {
                                        ImageView::new(
                                            self.image_states.get_mut(&ci).unwrap(),
                                            self.full_images_cache.cache_get(&ci),
                                        )
                                        .ui(ui);
                                    });
                                });
                        });
                        strip.cell(|ui| {
                            egui::containers::ScrollArea::horizontal().show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    for img in self.image_files.iter() {
                                        let data = self.thumbnails_cache.get(img);
                                        let is_current = &ci == img;
                                        let thumb =
                                            Thumbnail::new(data, THUMBNAIL_SIZE as _, is_current);
                                        if ui.add(thumb).clicked() {
                                            self.current_image = Some(img.clone());
                                            self.file_system.read_file(&img);
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
    }
}
