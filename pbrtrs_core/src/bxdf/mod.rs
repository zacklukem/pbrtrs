pub mod distribution;

pub use distribution::*;
use std::fmt::{Debug, Formatter};

use crate::bxdf::distribution::Distribution;
use crate::debugger;
use crate::intersect::Intersection;
use crate::material::TransportMode;
use crate::types::color::BLACK;
use crate::types::scalar::consts::{FRAC_1_PI, PI};
use crate::types::{color, scalar, Color, Scalar, Vec3};
use crate::util::{
    bitfield_methods, random_cos_sample_hemisphere, random_unit_vec, reflect, NormalBasisVector,
};
use cgmath::{point3, vec3, Array, ElementWise, InnerSpace, One, Zero};
use smallvec::SmallVec;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct BxDFKind(u8);

impl Debug for BxDFKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BxDFKind(")?;
        let kinds = [
            ("REFLECTION", Self::REFLECTION),
            ("TRANSMISSION", Self::TRANSMISSION),
            ("DIFFUSE", Self::DIFFUSE),
            ("GLOSSY", Self::GLOSSY),
            ("SPECULAR", Self::SPECULAR),
        ];
        for (kind_str, kind) in kinds {
            if self.has(kind) {
                write!(f, "{} ", kind_str)?;
            }
        }
        write!(f, ")")
    }
}

impl BxDFKind {
    pub const REFLECTION: BxDFKind = BxDFKind(1 << 0);
    pub const TRANSMISSION: BxDFKind = BxDFKind(1 << 1);
    pub const DIFFUSE: BxDFKind = BxDFKind(1 << 2);
    pub const GLOSSY: BxDFKind = BxDFKind(1 << 3);
    pub const SPECULAR: BxDFKind = BxDFKind(1 << 4);
    pub const ALL: BxDFKind = Self::DIFFUSE
        .set(Self::GLOSSY)
        .set(Self::SPECULAR)
        .set(Self::REFLECTION)
        .set(Self::TRANSMISSION);
}

bitfield_methods!(BxDFKind);

pub trait BxDF: Debug {
    fn kind(&self) -> BxDFKind;

    fn f(&self, wo: Vec3, wi: Vec3) -> Color;

    fn sample_f(
        &self,
        wo: Vec3,
        wi: &mut Vec3,
        pdf: &mut Scalar,
        sampled_kind: &mut BxDFKind,
    ) -> Color {
        *sampled_kind = self.kind();
        *wi = random_cos_sample_hemisphere();
        wi.z = wi.z.abs();
        if wo.z < 0.0 {
            wi.z *= -1.0
        }
        *pdf = self.pdf(wo, *wi);
        self.f(wo, *wi)
    }

    fn rho(&self, _wo: Vec3, _samples: &[[Scalar; 2]]) -> Color {
        unimplemented!()
    }

    fn rho2(&self, _samples1: &[[Scalar; 2]], _samples2: &[[Scalar; 2]]) -> Color {
        unimplemented!()
    }

    fn pdf(&self, wo: Vec3, wi: Vec3) -> Scalar {
        if wo.same_hemisphere(wi) {
            wi.abs_cos_theta() * FRAC_1_PI
        } else {
            0.0
        }
    }

    fn scale(self, scale: Scalar) -> ScaledBxDF<Self>
    where
        Self: Sized,
    {
        ScaledBxDF(scale, self)
    }
}

#[derive(Debug)]
pub struct ScaledBxDF<B: BxDF>(Scalar, B);

impl<B: BxDF> BxDF for ScaledBxDF<B> {
    fn kind(&self) -> BxDFKind {
        self.1.kind()
    }

    #[inline]
    fn f(&self, wo: Vec3, wi: Vec3) -> Color {
        self.0 * self.1.f(wo, wi)
    }

    #[inline]
    fn sample_f(
        &self,
        wo: Vec3,
        wi: &mut Vec3,
        pdf: &mut Scalar,
        sampled_kind: &mut BxDFKind,
    ) -> Color {
        self.0 * self.1.sample_f(wo, wi, pdf, sampled_kind)
    }
}

#[derive(Debug)]
pub struct Lambertian(pub Color);

impl BxDF for Lambertian {
    fn kind(&self) -> BxDFKind {
        BxDFKind::DIFFUSE.set(BxDFKind::REFLECTION)
    }

    fn f(&self, _wo: Vec3, _wi: Vec3) -> Color {
        self.0 / PI
    }

    fn rho(&self, _wo: Vec3, _samples: &[[Scalar; 2]]) -> Color {
        self.0
    }

    fn rho2(&self, _samples1: &[[Scalar; 2]], _samples2: &[[Scalar; 2]]) -> Color {
        self.0
    }
}

#[inline]
fn fr_schlick(r0: Color, cos_i: Scalar) -> Color {
    // theta_i is the angle between wi and wo
    // theta_d is the angle between the half vector and wo, which in perfect specular reflection
    // is the same as theta_i / 2
    // Get cos_d with half angle identity
    let cos_d = ((cos_i + 1.0) / 2.0).sqrt();
    r0 + (Color::from_value(1.0) - r0) * (1.0 - cos_d).clamp(0.0, 1.0).powi(5)
}

#[inline]
fn fr_dielectric(mut cos_i: Scalar, mut eta_i: Scalar, mut eta_t: Scalar) -> Scalar {
    let entering = cos_i > 0.0;
    if !entering {
        std::mem::swap(&mut eta_i, &mut eta_t);
        cos_i = cos_i.abs()
    }

    let sin_i = (1.0 - cos_i * cos_i).max(0.0).sqrt();
    let sin_t = eta_i / eta_t * sin_i;
    if sin_t >= 1.0 {
        return 1.0;
    }

    let cos_t = (1.0 - sin_t * sin_t).max(0.0).sqrt();

    let r_parl = ((eta_t * cos_i) - (eta_i * cos_t)) / ((eta_t * cos_i) + (eta_i * cos_t));
    let r_perp = ((eta_i * cos_i) - (eta_t * cos_t)) / ((eta_i * cos_i) + (eta_t * cos_t));

    (r_parl.powi(2) + r_perp.powi(2)) / 2.0
}

fn schlick_r0_from_eta(eta: Scalar) -> Scalar {
    (eta - 1.0).powi(2) / (eta + 1.0).powi(2)
}

pub trait Fresnel: Sized + Copy + Debug {
    fn f(self, cos_i: Scalar) -> Color;
}

#[derive(Copy, Clone, Debug)]
pub struct DisneyFresnel {
    pub eta: Scalar,
    pub r0: Color,
    pub metallic: Scalar,
}

impl Fresnel for DisneyFresnel {
    fn f(self, cos_i: Scalar) -> Color {
        let schlick = fr_schlick(self.r0, cos_i);
        let dielectric = fr_dielectric(cos_i, 1.0, self.eta);
        color::mix(Color::from_value(dielectric), schlick, self.metallic)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FresnelDielectric {
    pub eta_i: Scalar,
    pub eta_t: Scalar,
}

impl Fresnel for FresnelDielectric {
    fn f(self, cos_i: Scalar) -> Color {
        Color::from_value(fr_dielectric(cos_i, self.eta_i, self.eta_t))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FresnelSchlick(pub Color);

impl Fresnel for FresnelSchlick {
    #[inline]
    fn f(self, cos_i: Scalar) -> Color {
        fr_schlick(self.0, cos_i)
    }
}

#[derive(Debug)]
pub struct TransmissionSpecular<F> {
    pub color: Color,
    pub eta_a: Scalar,
    pub eta_b: Scalar,
    pub fresnel: F,
    pub transport_mode: TransportMode,
}

fn refract(wi: Vec3, normal: Vec3, eta: Scalar) -> Option<Vec3> {
    let cos_theta_i = normal.dot(wi);
    let sin2_theta_i = (1.0 - cos_theta_i.powi(2)).max(0.0);
    let sin2_theta_t = eta.powi(2) * sin2_theta_i;
    if sin2_theta_t >= 1.0 {
        None
    } else {
        let cos_theta_t = (1.0 - sin2_theta_t).sqrt();
        Some(eta * -wi + (eta * cos_theta_i - cos_theta_t) * normal)
    }
}

fn faceforward(n: Vec3, v: Vec3) -> Vec3 {
    if n.dot(v) < 0.0 {
        -n
    } else {
        n
    }
}

impl<F: Fresnel> BxDF for TransmissionSpecular<F> {
    fn kind(&self) -> BxDFKind {
        BxDFKind::TRANSMISSION.set(BxDFKind::SPECULAR)
    }

    fn f(&self, _wo: Vec3, _wi: Vec3) -> Color {
        BLACK
    }

    fn sample_f(
        &self,
        wo: Vec3,
        wi: &mut Vec3,
        pdf: &mut Scalar,
        sampled_kind: &mut BxDFKind,
    ) -> Color {
        *sampled_kind = self.kind();
        *pdf = 1.0;
        let entering = wo.cos_theta() > 0.0;
        let eta_frac = if entering {
            self.eta_a / self.eta_b
        } else {
            self.eta_b / self.eta_a
        };

        *wi = if let Some(wi) = refract(wo, faceforward(vec3(0.0, 0.0, 1.0), wo), eta_frac) {
            wi
        } else {
            return BLACK;
        };

        let mut ft = self.color.mul_element_wise(
            point3(1.0, 1.0, 1.0).sub_element_wise(self.fresnel.f(wi.cos_theta())),
        );
        if self.transport_mode == TransportMode::Radiance {
            ft *= eta_frac.powi(2);
        }
        ft / wi.abs_cos_theta()
    }

    fn pdf(&self, _wo: Vec3, _wi: Vec3) -> Scalar {
        0.0
    }
}

#[derive(Debug)]
pub struct ReflectionSpecular<F> {
    pub color: Color,
    pub fresnel: F,
}

impl<F: Fresnel> BxDF for ReflectionSpecular<F> {
    fn kind(&self) -> BxDFKind {
        BxDFKind::REFLECTION.set(BxDFKind::SPECULAR)
    }

    fn f(&self, _wo: Vec3, _wi: Vec3) -> Color {
        BLACK
    }

    fn sample_f(
        &self,
        wo: Vec3,
        wi: &mut Vec3,
        pdf: &mut Scalar,
        sampled_kind: &mut BxDFKind,
    ) -> Color {
        *sampled_kind = self.kind();
        *wi = vec3(-wo.x, -wo.y, wo.z);
        *pdf = 1.0;
        self.fresnel.f(wi.cos_theta()).mul_element_wise(self.color) / wi.abs_cos_theta()
    }

    fn pdf(&self, _wo: Vec3, _wi: Vec3) -> Scalar {
        0.0
    }
}

#[derive(Debug)]
pub struct FresnelSpecular {
    pub color: Color,
    pub eta_a: Scalar,
    pub eta_b: Scalar,
    pub transport_mode: TransportMode,
}

impl BxDF for FresnelSpecular {
    fn kind(&self) -> BxDFKind {
        BxDFKind::REFLECTION
            .set(BxDFKind::TRANSMISSION)
            .set(BxDFKind::SPECULAR)
    }

    fn f(&self, _wo: Vec3, _wi: Vec3) -> Color {
        BLACK
    }

    fn sample_f(
        &self,
        wo: Vec3,
        wi: &mut Vec3,
        pdf: &mut Scalar,
        sampled_kind: &mut BxDFKind,
    ) -> Color {
        let f = fr_dielectric(wo.cos_theta(), self.eta_a, self.eta_b);
        if scalar::rand() < f {
            *wi = vec3(-wo.x, -wo.y, wo.z);
            *sampled_kind = BxDFKind::REFLECTION.set(BxDFKind::SPECULAR);
            *pdf = f;
            self.color.mul_element_wise(f / wi.abs_cos_theta())
        } else {
            *pdf = 1.0 - f;
            *sampled_kind = BxDFKind::TRANSMISSION.set(BxDFKind::SPECULAR);

            let entering = wo.cos_theta() > 0.0;
            let eta_frac = if entering {
                self.eta_a / self.eta_b
            } else {
                self.eta_b / self.eta_a
            };

            *wi = if let Some(wi) = refract(wo, faceforward(vec3(0.0, 0.0, 1.0), wo), eta_frac) {
                wi
            } else {
                return BLACK;
            };

            let mut ft = self.color.mul_element_wise(1.0 - f);
            if self.transport_mode == TransportMode::Radiance {
                ft *= eta_frac.powi(2);
            }
            ft / wi.abs_cos_theta()
        }
    }

    fn pdf(&self, _wo: Vec3, _wi: Vec3) -> Scalar {
        0.0
    }
}

/// Microfacet reflection
#[derive(Debug)]
pub struct MicrofacetReflection<D, F> {
    pub color: Color,
    pub distribution: D,
    pub fresnel: F,
}

impl<D: Distribution, F: Fresnel> BxDF for MicrofacetReflection<D, F> {
    fn kind(&self) -> BxDFKind {
        if self.distribution.is_specular() {
            BxDFKind::REFLECTION
                .set(BxDFKind::GLOSSY)
                .set(BxDFKind::SPECULAR)
        } else {
            BxDFKind::REFLECTION.set(BxDFKind::GLOSSY)
        }
    }

    fn f(&self, wo: Vec3, wi: Vec3) -> Color {
        let cos_theta_o = wo.cos_theta();
        let cos_theta_i = wi.cos_theta();
        let wh = wo + wi;
        if cos_theta_i == 0.0 || cos_theta_o == 0.0 || (wh.x <= 0.0 && wh.y == 0.0 && wh.z == 0.0) {
            BLACK
        } else {
            let wh = wh.normalize();
            let dfg =
                self.distribution.d(wh) * self.distribution.g(wo, wi) * self.fresnel.f(wi.dot(wo));
            debugger::ray_debug! {
                cos_theta_o,
                cos_theta_i,
                wh,
                dfg
            }
            dfg.mul_element_wise(self.color) / (4.0 * cos_theta_i * cos_theta_o)
        }
    }

    fn sample_f(
        &self,
        wo: Vec3,
        wi: &mut Vec3,
        pdf: &mut Scalar,
        sampled_kind: &mut BxDFKind,
    ) -> Color {
        *sampled_kind = self.kind();
        let wh = self.distribution.sample_wh(wo);
        *wi = reflect(wo, wh);
        if !wo.same_hemisphere(*wi) {
            BLACK
        } else {
            *pdf = self.distribution.pdf(wo, wh) / (4.0 * wo.dot(wh));
            self.f(wo, *wi)
        }
    }

    fn pdf(&self, wo: Vec3, wi: Vec3) -> Scalar {
        if !wo.same_hemisphere(wi) {
            0.0
        } else {
            let wh = (wo + wi).normalize();
            self.distribution.pdf(wo, wh) / (4.0 * wo.dot(wh))
        }
    }
}

pub struct BSDF<'arena> {
    bxdfs: SmallVec<[&'arena dyn BxDF; 8]>,
    surface_normal: Vec3,
    geom_normal: Vec3,
    surface_tangent: Vec3,
    surface_cotangent: Vec3,
}

impl<'arena> BSDF<'arena> {
    pub fn new<'a, M, O>(intersect: &Intersection<M, O>) -> BSDF<'a> {
        let geom_normal = intersect.normal;
        let surface_normal = intersect.normal; // TODO: make this right
        let surface_tangent = intersect.tangent; // TODO: <<<<<
        let surface_cotangent = surface_normal.cross(surface_tangent).normalize();

        BSDF {
            bxdfs: SmallVec::new(),
            surface_normal,
            surface_tangent,
            surface_cotangent,
            geom_normal,
        }
    }

    pub fn add(&mut self, bxdf: &'arena dyn BxDF) {
        self.bxdfs.push(bxdf);
    }

    pub fn world_to_normal(&self, v: Vec3) -> Vec3 {
        vec3(
            v.dot(self.surface_cotangent),
            v.dot(self.surface_tangent),
            v.dot(self.surface_normal),
        )
    }

    #[rustfmt::skip]
    pub fn normal_to_world(&self, v: Vec3) -> Vec3 {
        vec3(
            self.surface_cotangent.x * v.x + self.surface_tangent.x * v.y + self.surface_normal.x * v.z,
            self.surface_cotangent.y * v.x + self.surface_tangent.y * v.y + self.surface_normal.y * v.z,
            self.surface_cotangent.z * v.x + self.surface_tangent.z * v.y + self.surface_normal.z * v.z
        )
    }

    pub fn num_components(&self, kind: BxDFKind) -> usize {
        self.bxdfs
            .iter()
            .filter(|bxdf| bxdf.kind().matches(kind))
            .count()
    }

    pub fn f(&self, wo: Vec3, wi: Vec3, kind: BxDFKind) -> Color {
        let reflect = wi.dot(self.geom_normal) * wo.dot(self.geom_normal) > 0.0;
        let wo = self.world_to_normal(wo);
        let wi = self.world_to_normal(wi);
        self.f_normal_space(wo, wi, reflect, kind)
    }

    #[inline]
    fn f_normal_space(&self, wo: Vec3, wi: Vec3, reflect: bool, kind: BxDFKind) -> Color {
        let mut f = BLACK;
        self.bxdfs
            .iter()
            .filter(|bxdf| {
                bxdf.kind().matches(kind)
                    && ((reflect && bxdf.kind().has(BxDFKind::REFLECTION))
                        || (!reflect && bxdf.kind().has(BxDFKind::TRANSMISSION)))
            })
            .for_each(|bxdf| {
                let f_b = bxdf.f(wo, wi);
                f.add_assign_element_wise(f_b);
            });
        f
    }

    pub fn sample_f(
        &self,
        wo_world: Vec3,
        wi_world: &mut Vec3,
        pdf: &mut Scalar,
        sampled_kind: &mut BxDFKind,
        kind: BxDFKind,
    ) -> Color {
        let num_matching = self.num_components(kind);
        if num_matching == 0 {
            *pdf = 0.0;
            return BLACK;
        }

        // Choose a random bxdf
        let comp =
            ((scalar::rand() * num_matching as Scalar).floor() as usize).min(num_matching - 1);

        let (bxdf_index, bxdf) = self
            .bxdfs
            .iter()
            .enumerate()
            .filter(|(_, bxdf)| bxdf.kind().matches(kind))
            .nth(comp)
            .unwrap();

        *pdf = 0.0;

        let wo = self.world_to_normal(wo_world);
        if wo.z == 0.0 {
            return BLACK;
        }
        let mut wi = Vec3::zero();
        let mut f = bxdf.sample_f(wo, &mut wi, pdf, sampled_kind);
        *wi_world = self.normal_to_world(wi);

        if !bxdf.kind().has(BxDFKind::SPECULAR) {
            for (i, bxdf) in self.bxdfs.iter().enumerate() {
                if i != bxdf_index && bxdf.kind().matches(kind) {
                    *pdf += bxdf.pdf(wo, wi);
                }
            }
            *pdf /= num_matching as Scalar;

            if num_matching > 1 {
                f = BLACK;
                let reflect = wi_world.dot(self.geom_normal) * wo_world.dot(self.geom_normal) > 0.0;
                f.add_assign_element_wise(self.f_normal_space(wo, wi, reflect, kind));
            }
        }
        f
    }

    pub fn rho(&self, wo: Vec3, samples: &[[Scalar; 2]], kind: BxDFKind) -> Color {
        self.bxdfs
            .iter()
            .filter(|bxdf| bxdf.kind().matches(kind))
            .fold(BLACK, |rho, bxdf| {
                rho.add_element_wise(bxdf.rho(wo, samples))
            })
    }

    pub fn rho2(
        &self,
        samples1: &[[Scalar; 2]],
        samples2: &[[Scalar; 2]],
        kind: BxDFKind,
    ) -> Color {
        self.bxdfs
            .iter()
            .filter(|bxdf| bxdf.kind().matches(kind))
            .fold(BLACK, |rho, bxdf| {
                rho.add_element_wise(bxdf.rho2(samples1, samples2))
            })
    }

    pub fn pdf(&self, wo: Vec3, wi: Vec3, kind: BxDFKind) -> Scalar {
        let (count, pdf) = self
            .bxdfs
            .iter()
            .filter(|bxdf| bxdf.kind().matches(kind))
            .fold((0, 0.0), |(count, pdf), bxdf| {
                (count + 1, pdf + bxdf.pdf(wo, wi))
            });
        pdf / count as Scalar
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::EmptyMaterial;
    use crate::scene::Shape;
    use crate::types::{Euler, Mat4, Pt3, Quaternion, Ray};
    use cgmath::{assert_abs_diff_eq, point3, EuclideanSpace, Rad, SquareMatrix};

    #[test]
    fn bsdf_world_to_normal() {
        // test if world_to_normal and normal_to_world are inverses
        macro_rules! test_direction {
            ($direction: expr) => {
                let shape = Shape::Sphere { radius: 1.0 };
                let si = shape
                    .intersect(
                        &Ray::new(Pt3::from_vec($direction * 10.0), -$direction, 0.0),
                        Quaternion::zero(),
                        vec3(0.0, 0.0, 0.0),
                        &EmptyMaterial,
                        &(),
                    )
                    .unwrap_into();

                let bsdf = BSDF::new(&si);
                assert_abs_diff_eq!(
                    bsdf.world_to_normal(si.normal),
                    vec3(0.0, 0.0, 1.0),
                    epsilon = 1e-6,
                );
                assert_abs_diff_eq!(
                    bsdf.normal_to_world(vec3(0.0, 0.0, 1.0)),
                    si.normal,
                    epsilon = 1e-6,
                );

                assert_abs_diff_eq!(
                    bsdf.world_to_normal(si.tangent),
                    vec3(0.0, 1.0, 0.0),
                    epsilon = 1e-6,
                );
                assert_abs_diff_eq!(
                    bsdf.normal_to_world(vec3(0.0, 1.0, 0.0)),
                    si.tangent,
                    epsilon = 1e-6,
                );
            };
        }
        test_direction!(vec3(1.0, 0.0, 0.0));
        test_direction!(vec3(-1.0, 0.0, 0.0));
        test_direction!(vec3(0.0, 1.0, 0.0));
        test_direction!(vec3(0.0, -1.0, 0.0));
        test_direction!(vec3(0.0, 0.0, 1.0));
        test_direction!(vec3(0.0, 0.0, -1.0));
        for i in 1..100 {
            for j in 1..100 {
                let phi = (i as Scalar * 0.02) * PI;
                let theta = (j as Scalar * 0.01) * PI;

                let dir = vec3(phi.cos(), phi.sin(), theta.cos()).normalize();

                test_direction!(dir);
            }
        }
    }
}
