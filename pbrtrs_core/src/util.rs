use crate::types::Vec3;
use cgmath::{vec3, InnerSpace};

pub fn random_vec() -> Vec3 {
    vec3(
        fastrand::f32() * 2.0 - 1.0,
        fastrand::f32() * 2.0 - 1.0,
        fastrand::f32() * 2.0 - 1.0,
    )
}

pub fn random_in_unit_sphere() -> Vec3 {
    let mut out = random_vec();
    while out.magnitude2() > 1.0 {
        out = random_vec();
    }
    out
}

pub fn random_unit_vec() -> Vec3 {
    random_in_unit_sphere().normalize()
}

#[cfg(test)]
mod tests {}
