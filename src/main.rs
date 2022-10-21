#![feature(sync_unsafe_cell)]
extern crate cgmath;
extern crate core;
extern crate fastrand;
extern crate image;
extern crate serde;
extern crate serde_derive;
extern crate threadpool;
extern crate toml;

use crate::image_tiler::{TiledImage, TILE_SIZE};
use crate::types::{Color, Mat3, Pt3, R8G8B8Color, Ray, Scalar, Vec3};

use crate::scene::load_scene;
use cgmath::{point3, vec3, Array, EuclideanSpace, InnerSpace};
use fastrand::Rng;
use std::num::NonZeroUsize;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

mod image_tiler;
mod intersect;
mod scene;
mod types;

const NUM_SAMPLES: usize = 10000;
const WIDTH: usize = 256;
const HEIGHT: usize = 256;

#[derive(Default, Clone, Copy)]
enum TileStatus {
    #[default]
    NotStarted,
    Started,
    Finished,
}

fn main() {
    let scene = Arc::new(load_scene("assets/scene.toml"));
    let image_width = WIDTH;
    let image_height = HEIGHT;
    let aspect_ratio = image_width as Scalar / image_height as Scalar;
    let image: TiledImage<R8G8B8Color> = TiledImage::new(image_width, image_height);

    let pool = threadpool::ThreadPool::new(
        std::thread::available_parallelism()
            .map(NonZeroUsize::get)
            .unwrap_or(4),
    );

    // Camera space direction basis
    let camera_x = scene
        .camera
        .direction
        .cross(vec3(0.0, 1.0, 0.0))
        .normalize();
    let camera_y = camera_x.cross(scene.camera.direction).normalize();
    let camera_z = scene.camera.direction.normalize();
    let camera_basis = Mat3::from([camera_x.into(), camera_y.into(), camera_z.into()]);

    let seed_generator = Arc::new(Mutex::new(Rng::new()));

    let num_tiles_height = (image_height as f64 / TILE_SIZE as f64).ceil() as usize;
    let num_tiles_width = (image_width as f64 / TILE_SIZE as f64).ceil() as usize;
    let tiles = Arc::new(Mutex::new(vec![
        TileStatus::NotStarted;
        num_tiles_height * num_tiles_width
    ]));

    while let Some(tile) = image.get_tile() {
        let seed_generator = seed_generator.clone();
        let scene = scene.clone();
        let tiles = tiles.clone();
        let (tile_x, tile_y) = tile.location();
        let (tile_x, tile_y) = (tile_x / TILE_SIZE, tile_y / TILE_SIZE);
        pool.execute(move || {
            {
                tiles.lock().unwrap()[tile_x + tile_y * num_tiles_width] = TileStatus::Started;
            }
            let rng = {
                let seed = seed_generator.lock().unwrap().u64(..);
                Rng::with_seed(seed)
            };
            let mut tile = tile;
            while let Some((pixel, x, y)) = tile.next() {
                let mut color = Color::origin();
                for _ in 0..NUM_SAMPLES {
                    let x = x as Scalar + rng.f32();
                    let y = y as Scalar + rng.f32();
                    let x = (x / image_width as Scalar) * 2.0 - 1.0;
                    let y = ((y / image_height as Scalar) * 2.0 - 1.0) * aspect_ratio;
                    let ray_dir = camera_basis * vec3(x, y, scene.camera.sensor_distance);
                    let ray = Ray::new(scene.camera.position, ray_dir);

                    let intersection = scene.intersect(&ray);
                    if let Some(_) = intersection {
                        color += Vec3::from_value(1.0);
                    }
                }
                color /= NUM_SAMPLES as Scalar;
                *pixel = color.into();
            }
            {
                tiles.lock().unwrap()[tile_x + tile_y * num_tiles_width] = TileStatus::Finished;
            }
        });
    }

    let should_stop = Arc::new(Mutex::new(false));
    let should_stop_owned = should_stop.clone();

    let print_thread = std::thread::spawn(move || {
        while !*should_stop.lock().unwrap() {
            {
                println!("Status:");
                let tiles = tiles.lock().unwrap();
                for x in 0..num_tiles_width {
                    for y in 0..num_tiles_height {
                        print!(
                            "{}",
                            match tiles[x + y * num_tiles_width] {
                                TileStatus::Started => '*',
                                TileStatus::NotStarted => '.',
                                TileStatus::Finished => 'X',
                            }
                        )
                    }
                    println!();
                }
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    });

    pool.join();

    *should_stop_owned.lock().unwrap() = true;

    print_thread.join().unwrap();

    let image = image.to_data().to_rgb_image().unwrap();
    image.save("out.png").unwrap();
}
