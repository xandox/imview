use image::io::Reader as ImageReader;
pub use image::RgbaImage;
use log::error;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug)]
pub struct LoadedImage {
    pub image: Option<image::RgbaImage>,
    pub path: PathBuf,
}

impl LoadedImage {
    fn new(image: RgbaImage, path: PathBuf) -> Self {
        Self {
            image: Some(image),
            path: path,
        }
    }
    fn error(path: PathBuf) -> Self {
        Self {
            image: None,
            path: path,
        }
    }
}

fn load_image_impl(path: &Path) -> Option<image::RgbaImage> {
    let reader = ImageReader::open(path);
    if reader.is_err() {
        error!("Failed to open image: {:?}", path);
        return None;
    }
    let image = reader.unwrap().decode();
    if image.is_err() {
        error!("Failed to decode image: {:?}", path);
        return None;
    }
    Some(image.unwrap().to_rgba8())
}

fn spawn_path_loader<UpdateFunc>(
    path_recv: Receiver<PathBuf>,
    img_send: Sender<LoadedImage>,
    update: UpdateFunc,
) where
    UpdateFunc: Fn() + Send + 'static,
{
    std::thread::spawn(move || {
        while let Ok(path) = path_recv.recv() {
            let image = load_image_impl(&path);
            let image = if let Some(image) = image {
                LoadedImage::new(image, path)
            } else {
                LoadedImage::error(path)
            };
            if let Ok(()) = img_send.send(image) {
                update();
            } else {
                break;
            }
        }
    });
}

pub fn spawn_loader<UpdateFunc>(
    filenames: Option<Vec<PathBuf>>,
    directory: Option<PathBuf>,
    updater: UpdateFunc,
) -> std::io::Result<Receiver<LoadedImage>>
where
    UpdateFunc: Fn() + Send + 'static,
{
    let (path_send, path_recv) = channel();
    let (img_send, img_recv) = channel();
    spawn_path_loader(path_recv, img_send, updater);
    std::thread::spawn(move || {
        if let Some(dir) = directory.as_ref() {
            let mut entries = std::fs::read_dir(dir.clone()).unwrap();
            while let Some(entry) = entries.next() {
                let path = entry.unwrap().path();
                if path.is_file() {
                    path_send.send(path).unwrap();
                }
            }
        }
        if let Some(fns) = filenames.as_ref() {
            for path in fns {
                path_send.send(path.clone()).unwrap();
            }
        }
    });
    Ok(img_recv)
}
