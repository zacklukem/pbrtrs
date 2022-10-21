use image::{ImageBuffer, RgbImage};
use std::cell::{Cell, SyncUnsafeCell};
use std::sync::{Arc, Mutex};

pub const TILE_SIZE: usize = 16;

struct TiledImageData<T> {
    data: SyncUnsafeCell<Vec<T>>,
    width: usize,
    height: usize,
}

pub struct TiledImage<T> {
    data: Arc<TiledImageData<T>>,
    next_tile_x: Cell<usize>,
    next_tile_y: Cell<usize>,
}

pub struct ImageData<T>(Arc<TiledImageData<T>>);

impl<T: Copy + Clone + IntoIterator<Item = u8>> ImageData<T> {
    pub fn to_rgb_image(self) -> Option<RgbImage> {
        unsafe {
            let vec = self
                .0
                .data
                .get()
                .as_ref()
                .unwrap()
                .into_iter()
                .flat_map(|pixel| pixel.into_iter())
                .collect();
            RgbImage::from_vec(self.0.width as u32, self.0.height as u32, vec)
        }
    }
}

impl<T> TiledImage<T>
where
    T: Copy + Default,
{
    pub fn new(width: usize, height: usize) -> TiledImage<T> {
        TiledImage {
            data: Arc::new(TiledImageData {
                data: SyncUnsafeCell::new(vec![Default::default(); width * height]),
                width,
                height,
            }),
            next_tile_x: Cell::new(0),
            next_tile_y: Cell::new(0),
        }
    }

    pub fn to_data(self) -> ImageData<T> {
        let data = self.data;
        assert_eq!(
            Arc::strong_count(&data) + Arc::weak_count(&data),
            1,
            "Leaked memory"
        );
        ImageData(data)
    }

    pub fn get_tile(&self) -> Option<ImageTileIter<T>> {
        if self.next_tile_y.get() >= self.data.height || self.next_tile_x.get() >= self.data.width {
            None
        } else {
            let tile_x = self.next_tile_x.get();
            let tile_y = self.next_tile_y.get();
            let tile_width = (self.data.width - tile_x).min(TILE_SIZE);
            let tile_height = (self.data.height - tile_y).min(TILE_SIZE);
            self.next_tile_x.set(tile_x + TILE_SIZE);
            if self.next_tile_x.get() >= self.data.width {
                self.next_tile_x.set(0);
                self.next_tile_y.set(self.next_tile_y.get() + TILE_SIZE);
            }
            Some(ImageTileIter {
                image: self.data.clone(),
                x: tile_x,
                y: tile_y,
                width: tile_width,
                height: tile_height,
                next_x: tile_x,
                next_y: tile_y,
            })
        }
    }
}

pub struct ImageTileIter<T> {
    image: Arc<TiledImageData<T>>,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    next_x: usize,
    next_y: usize,
}

impl<T> ImageTileIter<T> {
    pub fn location(&self) -> (usize, usize) {
        (self.x, self.y)
    }

    pub fn next(&mut self) -> Option<(&mut T, usize, usize)> {
        if self.next_y >= self.y + self.height || self.next_x >= self.x + self.width {
            None
        } else {
            // Safety: as long as the ImageTileIter class cannot be constructed by anything other
            // than a TiledImage, we can be sure that the range of the image tile is not shared by
            // any other generated image tile, so there will be only one copy of this iterator
            let item = unsafe {
                (
                    &mut self.image.data.get().as_mut().unwrap()
                        [self.next_x + self.next_y * self.image.width],
                    self.next_x,
                    self.next_y,
                )
            };
            self.next_x += 1;
            if self.next_x >= self.x + self.width {
                self.next_x = self.x;
                self.next_y += 1;
            }
            Some(item)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiled_image_get_tile_both_small() {
        let image: TiledImage<u8> = TiledImage::new(10, 10);
        let tile = image.get_tile().unwrap();
        assert!(image.get_tile().is_none());
        assert_eq!(tile.width, 10);
        assert_eq!(tile.height, 10);
        assert_eq!(tile.x, 0);
        assert_eq!(tile.y, 0);
    }

    #[test]
    fn tiled_image_get_tile_small_width() {
        let image: TiledImage<u8> = TiledImage::new(10, 32);
        let tile1 = image.get_tile().unwrap();
        let tile2 = image.get_tile().unwrap();
        assert!(image.get_tile().is_none());
        assert_eq!(tile1.width, 10);
        assert_eq!(tile1.height, 16);
        assert_eq!(tile1.x, 0);
        assert_eq!(tile1.y, 0);
        assert_eq!(tile2.width, 10);
        assert_eq!(tile2.height, 16);
        assert_eq!(tile2.x, 0);
        assert_eq!(tile2.y, 16);
    }

    #[test]
    fn tiled_image_get_tile_square() {
        let image: TiledImage<u8> = TiledImage::new(32, 32);
        let tile1 = image.get_tile().unwrap();
        let tile2 = image.get_tile().unwrap();

        let tile3 = image.get_tile().unwrap();
        let tile4 = image.get_tile().unwrap();
        assert!(image.get_tile().is_none());
        assert_eq!(tile1.width, 16);
        assert_eq!(tile1.height, 16);
        assert_eq!(tile1.x, 0);
        assert_eq!(tile1.y, 0);
        assert_eq!(tile2.width, 16);
        assert_eq!(tile2.height, 16);
        assert_eq!(tile2.x, 16);
        assert_eq!(tile2.y, 0);
        assert_eq!(tile3.width, 16);
        assert_eq!(tile3.height, 16);
        assert_eq!(tile3.x, 0);
        assert_eq!(tile3.y, 16);
        assert_eq!(tile4.width, 16);
        assert_eq!(tile4.height, 16);
        assert_eq!(tile4.x, 16);
        assert_eq!(tile4.y, 16);
    }

    #[test]
    fn tiled_image_get_tile_uneven_square() {
        let image: TiledImage<u8> = TiledImage::new(34, 34);
        let tile1 = image.get_tile().unwrap();
        let tile2 = image.get_tile().unwrap();
        let tile3 = image.get_tile().unwrap();

        let tile4 = image.get_tile().unwrap();
        let tile5 = image.get_tile().unwrap();
        let tile6 = image.get_tile().unwrap();

        let tile7 = image.get_tile().unwrap();
        let tile8 = image.get_tile().unwrap();
        let tile9 = image.get_tile().unwrap();
        assert!(image.get_tile().is_none());

        assert_eq!(tile1.width, 16);
        assert_eq!(tile1.height, 16);
        assert_eq!(tile1.x, 0);
        assert_eq!(tile1.y, 0);
        assert_eq!(tile2.width, 16);
        assert_eq!(tile2.height, 16);
        assert_eq!(tile2.x, 16);
        assert_eq!(tile2.y, 0);
        assert_eq!(tile3.width, 2);
        assert_eq!(tile3.height, 16);
        assert_eq!(tile3.x, 32);
        assert_eq!(tile3.y, 0);

        assert_eq!(tile4.width, 16);
        assert_eq!(tile4.height, 16);
        assert_eq!(tile4.x, 0);
        assert_eq!(tile4.y, 16);
        assert_eq!(tile5.width, 16);
        assert_eq!(tile5.height, 16);
        assert_eq!(tile5.x, 16);
        assert_eq!(tile5.y, 16);
        assert_eq!(tile6.width, 2);
        assert_eq!(tile6.height, 16);
        assert_eq!(tile6.x, 32);
        assert_eq!(tile6.y, 16);

        assert_eq!(tile7.width, 16);
        assert_eq!(tile7.height, 2);
        assert_eq!(tile7.x, 0);
        assert_eq!(tile7.y, 32);
        assert_eq!(tile8.width, 16);
        assert_eq!(tile8.height, 2);
        assert_eq!(tile8.x, 16);
        assert_eq!(tile8.y, 32);
        assert_eq!(tile9.width, 2);
        assert_eq!(tile9.height, 2);
        assert_eq!(tile9.x, 32);
        assert_eq!(tile9.y, 32);
    }

    #[test]
    fn image_tile_top_left() {
        let image: TiledImage<u8> = TiledImage::new(32, 32);
        let mut tile = image.get_tile().unwrap();

        let mut tile_count = 0;
        while tile.next().is_some() {
            tile_count += 1;
        }

        assert_eq!(tile_count, TILE_SIZE * TILE_SIZE);
    }

    #[test]
    fn image_tile_top_right() {
        let image: TiledImage<u8> = TiledImage::new(30, 10);
        image.get_tile();
        let mut tile = image.get_tile().unwrap();

        let mut tile_count = 0;
        while tile.next().is_some() {
            tile_count += 1;
        }

        assert_eq!(tile_count, tile.width * tile.height);
    }
}
