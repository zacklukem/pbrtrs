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

    pub fn get_num_tiles(&self) -> usize {
        self.tiles.len()
    }

    pub fn get_tile<T: Copy>(&mut self, default: T) -> Option<ImageTile<T>> {
        let (tile_x, tile_y, tile_width, tile_height) = self.tiles.pop()?;
        Some(ImageTile {
            tile: vec![default; tile_width * tile_height],
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

    pub fn next_tile(&mut self) -> Option<(&mut T, usize, usize)> {
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
