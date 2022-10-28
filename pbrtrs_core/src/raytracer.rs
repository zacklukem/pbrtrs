use crate::bxdf::BxDFKind;
use crate::debugger;
use crate::material::{Material, TransportMode};
use crate::scene::{DisneyMaterial, Scene};
use crate::types::color::{BLACK, WHITE};
use crate::types::{color, Mat3, Mat4, Pt2, Pt3, Vec3};
use crate::types::{scalar, Color, Ray, Scalar};
use bumpalo::Bump;
use cgmath::{ElementWise, EuclideanSpace, InnerSpace, MetricSpace, Zero};

pub fn ray_color<'arena>(ray: &Ray, scene: &Scene, arena: &'arena Bump) -> Color {
    let mut radiance = BLACK;
    let mut beta = WHITE;
    let mut ray = *ray;
    #[cfg(feature = "enable_debugger")]
    let mut num_begin = 0;
    for _ in 0..scene.camera.bounce_limit {
        #[cfg(feature = "enable_debugger")]
        {
            num_begin += 1;
        }
        debugger::begin_ray!();
        if let Some(intersection) = scene.intersect(&ray) {
            let bsdf = DisneyMaterial::compute_scattering(
                &intersection,
                arena,
                TransportMode::Importance,
                true,
            );

            let mut wi = Vec3::zero();
            let mut pdf = 0.0;
            let mut sampled_kind = BxDFKind::ALL;
            let f = bsdf.sample_f(
                -ray.direction,
                &mut wi,
                &mut pdf,
                &mut sampled_kind,
                BxDFKind::ALL,
            );

            if f.distance2(Color::origin()) == 0.0 || pdf == 0.0 {
                debugger::ray_print!("PDF 0 Miss ");
                debugger::ray_debug! {
                    f,
                    pdf
                }
                break;
            }

            beta.mul_assign_element_wise(f * wi.dot(intersection.normal).abs() / pdf);

            debugger::ray_debug! {
                wi,
                f,
                pdf,
                sampled_kind,
                -ray.direction,
                beta
            }

            ray = Ray::new(intersection.point, wi);
        } else {
            let light = scene
                .camera
                .hdri
                .in_direction(ray.direction)
                .map(|v| (v + 1.0).log2());
            radiance.add_assign_element_wise(light.mul_element_wise(beta));
            break;
        }
    }

    #[cfg(feature = "enable_debugger")]
    for _ in 0..num_begin {
        debugger::end_ray!(BLACK);
    }

    radiance
}
