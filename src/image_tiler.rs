pub const TILE_SIZE: usize = 16;

pub struct ImageTileGenerator {
    tiles: Vec<(usize, usize, usize, usize)>,
}

impl ImageTileGenerator {
    pub fn new(width: usize, height: usize) -> ImageTileGenerator {
        let mut tiles = Vec::new();
        let (mut next_tile_x, mut next_tile_y) = (0, 0);
        while next_tile_y < height && next_tile_x < width {
            let tile_x = next_tile_x;
            let tile_y = next_tile_y;
            let tile_width = (width - tile_x).min(TILE_SIZE);
            let tile_height = (height - tile_y).min(TILE_SIZE);
            next_tile_x += TILE_SIZE;
            if next_tile_x >= width {
                next_tile_x = 0;
                next_tile_y += TILE_SIZE;
            }
            tiles.push((tile_x, tile_y, tile_width, tile_height));
        }
        fastrand::shuffle(&mut tiles);
        ImageTileGenerator { tiles }
    }

    pub fn get_tile<T: Copy + Default>(&mut self) -> Option<ImageTile<T>> {
        let (tile_x, tile_y, tile_width, tile_height) = self.tiles.pop()?;
        Some(ImageTile {
            tile: vec![Default::default(); tile_width * tile_height],
            x: tile_x,
            y: tile_y,
            width: tile_width,
            height: tile_height,
            next_x: 0,
            next_y: 0,
        })
    }
}

pub struct ImageTile<T> {
    tile: Vec<T>,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    next_x: usize,
    next_y: usize,
}

impl<T> ImageTile<T> {
    pub fn location(&self) -> (usize, usize) {
        (self.x, self.y)
    }

    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    pub fn get(&self, idx: usize) -> &T {
        &self.tile[idx]
    }

    #[allow(unused)]
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        self.tile.get_mut(idx)
    }

    pub fn next(&mut self) -> Option<(&mut T, usize, usize)> {
        if self.next_x >= self.width || self.next_y >= self.height {
            None
        } else {
            let item = (
                &mut self.tile[self.next_x + self.next_y * self.width],
                self.next_x + self.x,
                self.next_y + self.y,
            );
            self.next_x += 1;
            if self.next_x >= self.width {
                self.next_x = 0;
                self.next_y += 1;
            }
            Some(item)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /*
    #[test]
    fn tiled_image_get_tile_both_small() {
        let image = ImageTileGenerator::new(10, 10);
        let tile = image.get_tile::<u8>().unwrap();
        assert!(image.get_tile::<u8>().is_none());
        assert_eq!(tile.width, 10);
        assert_eq!(tile.height, 10);
        assert_eq!(tile.x, 0);
        assert_eq!(tile.y, 0);
    }

    #[test]
    fn tiled_image_get_tile_small_width() {
        let image = ImageTileGenerator::new(10, 32);
        let tile1 = image.get_tile::<u8>().unwrap();
        let tile2 = image.get_tile::<u8>().unwrap();
        assert!(image.get_tile::<u8>().is_none());
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
        let image = ImageTileGenerator::new(32, 32);
        let tile1 = image.get_tile::<u8>().unwrap();
        let tile2 = image.get_tile::<u8>().unwrap();

        let tile3 = image.get_tile::<u8>().unwrap();
        let tile4 = image.get_tile::<u8>().unwrap();
        assert!(image.get_tile::<u8>().is_none());
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
        let image = ImageTileGenerator::new(34, 34);
        let tile1 = image.get_tile::<u8>().unwrap();
        let tile2 = image.get_tile::<u8>().unwrap();
        let tile3 = image.get_tile::<u8>().unwrap();

        let tile4 = image.get_tile::<u8>().unwrap();
        let tile5 = image.get_tile::<u8>().unwrap();
        let tile6 = image.get_tile::<u8>().unwrap();

        let tile7 = image.get_tile::<u8>().unwrap();
        let tile8 = image.get_tile::<u8>().unwrap();
        let tile9 = image.get_tile::<u8>().unwrap();
        assert!(image.get_tile::<u8>().is_none());

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
    */

    #[test]
    fn image_tile_top_left() {
        let mut image = ImageTileGenerator::new(32, 32);
        let mut tile = image.get_tile::<u8>().unwrap();

        let mut tile_count = 0;
        while tile.next().is_some() {
            tile_count += 1;
        }

        assert_eq!(tile_count, TILE_SIZE * TILE_SIZE);
    }

    #[test]
    fn image_tile_top_right() {
        let mut image = ImageTileGenerator::new(30, 10);
        image.get_tile::<u8>();
        let mut tile = image.get_tile::<u8>().unwrap();

        let mut tile_count = 0;
        while tile.next().is_some() {
            tile_count += 1;
        }

        assert_eq!(tile_count, tile.width * tile.height);
    }
}
