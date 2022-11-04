use crate::types::scalar::consts::PI;
use crate::types::{scalar, Pt2, Scalar, Vec3};
use crate::util::NormalBasisVector;
use cgmath::{vec3, InnerSpace};
use std::fmt::Debug;

pub trait Distribution: Sized + Copy + Debug {
    fn is_specular(self) -> bool;
    fn d(self, wh: Vec3) -> Scalar;
    fn lambda(self, w: Vec3) -> Scalar;

    fn g1(self, w: Vec3) -> Scalar {
        1.0 / (1.0 + self.lambda(w))
    }

    fn g(self, wo: Vec3, wi: Vec3) -> Scalar {
        1.0 / (1.0 + self.lambda(wo) + self.lambda(wi))
    }

    fn sample_wh(self, wo: Vec3) -> Vec3;

    fn pdf(self, wo: Vec3, wh: Vec3) -> Scalar {
        self.d(wh) * self.g1(wo) * wo.dot(wh).abs() / wo.abs_cos_theta()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TrowbridgeReitzDistribution {
    alpha: Pt2,
}

impl TrowbridgeReitzDistribution {
    #[inline]
    pub fn new(alpha: Pt2) -> TrowbridgeReitzDistribution {
        TrowbridgeReitzDistribution {
            alpha: alpha.map(|v| v.max(0.001)),
        }
    }
}

fn trowbridge_reitz_sample11(cos_theta: Scalar, u1: Scalar, mut u2: Scalar) -> (Scalar, Scalar) {
    if cos_theta > 0.9999 {
        let r = (u1 / (1.0 - u1)).sqrt();
        let phi = 2.0 * PI * u2;
        (r * phi.cos(), r * phi.sin())
    } else {
        let sin_theta = (1.0 - cos_theta.powi(2)).max(0.0).sqrt();
        let tan_theta = sin_theta / cos_theta;
        let a = 1.0 / tan_theta;
        let g1 = 2.0 / (1.0 + (1.0 + 1.0 / a.powi(2)).sqrt());

        let a = 2.0 * u1 / g1 - 1.0;
        let mut tmp = 1.0 / (a.powi(2) - 1.0);
        if tmp > 1.0e10 {
            tmp = 1.0e10
        }
        let b = tan_theta;
        let d = (b.powi(2) * tmp.powi(2) - (a.powi(2) - b.powi(2)) * tmp)
            .max(0.0)
            .sqrt();
        let slope_x_1 = b * tmp - d;
        let slope_x_2 = b * tmp + d;
        let slope_x = if a < 0.0 || slope_x_2 > 1.0 / tan_theta {
            slope_x_1
        } else {
            slope_x_2
        };

        let s = if u2 > 0.5 {
            u2 = 2.0 * (u2 - 0.5);
            1.0
        } else {
            u2 = 2.0 * (0.5 - u2);
            -1.0
        };
        let z = (u2 * (u2 * (u2 * 0.27385 - 0.73369) + 0.46341))
            / (u2 * (u2 * (u2 * 0.093073 + 0.309420) - 1.0) + 0.597999);
        let slope_y = s * z * (1.0 + slope_x.powi(2));
        assert!(slope_y.is_finite());
        (slope_x, slope_y)
    }
}

fn trowbridge_reitz_sample(
    wi: Vec3,
    alpha_x: Scalar,
    alpha_y: Scalar,
    u1: Scalar,
    u2: Scalar,
) -> Vec3 {
    let wi_stretched = vec3(alpha_x * wi.x, alpha_y * wi.y, wi.z).normalize();

    let (mut slope_x, mut slope_y) = trowbridge_reitz_sample11(wi_stretched.cos_theta(), u1, u2);

    let tmp = wi_stretched.cos_phi() * slope_x - wi_stretched.sin_phi() * slope_y;
    slope_y = wi_stretched.sin_phi() * slope_x + wi_stretched.cos_phi() * slope_y;
    slope_x = tmp;

    slope_x *= alpha_x;
    slope_y *= alpha_y;

    vec3(-slope_x, -slope_y, 1.0).normalize()
}

impl Distribution for TrowbridgeReitzDistribution {
    #[inline]
    fn is_specular(self) -> bool {
        self.alpha.x < 0.04 && self.alpha.y < 0.04
    }

    #[inline]
    fn d(self, wh: Vec3) -> Scalar {
        let tan2_theta = wh.tan2_theta();
        if tan2_theta.is_infinite() {
            0.0
        } else {
            let cos2g_theta = wh.cos2_theta().powi(2);
            let e = (wh.cos2_phi() / self.alpha.x.powi(2) + wh.sin2_phi() / self.alpha.y.powi(2))
                * tan2_theta;
            1.0 / (PI * self.alpha.x * self.alpha.y * cos2g_theta * (1.0 + e).powi(2))
        }
    }

    #[inline]
    fn lambda(self, w: Vec3) -> Scalar {
        let abs_tan_theta = w.tan_theta().abs();
        if abs_tan_theta.is_infinite() {
            0.0
        } else {
            let alpha = w.cos2_phi() * self.alpha.x.powi(2) * w.sin2_phi() * self.alpha.y.powi(2);
            let alpha2_tan2_theta = (alpha * abs_tan_theta).powi(2);
            (-1.0 + (1.0 + alpha2_tan2_theta).sqrt()) / 2.0
        }
    }

    #[inline]
    fn sample_wh(self, wo: Vec3) -> Vec3 {
        let u_0 = scalar::rand();
        let u_1 = scalar::rand();

        let flip = wo.z < 0.0;
        let wh = trowbridge_reitz_sample(
            if flip { -wo } else { wo },
            self.alpha.x,
            self.alpha.y,
            u_0,
            u_1,
        );
        if flip {
            -wh
        } else {
            wh
        }
    }
}
