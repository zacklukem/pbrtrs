extern crate kiss3d;
extern crate xml;

use cgmath::{point3, vec3, EuclideanSpace, Zero};
use kiss3d::builtin::NormalsMaterial;
use kiss3d::light::Light;
use kiss3d::loader::mtl::MtlMaterial;
use kiss3d::nalgebra::{Point3, Translation3};
use kiss3d::resource::Material;
use kiss3d::window::Window;
use pbrtrs_core::scene::{load_scene, Camera, Shape};
use pbrtrs_core::types::{scalar, Color, Pt3, Vec3};
use std::cell::RefCell;
use std::fs::File;
use std::io::{BufReader, Read};
use std::rc::Rc;
use xml::attribute::OwnedAttribute;
use xml::reader::{Events, XmlEvent};
use xml::EventReader;

#[derive(Debug)]
struct Pixel {
    color: Color,
    samples: Vec<Sample>,
}

#[derive(Debug)]
struct Sample {
    idx: usize,
    color: Color,
    bounces: Vec<Ray>,
}

#[derive(Debug)]
struct Ray {
    idx: usize,
    origin: Pt3,
    direction: Vec3,
    debug: String,
}

fn main() {
    let file = File::open("debug_out.xml").unwrap();
    let file = BufReader::new(file);
    let parser = EventReader::new(file);

    let mut parser = parser.into_iter();
    let (pixel, _camera) = parse_document(&mut parser);
    drop(parser);

    let scene = load_scene("examples/hdr.toml");

    let sample = &pixel.samples[186];
    let mut rays = vec![];
    let mut last: Option<Pt3> = None;
    for ray in &sample.bounces {
        if let Some(l) = last {
            let l = Point3::new(l.x, l.y, l.z);
            let o = Point3::new(ray.origin.x, ray.origin.y, ray.origin.z);
            rays.push((l, o));
            last = Some(ray.origin);
        } else {
            last = Some(ray.origin);
        }
    }

    let mut window = Window::new("Debug");
    window.set_light(Light::StickToCamera);

    for object in &scene.objects {
        match &object.shape {
            Shape::Sphere { radius } => {
                let mut sphere = window.add_sphere(*radius);
                sphere.set_color(scalar::rand(), scalar::rand(), scalar::rand());
                sphere.set_local_translation(Translation3::new(
                    object.position.x,
                    object.position.y,
                    object.position.z,
                ));
            }
        }
    }

    while window.render() {
        for ray in &rays {
            window.draw_line(&ray.0, &ray.1, &Point3::new(1.0, 0.0, 0.0));
        }
    }
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
    while let Some(e) = parser.next() {
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
            Ok(XmlEvent::EndElement { name }) => match name.local_name.as_str() {
                "camera" => break,
                _ => {}
            },
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
        .trim_end_matches("]")
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
        .trim_end_matches("]")
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
            }) => match name.local_name.as_str() {
                "sample" => out.samples.push(parse_sample(parser, &attributes)),
                _ => {}
            },
            Ok(XmlEvent::EndElement { name }) => match name.local_name.as_str() {
                "pixel" => break,
                _ => {}
            },
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
            }) => match name.local_name.as_str() {
                "ray" => out.bounces.push(parse_ray(parser, &attributes)),
                _ => {}
            },
            Ok(XmlEvent::EndElement { name }) => match name.local_name.as_str() {
                "sample" => break,
                _ => {}
            },
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
    while let Some(e) = parser.next() {
        match e {
            Ok(XmlEvent::Whitespace(s)) | Ok(XmlEvent::Characters(s)) => out.debug.push_str(&s),
            Ok(XmlEvent::EndElement { name }) => match name.local_name.as_str() {
                "ray" => break,
                _ => {}
            },
            Err(e) => println!("Error: {}", e),
            _ => {}
        }
    }
    out
}
