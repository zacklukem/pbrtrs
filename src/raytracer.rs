use crate::scene::Scene;
use crate::types::color::{BLACK, WHITE};
use crate::types::scalar::consts::PI;
use crate::util::random_unit_vec;
use crate::{Color, Ray, MAX_BOUNCE};
use cgmath::{ElementWise, InnerSpace};

pub fn ray_color(ray: &Ray, scene: &Scene, depth: usize) -> Color {
    if depth >= MAX_BOUNCE {
        BLACK
    } else if let Some(intersection) = scene.intersect(ray) {
        let bounce_ray = Ray::new(intersection.point, intersection.normal + random_unit_vec());

        let radiance_in = ray_color(&bounce_ray, scene, depth + 1);

        let f_d = intersection.material.base_color.get(intersection.uv) / PI;

        // Radiance out
        radiance_in.mul_element_wise(f_d) * intersection.normal.dot(bounce_ray.direction)
    } else {
        WHITE
    }
}
