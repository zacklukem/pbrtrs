extern crate bumpalo;
extern crate cgmath;
extern crate fastrand;
extern crate image;
extern crate pbrtrs_core;
extern crate show_image;
extern crate threadpool;

mod image_tiler;

use pbrtrs_core::debugger;
use pbrtrs_core::types::{scalar, Color, Mat3, R8G8B8Color, Ray, Scalar};

use bumpalo::Bump;
use cgmath::{vec3, EuclideanSpace, InnerSpace};
use image::{Rgb, RgbImage};
use image_tiler::{ImageTile, ImageTileGenerator};
use pbrtrs_core::raytracer::ray_color;
use pbrtrs_core::scene::load_scene;
use pbrtrs_core::types::scalar::consts::PI;
use show_image::event::WindowEvent;
use show_image::WindowOptions;
use std::num::NonZeroUsize;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(feature = "enable_debugger")]
use pbrtrs_core::debugger::debug_info;

#[cfg(feature = "enable_debugger")]
const DEBUG_PIXEL: (usize, usize) = (175, 153);

#[show_image::main]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Deterministic rendering
    fastrand::seed(0x8815_6e97_8ca3_1877);

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

    let total_num_tiles = image_tile_generator.get_num_tiles();

    let pool = threadpool::Builder::new()
        .thread_name("render_thread".to_owned())
        .num_threads(
            thread::available_parallelism()
                .map(NonZeroUsize::get)
                .unwrap_or(4),
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

    let (image_writer_tx, image_writer_rx) = mpsc::channel();

    // start of rt
    let rt_start = Instant::now();

    while let Some(tile) = image_tile_generator.get_tile() {
        let scene = scene.clone();
        let image_writer_tx = image_writer_tx.clone();
        let seed = fastrand::u64(..);
        pool.execute(move || {
            fastrand::seed(seed);
            // Render tile
            let mut tile: ImageTile<R8G8B8Color> = tile;
            while let Some((pixel, x, y)) = tile.next_tile() {
                #[cfg(feature = "enable_debugger")]
                debugger::set_should_debug_pixel((x, y) == DEBUG_PIXEL);

                let arena = Bump::new();

                let mut color = Color::origin();
                for _ in 0..scene.camera.num_samples {
                    debugger::begin_sample!();
                    let time = scalar::rand() * scene.camera.exposure_time;
                    let x = x as Scalar + scalar::rand();
                    let y = y as Scalar + scalar::rand();
                    let x = (x / image_width as Scalar) * 2.0 - 1.0;
                    let y = ((y / image_height as Scalar) * 2.0 - 1.0) / aspect_ratio;
                    let ray_dir = camera_basis * vec3(x, y, scene.camera.sensor_distance);
                    let ray = Ray::new(scene.camera.position, ray_dir, time);

                    let sample_color = ray_color(&ray, &scene, &arena);
                    debugger::end_sample!(sample_color);
                    if sample_color.x.is_finite()
                        && sample_color.y.is_finite()
                        && sample_color.z.is_finite()
                    {
                        color += sample_color.to_vec();
                    }
                }
                color /= scene.camera.num_samples as Scalar;
                debugger::end_pixel!(color);
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

    let mut num_tiles: usize = 0;

    while let Some(tile) = image_writer_rx.recv().unwrap() {
        num_tiles += 1;
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
            let elapsed_time = rt_start.elapsed();
            let time_per_tile = elapsed_time / num_tiles as u32;
            let remaining_tiles = total_num_tiles - num_tiles;
            let remaining_time = time_per_tile * remaining_tiles as u32;

            println!(
                "{num_tiles}/{total_num_tiles}; Elapsed: {:?}, Remaining Time: {:?}, Time Per Tile: {:?}",
                elapsed_time, remaining_time, time_per_tile,
            );

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

    #[cfg(feature = "enable_debugger")]
    {
        let debug = debug_info().lock().unwrap();
        debug.save("debug_out.txt");
    }

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
fn draw_axis(tile: &mut ImageTile<R8G8B8Color>, scene: &pbrtrs_core::scene::Scene) {
    use crate::image_tiler::TILE_SIZE;
    use cgmath::{point3, vec2, SquareMatrix, Transform};
    use pbrtrs_core::types::color;

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
