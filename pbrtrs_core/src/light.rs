use crate::bxdf::{BxDFKind, BSDF};
use crate::debugger;
use crate::intersect::Intersection;
use crate::light::hdri::Hdri;
use crate::material::{Material, TransportMode};
use crate::scene::{Scene, Shape};
use crate::types::color::{BLACK};
use crate::types::scalar::consts::PI;
use crate::types::{Color, Pt2, Pt3, Quaternion, Ray, Scalar, Vec3};
use crate::util::{bitfield_methods, random_unit_vec};
use bumpalo::Bump;
use cgmath::{ElementWise, InnerSpace, Zero};
use std::fmt::{Debug, Formatter};

pub mod hdri;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct LightKind(u8);

impl Debug for LightKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "LightKind(")?;
        let kinds = [
            ("DELTA_POSITION", Self::DELTA_POSITION),
            ("DELTA_DIRECTION", Self::DELTA_DIRECTION),
            ("AREA", Self::AREA),
            ("INFINITE", Self::INFINITE),
            ("NO_BG", Self::NO_BG),
        ];
        for (kind_str, kind) in kinds {
            if self.has(kind) {
                write!(f, "{} ", kind_str)?;
            }
        }
        write!(f, ")")
    }
}

impl LightKind {
    pub const DELTA_POSITION: LightKind = LightKind(1 << 0);
    pub const DELTA_DIRECTION: LightKind = LightKind(1 << 1);
    pub const AREA: LightKind = LightKind(1 << 2);
    pub const INFINITE: LightKind = LightKind(1 << 3);
    pub const NO_BG: LightKind = LightKind(1 << 4);
}

bitfield_methods!(LightKind);

pub trait LightTrait {
    fn kind(&self) -> LightKind;

    fn le(&self, wi: &Ray) -> Color;

    fn sample_li<M, O>(
        &self,
        intersection: &Intersection<M, O>,
        wi: &mut Vec3,
        pdf: &mut Scalar,
    ) -> Color;

    fn pdf_li<M, O>(&self, intersection: &Intersection<M, O>, wi: Vec3) -> Scalar;

    fn is_delta(&self) -> bool {
        self.kind().has(LightKind::DELTA_POSITION) || self.kind().has(LightKind::DELTA_DIRECTION)
    }

    fn is_area(&self) -> bool {
        self.kind().has(LightKind::AREA)
    }
}

pub fn power_heuristic(nf: Scalar, f_pdf: Scalar, ng: Scalar, g_pdf: Scalar) -> Scalar {
    let f = nf * f_pdf;
    let g = ng * g_pdf;
    (f * f) / (f * f + g * g)
}

#[derive(Debug)]
pub struct PointLight {
    pub position: Pt3,
    pub radiance: Color,
}

impl LightTrait for PointLight {
    fn kind(&self) -> LightKind {
        LightKind::DELTA_POSITION
    }

    fn le(&self, _wi: &Ray) -> Color {
        BLACK
    }

    fn sample_li<M, O>(
        &self,
        intersection: &Intersection<M, O>,
        wi: &mut Vec3,
        pdf: &mut Scalar,
    ) -> Color {
        let to_light = self.position - intersection.point;
        let distance = to_light.magnitude();
        *wi = to_light / distance;
        *pdf = 1.0;
        self.radiance / (distance + 1.0).powi(2)
    }

    fn pdf_li<M, O>(&self, _intersection: &Intersection<M, O>, _wi: Vec3) -> Scalar {
        0.0
    }
}

#[derive(Debug)]
pub struct SpotLight {
    pub position: Pt3,
    pub direction: Vec3,
    pub cos_angle: Scalar,
    pub cos_falloff: Scalar,
    pub radiance: Color,
}

impl SpotLight {
    fn falloff(&self, cos_theta: Scalar) -> Scalar {
        if cos_theta < self.cos_angle {
            0.0
        } else if cos_theta > self.cos_falloff {
            1.0
        } else {
            let delta = (cos_theta - self.cos_angle) / (self.cos_falloff - self.cos_angle);
            delta.powi(4)
        }
    }
}

impl LightTrait for SpotLight {
    fn kind(&self) -> LightKind {
        LightKind::DELTA_POSITION
    }

    fn le(&self, _wi: &Ray) -> Color {
        BLACK
    }

    fn sample_li<M, O>(
        &self,
        intersection: &Intersection<M, O>,
        wi: &mut Vec3,
        pdf: &mut Scalar,
    ) -> Color {
        let to_light = self.position - intersection.point;
        let distance = to_light.magnitude();
        *wi = to_light / distance;
        let cos_wi_dir = (-*wi).dot(self.direction);
        if cos_wi_dir < self.cos_angle {
            *pdf = 0.0;
            BLACK
        } else {
            *pdf = 1.0;
            self.radiance * self.falloff(cos_wi_dir) / (distance + 1.0).powi(2)
        }
    }

    fn pdf_li<M, O>(&self, _intersection: &Intersection<M, O>, _wi: Vec3) -> Scalar {
        0.0
    }
}

#[derive(Debug)]
pub struct AmbientLight {
    pub radiance: Color,
}

impl LightTrait for AmbientLight {
    fn kind(&self) -> LightKind {
        LightKind::INFINITE.set(LightKind::NO_BG)
    }

    fn le(&self, _wi: &Ray) -> Color {
        self.radiance
    }

    fn sample_li<M, O>(
        &self,
        _intersection: &Intersection<M, O>,
        wi: &mut Vec3,
        pdf: &mut Scalar,
    ) -> Color {
        *wi = random_unit_vec();
        *pdf = 1.0 / (4.0 * PI);
        self.radiance
    }

    fn pdf_li<M, O>(&self, _intersection: &Intersection<M, O>, _wi: Vec3) -> Scalar {
        1.0 / (4.0 * PI)
    }
}

#[derive(Debug)]
pub struct DirectionLight {
    pub direction: Vec3,
    pub radiance: Color,
}

impl LightTrait for DirectionLight {
    fn kind(&self) -> LightKind {
        LightKind::DELTA_DIRECTION
    }

    fn le(&self, _wi: &Ray) -> Color {
        BLACK
    }

    fn sample_li<M, O>(
        &self,
        _intersection: &Intersection<M, O>,
        wi: &mut Vec3,
        pdf: &mut Scalar,
    ) -> Color {
        *wi = -self.direction;
        *pdf = 1.0;
        self.radiance
    }

    fn pdf_li<M, O>(&self, _intersection: &Intersection<M, O>, _wi: Vec3) -> Scalar {
        0.0
    }
}

#[derive(Debug)]
pub struct AreaLight {
    pub rotation: Quaternion,
    pub position: Pt3,
    pub shape: Shape,
    pub radiance: Color,
}

impl Material for AreaLight {
    type Sampled = Color;

    fn sample(&self, _uv: Pt2) -> Self::Sampled {
        self.radiance
    }

    fn compute_scattering<'arena, O>(
        _si: &Intersection<Self::Sampled, O>,
        _arena: &'arena Bump,
        _mode: TransportMode,
        _allow_multiple_lobes: bool,
    ) -> BSDF<'arena> {
        panic!()
    }
}

impl LightTrait for AreaLight {
    fn kind(&self) -> LightKind {
        LightKind::AREA
    }

    fn le(&self, _wi: &Ray) -> Color {
        self.radiance
    }

    fn sample_li<M, O>(
        &self,
        _intersection: &Intersection<M, O>,
        _wi: &mut Vec3,
        pdf: &mut Scalar,
    ) -> Color {
        *pdf = 0.0;
        BLACK
    }

    fn pdf_li<M, O>(&self, _intersection: &Intersection<M, O>, _wi: Vec3) -> Scalar {
        0.0
    }
}

#[derive(Debug)]
pub enum Light {
    Point(PointLight),
    Spot(SpotLight),
    Direction(DirectionLight),
    Hdri(Hdri),
    Area(AreaLight),
    Ambient(AmbientLight),
}

macro_rules! indirect_light_trait {
    ($self:expr, $fn_name:ident ( $($args: expr),* ) ) => {
        match $self {
            Light::Point(light) => light.$fn_name($($args),*),
            Light::Spot(light) => light.$fn_name($($args),*),
            Light::Direction(light) => light.$fn_name($($args),*),
            Light::Hdri(light) => light.$fn_name($($args),*),
            Light::Area(light) => light.$fn_name($($args),*),
            Light::Ambient(light) => light.$fn_name($($args),*),
        }
    };
}

impl LightTrait for Light {
    fn kind(&self) -> LightKind {
        indirect_light_trait!(self, kind())
    }

    fn le(&self, wi: &Ray) -> Color {
        indirect_light_trait!(self, le(wi))
    }

    fn sample_li<M, O>(
        &self,
        intersection: &Intersection<M, O>,
        wi: &mut Vec3,
        pdf: &mut Scalar,
    ) -> Color {
        indirect_light_trait!(self, sample_li(intersection, wi, pdf))
    }

    fn pdf_li<M, O>(&self, intersection: &Intersection<M, O>, wi: Vec3) -> Scalar {
        indirect_light_trait!(self, pdf_li(intersection, wi))
    }
}

pub fn sample_one_light<M, O>(
    ray: &Ray,
    intersection: &Intersection<M, O>,
    bsdf: &BSDF,
    scene: &Scene,
) -> Color {
    let num_lights = scene.lights.iter().filter(|light| !light.is_area()).count();

    if num_lights == 0 {
        return BLACK;
    }

    let light = scene
        .lights
        .iter()
        .filter(|light| !light.is_area())
        .nth(fastrand::usize(..num_lights))
        .unwrap();
    let pdf_scale = 1.0 / scene.lights.len() as Scalar;

    estimate_direct(ray, intersection, light, bsdf, scene, false) / pdf_scale
}

pub fn estimate_direct<M, O>(
    ray: &Ray,
    intersection: &Intersection<M, O>,
    light: &Light,
    bsdf: &BSDF,
    scene: &Scene,
    specular: bool,
) -> Color {
    let mut ld = BLACK;

    let mut scattering_pdf = 0.0;

    let mut wi = Vec3::zero();
    let mut light_pdf = 0.0;
    let li = light.sample_li(intersection, &mut wi, &mut light_pdf);

    let bxdf_kind = if specular {
        BxDFKind::ALL
    } else {
        BxDFKind::ALL.unset(BxDFKind::SPECULAR)
    };

    if light_pdf > 0.0 && li != BLACK {
        // TODO: handle medium interactions

        let inter_to_light = Ray::new(intersection.point, wi, ray.time);
        if scene.intersect(&inter_to_light).is_miss() {
            let f = bsdf.f(-ray.direction, wi, bxdf_kind);
            let f = f * wi.dot(intersection.normal).abs();
            scattering_pdf = bsdf.pdf(-ray.direction, wi, bxdf_kind);

            if f != BLACK {
                if light.is_delta() {
                    ld.add_assign_element_wise(f.mul_element_wise(li) / light_pdf);
                } else {
                    let weight = power_heuristic(1.0, light_pdf, 1.0, scattering_pdf);
                    ld.add_assign_element_wise(f.mul_element_wise(li) * weight / light_pdf);

                    debugger::ray_debug! {
                        f,
                        wi,
                        -ray.direction,
                        (-ray.direction).dot(wi),
                        wi.dot(intersection.normal),
                        li,
                        ld,
                        weight,
                        light_pdf,
                        scattering_pdf
                    }
                }
            }
        }
    }

    // TODO: handle medium interactions

    if !light.is_delta() {
        let mut sampled_kind = BxDFKind::ALL;

        let f = bsdf.sample_f(
            -ray.direction,
            &mut wi,
            &mut scattering_pdf,
            &mut sampled_kind,
            bxdf_kind,
        );
        let f = f * wi.dot(intersection.normal).abs();
        let sampled_specular = sampled_kind.has(BxDFKind::SPECULAR);

        if f != BLACK && scattering_pdf > 0.0 {
            let weight = if sampled_specular {
                1.0
            } else {
                let light_pdf = light.pdf_li(intersection, wi);
                if light_pdf == 0.0 {
                    return ld;
                }
                power_heuristic(1.0, scattering_pdf, 1.0, light_pdf)
            };

            let ray = Ray::new(intersection.point, wi, ray.time);

            if scene.intersect(&ray).is_miss() {
                let li = light.le(&ray);
                if li != BLACK {
                    ld.add_assign_element_wise(f.mul_element_wise(li) * weight / scattering_pdf);

                    debugger::ray_debug! {
                        f,
                        wi,
                        -ray.direction,
                        wi.dot(intersection.normal),
                        li,
                        ld
                    }
                }
            }
        }
    }

    ld
}
