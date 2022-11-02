use crate::types::scalar::consts::PI;
use crate::types::{color, Color, Euler, Mat4, Pt2, Pt3, Scalar, Vec3};

use cgmath::{vec3, EuclideanSpace, InnerSpace, Rad};
use image::{ImageBuffer, Luma, Pixel, Rgb, Rgb32FImage};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use std::path::Path;

use crate::light::hdri::Hdri;
use crate::types::R8G8B8Color;
use serde::de::{Error as SerdeError, SeqAccess, Visitor};
use serde::{Deserialize as DeserializeTrait, Deserializer};
use serde_derive::Deserialize;

pub trait PixelConverter<T> {
    type Pixel: Pixel;
    fn from_pixel(v: &Self::Pixel) -> T;
}

pub struct Rgb8ColorPixelConverter;

impl PixelConverter<Color> for Rgb8ColorPixelConverter {
    type Pixel = Rgb<u8>;

    fn from_pixel(v: &Self::Pixel) -> Color {
        let color: Color = R8G8B8Color(v.0).into();
        color
    }
}

pub struct Luma8ColorPixelConverter;

impl PixelConverter<Scalar> for Luma8ColorPixelConverter {
    type Pixel = Luma<u8>;

    fn from_pixel(v: &Self::Pixel) -> Scalar {
        v.0[0] as f32 / 255.0
    }
}

pub enum Texture<T, P: PixelConverter<T>> {
    Value(T),
    Image(ImageBuffer<P::Pixel, Vec<<P::Pixel as Pixel>::Subpixel>>),
}

impl<T: Debug, P: PixelConverter<T>> Debug for Texture<T, P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value(v) => v.fmt(f),
            _ => write!(f, "[image]"),
        }
    }
}

impl<T: Default, P: PixelConverter<T>> Default for Texture<T, P> {
    fn default() -> Self {
        Self::Value(Default::default())
    }
}

impl<T: Copy, P: PixelConverter<T>> Texture<T, P> {
    pub fn get(&self, uv: Pt2) -> T {
        match self {
            Self::Value(value) => *value,
            Self::Image(image) => {
                let (width, height) = image.dimensions();
                let (x, y) = (
                    ((width as Scalar * uv.x) as u32).min(width - 1),
                    ((height as Scalar * uv.y) as u32).min(height - 1),
                );
                P::from_pixel(image.get_pixel(x, y))
            }
        }
    }
}

struct TextureScalarVisitor<P>(PhantomData<P>);

impl<'de, P: PixelConverter<Scalar, Pixel = Luma<u8>>> Visitor<'de> for TextureScalarVisitor<P> {
    type Value = Texture<Scalar, P>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "String path to texture or scalar")
    }

    fn visit_f64<E: SerdeError>(self, v: f64) -> Result<Self::Value, E> {
        Ok(Texture::Value(v as Scalar))
    }

    fn visit_str<E: SerdeError>(self, v: &str) -> Result<Self::Value, E> {
        let image = image::io::Reader::open(v).unwrap().decode().unwrap();
        Ok(Texture::Image(image.into_luma8()))
    }
}

impl<'de, P: PixelConverter<Scalar, Pixel = Luma<u8>>> DeserializeTrait<'de>
    for Texture<Scalar, P>
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(TextureScalarVisitor::<P>(PhantomData))
    }
}

struct TextureColorVisitor<P>(PhantomData<P>);

impl<'de, P: PixelConverter<Color, Pixel = Rgb<u8>>> Visitor<'de> for TextureColorVisitor<P> {
    type Value = Texture<Color, P>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "String path to texture or 3 component rgb color")
    }

    fn visit_str<E: SerdeError>(self, v: &str) -> Result<Self::Value, E> {
        let image = image::io::Reader::open(v).unwrap().decode().unwrap();
        Ok(Texture::Image(image.into_rgb8()))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let a: Scalar = seq
            .next_element()?
            .ok_or_else(|| A::Error::custom("Expected 3 elements"))?;
        let b: Scalar = seq
            .next_element()?
            .ok_or_else(|| A::Error::custom("Expected 3 elements"))?;
        let c: Scalar = seq
            .next_element()?
            .ok_or_else(|| A::Error::custom("Expected 3 elements"))?;
        Ok(Texture::Value(color(a, b, c)))
    }
}

impl<'de, P: PixelConverter<Color, Pixel = Rgb<u8>>> DeserializeTrait<'de> for Texture<Color, P> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(TextureColorVisitor::<P>(PhantomData))
    }
}

#[derive(Debug, Deserialize)]
pub struct DisneyMaterial {
    pub base_color: Texture<Color, Rgb8ColorPixelConverter>,
    pub subsurface: Texture<Scalar, Luma8ColorPixelConverter>,
    pub metallic: Texture<Scalar, Luma8ColorPixelConverter>,
    pub specular: Texture<Scalar, Luma8ColorPixelConverter>,
    pub specular_tint: Texture<Scalar, Luma8ColorPixelConverter>,
    pub roughness: Texture<Scalar, Luma8ColorPixelConverter>,
    pub anisotropic: Texture<Scalar, Luma8ColorPixelConverter>,
    pub sheen: Texture<Scalar, Luma8ColorPixelConverter>,
    pub sheen_tint: Texture<Scalar, Luma8ColorPixelConverter>,
    pub clearcoat: Texture<Scalar, Luma8ColorPixelConverter>,
    pub clearcoat_gloss: Texture<Scalar, Luma8ColorPixelConverter>,
}

#[derive(Debug)]
pub struct SampledDisneyMaterial {
    pub base_color: Color,
    pub subsurface: Scalar,
    pub metallic: Scalar,
    pub specular: Scalar,
    pub specular_tint: Scalar,
    pub roughness: Scalar,
    pub anisotropic: Scalar,
    pub sheen: Scalar,
    pub sheen_tint: Scalar,
    pub clearcoat: Scalar,
    pub clearcoat_gloss: Scalar,
}

impl Default for DisneyMaterial {
    fn default() -> Self {
        Self {
            base_color: Texture::Value(Color::origin()),
            subsurface: Default::default(),
            metallic: Default::default(),
            specular: Default::default(),
            specular_tint: Default::default(),
            roughness: Default::default(),
            anisotropic: Default::default(),
            sheen: Default::default(),
            sheen_tint: Default::default(),
            clearcoat: Default::default(),
            clearcoat_gloss: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Object {
    pub shape: Shape,
    pub position: Pt3,
    pub rotation: Option<Euler>,
    pub scale: Option<Euler>,
    pub material: DisneyMaterial,
}

#[derive(Debug)]
pub enum Shape {
    Sphere(Scalar),
}

impl<'de> DeserializeTrait<'de> for Shape {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Scalar::deserialize(deserializer).map(Shape::Sphere)
    }
}

impl<'de> DeserializeTrait<'de> for Hdri {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let path = String::deserialize(deserializer)?;
        let image = image::io::Reader::open(path).unwrap().decode().unwrap();
        Ok(Hdri::new(image.into_rgb32f()))
    }
}

#[derive(Debug, Deserialize)]
struct CameraRaw {
    pub position: Pt3,
    pub direction: Vec3,
    pub sensor_distance: Scalar,
    pub hdri: Hdri,
    pub hdri_bias: Option<[u32; 2]>,

    pub bounce_limit: usize,
    pub num_samples: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug)]
pub struct Camera {
    pub position: Pt3,
    pub direction: Vec3,
    pub sensor_distance: Scalar,
    pub hdri: Hdri,
    pub hdri_bias: Option<Vec3>,

    pub bounce_limit: usize,
    pub num_samples: usize,
    pub width: usize,
    pub height: usize,
}

impl<'de> DeserializeTrait<'de> for Camera {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let camera_raw: CameraRaw = CameraRaw::deserialize(deserializer)?;
        let hdri_bias = camera_raw.hdri_bias.map(|[x, y]| {
            let (width, height) = camera_raw.hdri.image.dimensions();
            // Theta = 0 := up, Theta = PI := down
            let (phi, theta) = (
                (x as Scalar / width as Scalar) * 2.0 * PI - PI,
                (y as Scalar / height as Scalar) * PI,
            );

            (Mat4::from_angle_y(Rad(phi))
                * Mat4::from_angle_x(Rad(theta))
                * vec3(0.0, 1.0, 0.0).extend(1.0))
            .truncate()
        });
        Ok(Camera {
            position: camera_raw.position,
            direction: camera_raw.direction.normalize(),
            sensor_distance: camera_raw.sensor_distance,
            hdri: camera_raw.hdri,
            hdri_bias,

            bounce_limit: camera_raw.bounce_limit,
            num_samples: camera_raw.num_samples,
            width: camera_raw.width,
            height: camera_raw.height,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct Scene {
    pub camera: Camera,
    pub objects: Vec<Object>,
}

pub fn load_scene<P: AsRef<Path>>(path: P) -> Scene {
    let source = std::fs::read_to_string(path).unwrap();
    let mut scene: Scene = toml::from_str(&source).unwrap();
    scene.camera.direction = scene.camera.direction.normalize();
    scene
}
