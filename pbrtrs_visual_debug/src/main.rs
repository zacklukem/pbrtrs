extern crate kiss3d;
extern crate xml;

use cgmath::{point3, vec3, EuclideanSpace, Zero};
use kiss3d::light::Light;
use kiss3d::nalgebra::{Point3, Translation3, Vector3};
use kiss3d::window::Window;
use pbrtrs_core::scene::{load_scene, Camera, Shape, Texture};
use pbrtrs_core::types::{scalar, Color, Pt3, Vec3};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use xml::attribute::OwnedAttribute;
use xml::reader::{Events, XmlEvent};
use xml::EventReader;

#[allow(unused)]
#[derive(Debug)]
struct Pixel {
    color: Color,
    samples: Vec<Sample>,
}

#[allow(unused)]
#[derive(Debug)]
struct Sample {
    idx: usize,
    color: Color,
    bounces: Vec<Ray>,
}

#[allow(unused)]
#[derive(Debug)]
struct Ray {
    idx: usize,
    origin: Pt3,
    direction: Vec3,
    debug: String,
}

struct VisualDebuggerSharedData {
    ray_lines: Vec<(Point3<f32>, Point3<f32>, Point3<f32>)>,
    debug_vectors: Vec<(Point3<f32>, Point3<f32>, Point3<f32>)>,
}

struct VisualDebugger {
    shared_data: Arc<Mutex<VisualDebuggerSharedData>>,
    pixel: Pixel,
    sample: usize,
}

impl VisualDebugger {
    pub fn new(pixel: Pixel) -> VisualDebugger {
        let vd = VisualDebugger {
            shared_data: Arc::new(Mutex::new(VisualDebuggerSharedData {
                ray_lines: vec![],
                debug_vectors: vec![],
            })),
            pixel,
            sample: 0,
        };
        vd.update_ray_lines();
        vd
    }

    fn reset_debug_vectors(&self) {
        let mut shared_data = self.shared_data.lock().unwrap();
        shared_data.debug_vectors.clear();
    }

    fn add_debug_vector(&self, v: (Point3<f32>, Point3<f32>, Point3<f32>)) {
        let mut shared_data = self.shared_data.lock().unwrap();
        shared_data.debug_vectors.push(v);
    }

    fn update_ray_lines(&self) {
        let mut d = self.shared_data.lock().unwrap();

        d.ray_lines.clear();

        let mut last: Option<Pt3> = None;
        for ray in &self.current_sample().bounces {
            if let Some(l) = last {
                let l = Point3::new(l.x, l.y, l.z);
                let o = Point3::new(ray.origin.x, ray.origin.y, ray.origin.z);
                d.ray_lines.push((l, o, Point3::new(1.0, 0.0, 0.0)));
                last = Some(ray.origin);
            } else {
                last = Some(ray.origin);
            }
        }
        if let Some(ray) = self.current_sample().bounces.last() {
            let o0 = Point3::new(ray.origin.x, ray.origin.y, ray.origin.z);
            let o1 = ray.origin + ray.direction;
            let o1 = Point3::new(o1.x, o1.y, o1.z);
            d.ray_lines.push((o0, o1, Point3::new(1.0, 0.0, 1.0)));
        }
    }

    fn highlight_ray(&self, idx: usize) {
        let mut d = self.shared_data.lock().unwrap();

        for (i, ray) in d.ray_lines.iter_mut().enumerate() {
            if i != idx {
                ray.2 = Point3::new(1.0, 0.0, 0.0);
            } else {
                ray.2 = Point3::new(0.0, 1.0, 0.0);
            }
        }
    }

    fn current_sample(&self) -> &Sample {
        &self.pixel.samples[self.sample]
    }
}

fn cgm_to_kiss3d_vec3(v: Vec3) -> Vector3<f32> {
    Vector3::new(v.x, v.y, v.z)
}

fn cgm_to_kiss3d_pt3(v: Pt3) -> Point3<f32> {
    Point3::new(v.x, v.y, v.z)
}

fn main() {
    let file = File::open("debug_out.xml").unwrap();
    let file = BufReader::new(file);
    let parser = EventReader::new(file);

    let mut parser = parser.into_iter();
    let (pixel, _camera) = parse_document(&mut parser);
    drop(parser);

    let scene = load_scene("examples/hdr.toml");

    let mut vd = VisualDebugger::new(pixel);

    let mut window = Window::new("Debug");
    window.set_light(Light::StickToCamera);

    for object in &scene.objects {
        let mut node = match &object.shape {
            Shape::Sphere { radius } => {
                let mut sphere = window.add_sphere(*radius);
                sphere.set_local_translation(Translation3::new(
                    object.position.x,
                    object.position.y,
                    object.position.z,
                ));
                sphere
            }
        };

        match &object.material.base_color {
            Texture::Value(c) => {
                node.set_color(c.x, c.y, c.z);
            }
            Texture::Image(_) => {
                node.set_color(scalar::rand(), scalar::rand(), scalar::rand());
            }
        }
    }

    let window_is_open = Arc::new(AtomicBool::new(true));

    let vd_shared_data = vd.shared_data.clone();

    let _prompt_thread = {
        let window_is_open = window_is_open.clone();
        let current_origin = cgm_to_kiss3d_pt3(scene.camera.position);
        thread::spawn(move || {
            let mut current_origin = current_origin;
            let mut current_debug_refs = Vec::new();
            while window_is_open.load(Ordering::Relaxed) {
                print!("> ");
                std::io::stdout().flush().unwrap();
                let mut input_raw = String::new();
                std::io::stdin().read_line(&mut input_raw).unwrap();
                let input = input_raw.trim().split(' ').collect::<Vec<_>>();
                macro_rules! prompt_try {
                    ($e: expr) => {
                        (match $e {
                            Ok(v) => v,
                            Err(_) => {
                                println!("Invalid input");
                                continue;
                            }
                        })
                    };
                }
                macro_rules! prompt_try_opt {
                    ($e: expr) => {
                        (match $e {
                            Some(v) => v,
                            None => {
                                println!("Invalid input");
                                continue;
                            }
                        })
                    };
                }
                macro_rules! arg {
                    ($n: expr) => {
                        prompt_try_opt!(input.get($n))
                    };
                }
                match input[0] {
                    "q" => {
                        window_is_open.store(false, Ordering::Relaxed);
                    }
                    "s" => {
                        let sample = prompt_try!(arg!(1).parse::<usize>());
                        vd.sample = sample;
                        vd.update_ray_lines();
                    }
                    "r" => {
                        let ray_idx = arg!(1).parse::<usize>().unwrap();
                        vd.highlight_ray(ray_idx);
                        let ray = prompt_try_opt!(vd.current_sample().bounces.get(ray_idx));
                        if let Some(after) = vd.current_sample().bounces.get(ray_idx + 1) {
                            current_origin = cgm_to_kiss3d_pt3(after.origin);
                        } else {
                            current_origin = cgm_to_kiss3d_pt3(ray.origin);
                        }

                        for line in ray.debug.trim().lines() {
                            let line = line.trim();
                            if line.starts_with("pbrtrs_core") {
                                println!("@{line}");
                            } else {
                                let (name, value) = prompt_try_opt!(line.split_once(':'));
                                let name = name.trim();
                                let value = value.trim();
                                let idx = current_debug_refs.len();
                                current_debug_refs.push(value.to_string());
                                println!("    {idx}: {name}: {value}");
                            }
                        }
                    }
                    "clear" => {
                        vd.reset_debug_vectors();
                    }
                    "v" => {
                        let x = prompt_try!(arg!(1).parse::<f32>());
                        let y = prompt_try!(arg!(2).parse::<f32>());
                        let z = prompt_try!(arg!(3).parse::<f32>());
                        let v = Vector3::new(x, y, z);
                        vd.add_debug_vector((
                            current_origin,
                            current_origin + v,
                            Point3::new(0.0, 1.0, 1.0),
                        ));
                    }
                    "vr" => {
                        let r = prompt_try!(arg!(1).parse::<usize>());
                        let val = &current_debug_refs[r];
                        let v = cgm_to_kiss3d_vec3(parse_vec3(val));

                        vd.add_debug_vector((
                            current_origin,
                            current_origin + v,
                            Point3::new(0.0, 1.0, 1.0),
                        ));
                    }
                    _ => {
                        println!("Invalid");
                    }
                }
            }
        })
    };

    while window.render() {
        let vd = vd_shared_data.lock().unwrap();
        for ray in vd.ray_lines.iter().chain(vd.debug_vectors.iter()) {
            window.draw_line(&ray.0, &ray.1, &ray.2);
            window.draw_point(&ray.1, &ray.2)
        }
    }

    window_is_open.store(false, Ordering::Relaxed);
}

fn parse_document(parser: &mut Events<impl Read>) -> (Pixel, Camera) {
    let mut pixel = None;
    let mut camera = None;
    while let Some(e) = parser.next() {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => match name.local_name.as_str() {
                "pixel" => pixel = Some(parse_pixel(parser, &attributes)),
                "camera" => camera = Some(parse_camera(parser, &attributes)),
                _ => {}
            },
            Err(e) => println!("Error: {}", e),
            _ => {}
        }
    }
    (pixel.unwrap(), camera.unwrap())
}

fn parse_camera(parser: &mut Events<impl Read>, _attr: &[OwnedAttribute]) -> Camera {
    let mut out = Camera {
        position: Pt3::origin(),
        direction: Vec3::zero(),
        sensor_distance: 0.0,
        exposure_time: 0.0,
        aperture: 0.0,
        focus_distance: 0.0,
        ldr_scale: 0.0,
        bounce_limit: 0,
        num_samples: 0,
        width: 0,
        height: 0,
    };
    for e in parser.by_ref() {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                let v = get_attr(&attributes, "value");
                match name.local_name.as_str() {
                    "position" => out.position = parse_pt3(v.unwrap()),
                    "direction" => out.direction = parse_vec3(v.unwrap()),
                    "sensor_distance" => out.sensor_distance = v.unwrap().parse().unwrap(),
                    "exposure_time" => out.exposure_time = v.unwrap().parse().unwrap(),
                    "aperture" => out.aperture = v.unwrap().parse().unwrap(),
                    "focus_distance" => out.focus_distance = v.unwrap().parse().unwrap(),
                    "ldr_scale" => out.ldr_scale = v.unwrap().parse().unwrap(),
                    "bounce_limit" => out.bounce_limit = v.unwrap().parse().unwrap(),
                    "num_samples" => out.num_samples = v.unwrap().parse().unwrap(),
                    "width" => out.width = v.unwrap().parse().unwrap(),
                    "height" => out.height = v.unwrap().parse().unwrap(),
                    _ => {}
                }
            }
            Ok(XmlEvent::EndElement { name }) if name.local_name.as_str() == "camera" => break,
            Err(e) => println!("Error: {}", e),
            _ => {}
        }
    }
    out
}

fn get_attr<'a>(attributes: &'a [OwnedAttribute], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|a| a.name.local_name == name)
        .map(|a| a.value.as_str())
}

fn parse_pt3(s: &str) -> Pt3 {
    let brackets = s
        .trim_start_matches("Point3 [")
        .trim_end_matches(']')
        .split(',');
    let el = brackets
        .map(|s| s.trim().parse::<f32>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(el.len(), 3);
    point3(el[0], el[1], el[2])
}

fn parse_vec3(s: &str) -> Vec3 {
    let brackets = s
        .trim_start_matches("Vector3 [")
        .trim_end_matches(']')
        .split(',');
    let el = brackets
        .map(|s| s.trim().parse::<f32>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(el.len(), 3);
    vec3(el[0], el[1], el[2])
}

fn parse_color(s: &str) -> Color {
    parse_pt3(s)
}

fn parse_pixel(parser: &mut Events<impl Read>, attr: &[OwnedAttribute]) -> Pixel {
    let mut out = Pixel {
        color: parse_color(get_attr(attr, "color").unwrap()),
        samples: vec![],
    };
    while let Some(e) = parser.next() {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) if name.local_name.as_str() == "sample" => {
                out.samples.push(parse_sample(parser, &attributes))
            }
            Ok(XmlEvent::EndElement { name }) if name.local_name.as_str() == "pixel" => {
                break;
            }
            Err(e) => println!("Error: {}", e),
            _ => {}
        }
    }
    out
}

fn parse_sample(parser: &mut Events<impl Read>, attr: &[OwnedAttribute]) -> Sample {
    let mut out = Sample {
        idx: get_attr(attr, "idx").unwrap().parse().unwrap(),
        color: parse_color(get_attr(attr, "color").unwrap()),
        bounces: vec![],
    };
    while let Some(e) = parser.next() {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) if name.local_name.as_str() == "ray" => {
                out.bounces.push(parse_ray(parser, &attributes))
            }
            Ok(XmlEvent::EndElement { name }) if name.local_name.as_str() == "sample" => {
                break;
            }
            Err(e) => println!("Error: {}", e),
            _ => {}
        }
    }
    out
}
fn parse_ray(parser: &mut Events<impl Read>, attr: &[OwnedAttribute]) -> Ray {
    let mut out = Ray {
        idx: get_attr(attr, "idx").unwrap().parse().unwrap(),
        origin: parse_pt3(get_attr(attr, "origin").unwrap()),
        direction: parse_vec3(get_attr(attr, "direction").unwrap()),
        debug: String::new(),
    };
    for e in parser.by_ref() {
        match e {
            Ok(XmlEvent::Whitespace(s)) | Ok(XmlEvent::Characters(s)) => out.debug.push_str(&s),
            Ok(XmlEvent::EndElement { name }) if name.local_name.as_str() == "ray" => {
                break;
            }
            Err(e) => println!("Error: {}", e),
            _ => {}
        }
    }
    out
}
