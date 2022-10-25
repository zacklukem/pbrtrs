extern crate cgmath;
extern crate core;
extern crate fastrand;
extern crate image;
extern crate serde;
extern crate serde_derive;
extern crate show_image;
extern crate threadpool;
extern crate toml;

use crate::image_tiler::{ImageTile, ImageTileGenerator};
use crate::types::{scalar, Color, Mat3, R8G8B8Color, Ray, Scalar};

use crate::raytracer::ray_color;
use crate::scene::load_scene;
use cgmath::{vec3, EuclideanSpace, InnerSpace};
use fastrand::Rng;
use image::{Rgb, RgbImage};
use show_image::event::WindowEvent;
use show_image::WindowOptions;
use std::num::NonZeroUsize;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

mod image_tiler;
mod intersect;
mod raytracer;
mod scene;
mod types;
mod util;

#[show_image::main]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Loading scene...");
    let scene = Arc::new(load_scene("assets/scene.toml"));
    println!("Rendering...");

    let image_width = scene.camera.width;
    let image_height = scene.camera.height;

    let image_viewer = show_image::create_window(
        "pbrtrs",
        WindowOptions {
            size: Some([image_width as u32 * 3, image_height as u32 * 3]),
            ..WindowOptions::default()
        },
    )
    .unwrap();

    let aspect_ratio = image_width as Scalar / image_height as Scalar;
    let mut image_tile_generator = ImageTileGenerator::new(image_width, image_height);

    let pool = threadpool::Builder::new()
        .thread_name("render_thread".to_owned())
        .num_threads(
            thread::available_parallelism()
                .map(NonZeroUsize::get)
                .unwrap_or(4)
                * 2,
        )
        .build();

    // Camera space direction basis
    let camera_x = -scene
        .camera
        .direction
        .cross(vec3(0.0, 1.0, 0.0))
        .normalize();
    let camera_y = camera_x.cross(scene.camera.direction).normalize();
    let camera_z = scene.camera.direction.normalize();
    let camera_basis = Mat3::from([camera_x.into(), camera_y.into(), camera_z.into()]);

    let seed_generator = Arc::new(Mutex::new(Rng::new()));

    let (image_writer_tx, image_writer_rx) = mpsc::channel();

    // start of rt
    let rt_start = Instant::now();

    while let Some(tile) = image_tile_generator.get_tile() {
        let seed_generator = seed_generator.clone();
        let scene = scene.clone();
        let image_writer_tx = image_writer_tx.clone();
        pool.execute(move || {
            {
                let seed = seed_generator.lock().unwrap().u64(..);
                fastrand::seed(seed);
            }
            // Render tile
            let mut tile: ImageTile<R8G8B8Color> = tile;
            while let Some((pixel, x, y)) = tile.next() {
                let mut color = Color::origin();
                for _ in 0..scene.camera.num_samples {
                    let x = x as Scalar + scalar::rand();
                    let y = y as Scalar + scalar::rand();
                    let x = (x / image_width as Scalar) * 2.0 - 1.0;
                    let y = ((y / image_height as Scalar) * 2.0 - 1.0) / aspect_ratio;
                    let ray_dir = camera_basis * vec3(x, y, scene.camera.sensor_distance);
                    let ray = Ray::new(scene.camera.position, ray_dir);

                    color += ray_color(&ray, &scene, 0).to_vec();
                }
                color /= scene.camera.num_samples as Scalar;
                color = color.map(|v| v.sqrt());
                *pixel = color.into();
            }

            #[cfg(feature = "enable_axis")]
            if tile.location() == (0, 0) {
                draw_axis(&mut tile, &scene);
            }

            image_writer_tx.send(Some(tile)).unwrap();
        });
    }

    // Draw tiles to image preview

    let pool_ender_thread = thread::Builder::new()
        .name("pool_ender".to_owned())
        .spawn(move || {
            pool.join();
            let end = rt_start.elapsed();
            println!("Time required: {:?}", end);
            image_writer_tx.send(None).unwrap();
        })
        .unwrap();

    let mut output_image =
        RgbImage::from_pixel(image_width as u32, image_height as u32, Rgb([80, 80, 80]));

    let mut time = Instant::now();

    while let Some(tile) = image_writer_rx.recv().unwrap() {
        let (tile_x, tile_y) = tile.location();
        let (width, height) = tile.dimensions();
        for x in 0..width {
            for y in 0..height {
                let (image_x, image_y) = (x + tile_x, y + tile_y);

                let pixel = *tile.get(x + y * width);

                output_image.put_pixel(image_x as u32, image_y as u32, pixel.into());
            }
        }
        if time.elapsed() > Duration::from_millis(250) {
            image_viewer
                .set_image("image", output_image.clone())
                .unwrap();
            time = Instant::now();
        }
    }

    image_viewer
        .set_image("image", output_image.clone())
        .unwrap();

    pool_ender_thread.join().unwrap();

    output_image.save("./out.png").unwrap();

    let window_rx = image_viewer.event_channel().unwrap();
    loop {
        if let WindowEvent::CloseRequested(_) = window_rx.recv().unwrap() {
            break;
        }
    }

    Ok(())
}

#[cfg(feature = "enable_axis")]
fn draw_axis(tile: &mut ImageTile<R8G8B8Color>, scene: &scene::Scene) {
    use crate::image_tiler::TILE_SIZE;
    use crate::types::color;
    use cgmath::{point3, vec2, SquareMatrix, Transform};

    let root_pt = point3(0.0, 0.0, 0.0);
    let x_pt = point3(1.0, 0.0, 0.0);
    let y_pt = point3(0.0, 1.0, 0.0);
    let z_pt = point3(0.0, 0.0, 1.0);

    let camera_x = -scene
        .camera
        .direction
        .cross(vec3(0.0, 1.0, 0.0))
        .normalize();
    let camera_y = camera_x.cross(scene.camera.direction).normalize();
    let camera_z = scene.camera.direction.normalize();
    // Ax = b, A: camera_basis, x: camera_space_coords, b: world_space_coords
    let camera_basis = Mat3::from([camera_x.into(), camera_y.into(), camera_z.into()]);
    let world_basis = camera_basis.invert().unwrap();

    let root_pt = world_basis.transform_point(root_pt).xy();
    let x_pt = world_basis.transform_point(x_pt).xy();
    let y_pt = world_basis.transform_point(y_pt).xy();
    let z_pt = world_basis.transform_point(z_pt).xy();

    let lines = [
        (x_pt - root_pt, color::RED),
        (y_pt - root_pt, color::GREEN),
        (z_pt - root_pt, color::BLUE),
    ];

    for t in 0..20 {
        let t = t as Scalar / 20.0;
        for (line, color) in lines {
            let pt = root_pt + line * t;
            let pt = (pt + vec2(1.0, 1.0) / 2.0) * TILE_SIZE as Scalar;
            let pt = pt.map(|v| v as usize);
            if pt.x < TILE_SIZE && pt.y < TILE_SIZE {
                *tile.get_mut(pt.x + pt.y * TILE_SIZE).unwrap() = R8G8B8Color::from(color);
            }
        }
    }
}
