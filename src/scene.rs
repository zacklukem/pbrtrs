use crate::types::{Color, Euler, Pt2, Pt3, Rad, Scalar, Vec3};
use cgmath::InnerSpace;
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

#[derive(Debug, Deserialize)]
pub struct Camera {
    pub position: Pt3,
    pub direction: Vec3,
    pub sensor_distance: Scalar,
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
