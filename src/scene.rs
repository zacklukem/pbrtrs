use crate::types::scalar::consts::PI;
use crate::types::{color, Color, Euler, Mat4, Pt2, Pt3, Scalar, Vec3};

use cgmath::{vec3, EuclideanSpace, InnerSpace, Rad};
use image::Rgb32FImage;
use std::fmt::{Debug, Formatter};

use std::path::Path;

use serde::{Deserialize as DeserializeTrait, Deserializer};
use serde_derive::Deserialize;

#[derive(Debug)]
pub enum Texture<T> {
    Value(T),
}

impl<T: Default> Default for Texture<T> {
    fn default() -> Self {
        Self::Value(Default::default())
    }
}

impl<T: Copy> Texture<T> {
    pub fn get(&self, _uv: Pt2) -> T {
        match self {
            Self::Value(value) => *value,
        }
    }
}

impl<'de, T: DeserializeTrait<'de>> DeserializeTrait<'de> for Texture<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self::Value)
    }
}

#[derive(Debug, Deserialize)]
pub struct Material {
    pub base_color: Texture<Color>,
    pub subsurface: Texture<Scalar>,
    pub metallic: Texture<Scalar>,
    pub specular: Texture<Scalar>,
    pub specular_tint: Texture<Scalar>,
    pub roughness: Texture<Scalar>,
    pub anisotropic: Texture<Scalar>,
    pub sheen: Texture<Scalar>,
    pub sheen_tint: Texture<Scalar>,
    pub clearcoat: Texture<Scalar>,
    pub clearcoat_gloss: Texture<Scalar>,
}

#[derive(Debug)]
pub struct SampledMaterial {
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

impl Material {
    pub fn sample(&self, uv: Pt2) -> SampledMaterial {
        SampledMaterial {
            base_color: self.base_color.get(uv),
            subsurface: self.subsurface.get(uv),
            metallic: self.metallic.get(uv),
            specular: self.specular.get(uv),
            specular_tint: self.specular_tint.get(uv),
            roughness: self.roughness.get(uv),
            anisotropic: self.anisotropic.get(uv),
            sheen: self.sheen.get(uv),
            sheen_tint: self.sheen_tint.get(uv),
            clearcoat: self.clearcoat.get(uv),
            clearcoat_gloss: self.clearcoat_gloss.get(uv),
        }
    }
}

impl Default for Material {
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
    pub material: Material,
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

pub struct Hdri(pub Rgb32FImage);

impl Hdri {
    pub fn in_direction(&self, direction: Vec3) -> Color {
        let theta = direction.angle(vec3(0.0, 1.0, 0.0)).0 / PI;
        let phi = (direction.x.atan2(direction.z) + PI) / (2.0 * PI);
        let x = ((self.0.width() as Scalar * phi) as u32).min(self.0.width() - 1);
        let y = ((self.0.height() as Scalar * theta) as u32).min(self.0.height() - 1);
        let [r, g, b] = self.0.get_pixel(x, y).0;
        color(r, g, b)
    }
}

impl Debug for Hdri {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[hdri]")
    }
}

impl<'de> DeserializeTrait<'de> for Hdri {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let path = String::deserialize(deserializer)?;
        let image = image::io::Reader::open(path).unwrap().decode().unwrap();
        Ok(Hdri(image.into_rgb32f()))
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
            let (width, height) = camera_raw.hdri.0.dimensions();
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
