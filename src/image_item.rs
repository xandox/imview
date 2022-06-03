struct ImageFile {}
struct Thumbnail {}
struct FullImage {}

pub struct ImageItem {
    file: ImageFile,
    thumbnail: Thumbnail,
    image: FullImage,
}

impl ImageItem {
    pub fn new(path: PathBuf) -> Self {
        Self {
            file: ImageFile {},
            thumbnail: Thumbnail {},
            image: FullImage {},
        }
    }
}
