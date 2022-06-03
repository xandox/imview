use crossbeam::channel::{never, unbounded, Receiver, Select, Sender};
use image::io::Reader as ImageReader;
use image::RgbaImage;
use log::{error, trace};
use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel as std_channel, Receiver as StdReceiver};
use std::sync::{atomic::AtomicBool, Arc};
use std::time::Duration;

struct Notify {
    watcher: RecommendedWatcher,
    reciver: StdReceiver<DebouncedEvent>,
}
pub struct FileSystem {
    pub receiver: Receiver<FileSystemEvent>,
    op_sender: Sender<InternalFSEvent>,
    thumbs_thread_pool: ThreadPool,
    image_thread_pool: ThreadPool,
    shutdown_flag: Arc<AtomicBool>,

    #[allow(dead_code)]
    notify_watcher: Option<RecommendedWatcher>,
}

fn map_err_notify(err: notify::Error) -> std::io::Error {
    match err {
        notify::Error::Io(err) => err,
        notify::Error::PathNotFound => {
            std::io::Error::new(std::io::ErrorKind::Other, "Path not found")
        }
        notify::Error::WatchNotFound => {
            std::io::Error::new(std::io::ErrorKind::Other, "Watch not found")
        }
        notify::Error::Generic(err) => std::io::Error::new(std::io::ErrorKind::Other, err),
    }
}

fn is_image(path: &Path) -> bool {
    image::ImageFormat::from_path(path)
        .map(|f| f.can_read())
        .unwrap_or(false)
}

pub enum FileEvent {
    Added(PathBuf),
    Removed(PathBuf),
    Modified(PathBuf),
    Renamed(PathBuf, PathBuf),
}

pub enum OperationEvent {
    ThumbnailLoaded((PathBuf, std::io::Result<RgbaImage>)),
    ImageLoaded((PathBuf, std::io::Result<RgbaImage>)),
}

enum InternalFSEvent {
    Notify(DebouncedEvent),
    Op(OperationEvent),
}

impl InternalFSEvent {
    fn image_loaded(path: PathBuf, image: std::io::Result<RgbaImage>) -> Self {
        InternalFSEvent::Op(OperationEvent::ImageLoaded((path, image)))
    }
    fn thumbnail_loaded(path: PathBuf, image: std::io::Result<RgbaImage>) -> Self {
        InternalFSEvent::Op(OperationEvent::ThumbnailLoaded((path, image)))
    }
}

pub enum FileSystemEvent {
    FileEvent(FileEvent),
    OperationEvent(OperationEvent),
}

impl FileSystem {
    pub fn start<F>(paths: Vec<PathBuf>, notifier: F) -> std::io::Result<Self>
    where
        F: Fn() + Send + 'static,
    {
        let (fs_sender, fs_receiver) = unbounded();
        let fs_sender_cl = fs_sender.clone();
        let (op_sender, op_receiver) = unbounded();
        let (root, files) = Self::select_root_and_files(&paths)?;
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let notify = if root.is_some() {
            trace!(
                "Start watching directory: {}",
                root.as_ref().unwrap().display()
            );
            Some(Self::start_notify(root.as_ref().unwrap())?)
        } else {
            None
        };

        let (notify_reciver, notify_watcher) = if let Some(notify) = notify {
            (Some(notify.reciver), Some(notify.watcher))
        } else {
            (None, None)
        };

        let notify_reciver = if let Some(nr) = notify_reciver {
            let (s, r) = unbounded();
            let sfc = Arc::clone(&shutdown_flag);
            std::thread::spawn(move || loop {
                match nr.recv() {
                    Err(e) => {
                        if !sfc.load(std::sync::atomic::Ordering::Acquire) {
                            error!("Notify watcher trhead ended by reason: {}", e);
                        }
                        break;
                    }
                    Ok(event) => match s.send(InternalFSEvent::Notify(event)) {
                        Ok(_) => (),
                        Err(err) => {
                            if !sfc.load(std::sync::atomic::Ordering::Acquire) {
                                error!("Failed to send event to filesystem thread: {}", err);
                            }
                            break;
                        }
                    },
                }
            });
            r
        } else {
            never()
        };

        let thumbs_thread_pool = ThreadPoolBuilder::new()
            .num_threads(num_cpus::get().min(4))
            .build()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let image_thread_pool = ThreadPoolBuilder::new()
            .num_threads(num_cpus::get().min(4))
            .build()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        {
            let sfc = Arc::clone(&shutdown_flag);
            std::thread::spawn(move || {
                let mut sel = Select::new();
                sel.recv(&notify_reciver);
                sel.recv(&op_receiver);
                let rs = [&notify_reciver, &op_receiver];
                loop {
                    let idx = sel.ready();
                    let res = rs[idx].try_recv();

                    if let Err(e) = res {
                        if e.is_empty() {
                            continue;
                        } else {
                            if !sfc.load(std::sync::atomic::Ordering::Acquire) {
                                error!("Internal receiver thread finished with error: {}", e);
                            }
                            break;
                        }
                    }
                    let res = match res.unwrap() {
                        InternalFSEvent::Notify(event) => {
                            Self::process_notify_event(event, &fs_sender)
                        }
                        InternalFSEvent::Op(event) => {
                            Self::process_operation_event(event, &fs_sender)
                        }
                    };
                    notifier();

                    if let Err(_) = res {
                        break;
                    }
                }
            });
        }

        for file in files {
            fs_sender_cl
                .send(FileSystemEvent::FileEvent(FileEvent::Added(file)))
                .unwrap();
        }

        Ok(Self {
            receiver: fs_receiver,
            op_sender: op_sender,
            thumbs_thread_pool: thumbs_thread_pool,
            image_thread_pool: image_thread_pool,
            notify_watcher: notify_watcher,
            shutdown_flag: shutdown_flag,
        })
    }

    pub fn read_file(&self, path: &Path) {
        let sender = self.op_sender.clone();
        let path = path.to_path_buf();
        self.image_thread_pool.spawn(move || {
            let res = ImageReader::open(&path).and_then(|r| {
                r.decode()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                    .map(|i| i.to_rgba8())
            });
            match sender.send(InternalFSEvent::image_loaded(path, res)) {
                Ok(_) => (),
                Err(e) => error!("Can't send image to main thread: {}", e),
            }
        });
    }

    pub fn shutdown(&self) {
        self.shutdown_flag
            .store(true, std::sync::atomic::Ordering::Release);
    }

    fn to_thumbnail(img: RgbaImage, size: u32) -> RgbaImage {
        let (w, h) = img.dimensions();
        let ws = size as f32 / w as f32;
        let hs = size as f32 / h as f32;
        let s = ws.min(hs);

        let w = (w as f32 * s).floor() as u32;
        let h = (h as f32 * s).floor() as u32;

        image::imageops::thumbnail(&img, w, h)
    }

    pub fn read_thumbnail(&self, path: &Path, size: u32) {
        let path = path.to_path_buf();
        let sender = self.op_sender.clone();
        self.thumbs_thread_pool.spawn(move || {
            let res = ImageReader::open(&path).and_then(|r| {
                r.decode()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                    .map(|i| Self::to_thumbnail(i.to_rgba8(), size))
            });
            match sender.send(InternalFSEvent::thumbnail_loaded(path, res)) {
                Ok(_) => (),
                Err(err) => error!("Can't send thumbnail to main thread: {}", err),
            }
        });
    }

    fn process_notify_event(
        event: DebouncedEvent,
        sender: &Sender<FileSystemEvent>,
    ) -> Result<(), crossbeam::channel::SendError<FileSystemEvent>> {
        let event = match event {
            DebouncedEvent::Create(path) => {
                if is_image(&path) {
                    Some(FileEvent::Added(path))
                } else {
                    None
                }
            }
            DebouncedEvent::Write(path) => {
                if is_image(&path) {
                    Some(FileEvent::Modified(path))
                } else {
                    None
                }
            }
            DebouncedEvent::Remove(path) => Some(FileEvent::Removed(path)),
            DebouncedEvent::Rename(old_path, new_path) => {
                Some(FileEvent::Renamed(old_path, new_path))
            }
            _ => None,
        };
        if let Some(event) = event {
            sender.send(FileSystemEvent::FileEvent(event))
        } else {
            Ok(())
        }
    }

    fn process_operation_event(
        event: OperationEvent,
        sender: &Sender<FileSystemEvent>,
    ) -> Result<(), crossbeam::channel::SendError<FileSystemEvent>> {
        sender.send(FileSystemEvent::OperationEvent(event))
    }

    fn start_notify(dir: &PathBuf) -> std::io::Result<Notify> {
        let (tx, rx) = std_channel();
        let mut watcher = watcher(tx, Duration::from_secs(10)).map_err(map_err_notify)?;
        watcher
            .watch(dir, RecursiveMode::NonRecursive)
            .map_err(map_err_notify)?;

        Ok(Notify {
            watcher,
            reciver: rx,
        })
    }

    fn drain_files_dirs(paths: Vec<PathBuf>) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let mut files = Vec::with_capacity(paths.len());
        let mut dirs = Vec::with_capacity(paths.len());
        for path in paths {
            if path.is_file() {
                files.push(path);
            } else if path.is_dir() {
                dirs.push(path);
            }
        }
        (files, dirs)
    }

    fn collect_files(dir: &PathBuf) -> std::io::Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let entries = std::fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path().canonicalize()?;
            if path.is_file() && is_image(&path) {
                files.push(path);
            }
        }
        Ok(files)
    }

    fn select_root_and_files(
        paths: &Vec<PathBuf>,
    ) -> std::io::Result<(Option<PathBuf>, HashSet<PathBuf>)> {
        if paths.len() == 0 {
            return Ok((None, HashSet::new()));
        }

        let (files, dirs) = Self::drain_files_dirs(
            paths
                .iter()
                .map(|p| p.canonicalize())
                .collect::<Result<Vec<_>, _>>()?,
        );

        let mut files = files
            .into_iter()
            .filter(|p| is_image(&p))
            .collect::<Vec<_>>();

        for dir in dirs.iter() {
            let new_files = Self::collect_files(&dir)?;
            files.extend(new_files);
        }

        let mut dirs: HashSet<PathBuf> = HashSet::from_iter(dirs.into_iter());
        for file in files.iter() {
            if let Some(parent) = file.parent() {
                dirs.insert(parent.to_path_buf());
            }
        }

        if dirs.len() == 1 {
            for dir in dirs.iter() {
                let new_files = Self::collect_files(&dir)?;
                files.extend(new_files);
            }
        }

        let files = HashSet::from_iter(files);

        let dirs = if dirs.len() == 1 {
            dirs.into_iter().next()
        } else {
            None
        };
        Ok((dirs, files))
    }
}
