use crate::types::scalar::consts::{FRAC_PI_2, FRAC_PI_4};
use crate::types::{scalar, Pt2, Pt3, Scalar, Vec3};
use cgmath::{point2, vec2, vec3, EuclideanSpace, InnerSpace};

pub fn max_value3(v: Pt3) -> Scalar {
    if v[0] > v[1] && v[0] > v[2] {
        v[0]
    } else if v[1] > v[2] {
        v[1]
    } else {
        v[2]
    }
}

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

pub fn random_concentric_disk() -> Pt2 {
    let u = point2(scalar::rand() * 2.0 - 1.0, scalar::rand() * 2.0 - 1.0);
    if u == Pt2::origin() {
        Pt2::origin()
    } else {
        let (theta, r) = if u.x.abs() > u.y.abs() {
            (FRAC_PI_4 * (u.y / u.x), u.x)
        } else {
            (FRAC_PI_2 - FRAC_PI_4 * (u.x / u.y), u.y)
        };
        r * point2(theta.cos(), theta.sin())
    }
}

pub fn random_cos_sample_hemisphere() -> Vec3 {
    let d = random_concentric_disk();
    let z = (1.0 - d.x * d.x - d.y * d.y).max(0.0).sqrt();
    vec3(d.x, d.y, z)
}

pub fn reflect(vec: Vec3, reflector: Vec3) -> Vec3 {
    -vec + 2.0 * reflector * vec.dot(reflector)
}

pub fn spherical_direction(sin_theta: Scalar, cos_theta: Scalar, phi: Scalar) -> Vec3 {
    vec3(sin_theta * phi.cos(), sin_theta * phi.cos(), cos_theta)
}

#[cfg(test)]
mod tests {}

pub trait NormalBasisVector<S> {
    fn cos_theta(self) -> S;
    fn cos2_theta(self) -> S;
    fn abs_cos_theta(self) -> S;
    fn sin_theta(self) -> S;
    fn sin2_theta(self) -> S;
    fn tan_theta(self) -> S;
    fn tan2_theta(self) -> S;
    fn cos_phi(self) -> S;
    fn sin_phi(self) -> S;
    fn cos2_phi(self) -> S;
    fn sin2_phi(self) -> S;
    fn same_hemisphere(self, other: Vec3) -> bool;
}

impl NormalBasisVector<Scalar> for Vec3 {
    #[inline]
    fn cos_theta(self) -> Scalar {
        self.z
    }

    #[inline]
    fn cos2_theta(self) -> Scalar {
        self.z * self.z
    }

    #[inline]
    fn abs_cos_theta(self) -> Scalar {
        self.cos_theta().abs()
    }

    #[inline]
    fn sin_theta(self) -> Scalar {
        self.sin2_theta().sqrt()
    }

    #[inline]
    fn sin2_theta(self) -> Scalar {
        (1.0 - self.cos2_theta()).max(0.0)
    }

    #[inline]
    fn tan_theta(self) -> Scalar {
        self.sin_theta() / self.cos_theta()
    }

    #[inline]
    fn tan2_theta(self) -> Scalar {
        self.sin2_theta() / self.cos2_theta()
    }

    #[inline]
    fn cos_phi(self) -> Scalar {
        let sin_theta = self.sin_theta();
        if sin_theta == 0.0 {
            0.0
        } else {
            (self.x / sin_theta).clamp(-1.0, 1.0)
        }
    }

    #[inline]
    fn sin_phi(self) -> Scalar {
        let sin_theta = self.sin_theta();
        if sin_theta == 0.0 {
            0.0
        } else {
            (self.y / sin_theta).clamp(-1.0, 1.0)
        }
    }

    #[inline]
    fn cos2_phi(self) -> Scalar {
        self.cos_phi() * self.cos_phi()
    }

    #[inline]
    fn sin2_phi(self) -> Scalar {
        self.sin_phi() * self.sin_phi()
    }

    #[inline]
    fn same_hemisphere(self, other: Vec3) -> bool {
        self.z * other.z > 0.0
    }
}

macro_rules! bitfield_methods {
    ($ty_name: ident) => {
        impl $ty_name {
            #[inline(always)]
            pub const fn set(self, other: Self) -> Self {
                Self(self.0 | other.0)
            }

            #[inline(always)]
            pub const fn unset(self, other: Self) -> Self {
                Self(self.0 & !other.0)
            }

            #[inline(always)]
            pub const fn mask(self, other: Self) -> Self {
                Self(self.0 & other.0)
            }

            #[inline(always)]
            pub const fn not(self) -> Self {
                Self(!self.0)
            }

            #[inline(always)]
            pub const fn has(self, other: Self) -> bool {
                self.0 & other.0 != 0
            }

            #[inline(always)]
            pub const fn matches(self, other: Self) -> bool {
                (self.0 & other.0) == self.0
            }
        }
    };
}

pub(crate) use bitfield_methods;
