#![allow(unused)]

use cgmath::{InnerSpace, Matrix2, Matrix3, Matrix4, Point2, Point3, Vector2, Vector3, Vector4};
use image::{Luma, LumaA, Pixel, Rgb, Rgba};
use std::ops::Index;

pub type Scalar = f32;

pub mod scalar {
    use super::Scalar;
    pub use std::f32::consts;

    pub fn rand() -> Scalar {
        fastrand::f32()
    }
}

pub type Basis2 = cgmath::Basis2<Scalar>;
pub type Basis3 = cgmath::Basis3<Scalar>;

pub type Mat2 = Matrix2<Scalar>;
pub type Mat3 = Matrix3<Scalar>;
pub type Mat4 = Matrix4<Scalar>;

pub type Vec2 = Vector2<Scalar>;
pub type Vec3 = Vector3<Scalar>;
pub type Vec4 = Vector4<Scalar>;

pub type Pt2 = Point2<Scalar>;
pub type Pt3 = Point3<Scalar>;

pub type Quaternion = cgmath::Quaternion<Scalar>;
pub type Euler = cgmath::Euler<Scalar>;

pub type Color = Pt3;

#[inline]
pub const fn color(r: Scalar, g: Scalar, b: Scalar) -> Color {
    Color::new(r, g, b)
}

pub mod color {
    use super::{color, Color, Scalar};
    use cgmath::ElementWise;

    pub const WHITE: Color = color(1.0, 1.0, 1.0);
    pub const BLACK: Color = color(0.0, 0.0, 0.0);
    pub const RED: Color = color(1.0, 0.0, 0.0);
    pub const GREEN: Color = color(0.0, 1.0, 0.0);
    pub const BLUE: Color = color(0.0, 0.0, 1.0);

    pub fn mix(a: Color, b: Color, value: Scalar) -> Color {
        let value = value.clamp(0.0, 1.0);
        (a * (1.0 - value)).add_element_wise(b * value)
    }
}

#[derive(Debug)]
pub struct Ray {
    pub origin: Pt3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Pt3, direction: Vec3) -> Ray {
        Ray {
            origin,
            direction: direction.normalize(),
        }
    }

    pub fn new_no_normalize(origin: Pt3, direction: Vec3) -> Ray {
        debug_assert_eq!(direction.magnitude2(), 1.0);
        Ray { origin, direction }
    }

    pub fn at(&self, t: Scalar) -> Pt3 {
        self.origin + self.direction * t
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct R8G8B8Color([u8; 3]);

impl Index<usize> for R8G8B8Color {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl From<R8G8B8Color> for Rgb<u8> {
    fn from(value: R8G8B8Color) -> Self {
        Rgb(value.0)
    }
}

impl From<Color> for R8G8B8Color {
    fn from(value: Color) -> R8G8B8Color {
        let value = value.map(|el| {
            let el = el.clamp(0.0, 1.0);
            (el * 256.0).floor().clamp(0.0, 255.0) as u8
        });
        R8G8B8Color(value.into())
    }
}

impl IntoIterator for R8G8B8Color {
    type Item = u8;
    type IntoIter = std::array::IntoIter<u8, 3>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_to_r8g8b8() {
        assert_eq!(
            R8G8B8Color::from(color(1.0, 1.0, 1.0)),
            R8G8B8Color([255, 255, 255])
        );
        assert_eq!(
            R8G8B8Color::from(color(0.0, 0.0, 0.0)),
            R8G8B8Color([0, 0, 0])
        );
        assert_eq!(
            R8G8B8Color::from(color(0.5, 0.5, 0.5)),
            R8G8B8Color([128, 128, 128])
        );
    }
}
