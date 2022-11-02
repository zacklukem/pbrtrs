use crate::bxdf::BxDFKind;
use crate::debugger;
use crate::intersect::PossibleIntersection;
use crate::light::{estimate_direct, Light};
use crate::material::{EmptyMaterial, Material, TransportMode};
use crate::scene::{DisneyMaterial, Scene, Shape};
use crate::types::color::{BLACK, WHITE};
use crate::types::{color, scalar, Scalar, Vec3};
use crate::types::{Color, Ray};
use crate::util::max_value3;
use bumpalo::Bump;
use cgmath::{vec3, ElementWise, EuclideanSpace, InnerSpace, MetricSpace, Zero};

const S: Shape = Shape::Sphere(1.0);

pub fn ray_color<'arena>(ray: &Ray, scene: &Scene, arena: &'arena Bump) -> Color {
    // if let PossibleIntersection::Hit(it) = S.intersect(ray, vec3(0.0, 1.0, 0.0), &EmptyMaterial) {
    //     let pdf = scene.camera.hdri.pdf_li(&it, it.normal);
    //     let val = scene.camera.hdri.in_direction(it.normal);
    //     let val = 0.299 * val.x + 0.587 * val.y + 0.114 * val.z;
    //     return color(pdf * 10.0, val, 0.0);
    // }

    let mut radiance = BLACK;
    let mut beta = WHITE;
    let mut ray = *ray;
    let mut specular_bounce = false;
    for bounce_count in 0..scene.camera.bounce_limit {
        debugger::begin_ray!();
        match scene.intersect(&ray) {
            PossibleIntersection::Hit(intersection) => {
                let bsdf = DisneyMaterial::compute_scattering(
                    &intersection,
                    arena,
                    TransportMode::Importance,
                    true,
                );

                if bsdf.num_components(BxDFKind::ALL.unset(BxDFKind::SPECULAR)) > 0 {
                    let ld = beta.mul_element_wise(estimate_direct(
                        &ray,
                        &intersection,
                        &bsdf,
                        scene,
                        false,
                    ));
                    radiance.add_assign_element_wise(ld);
                }

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
                specular_bounce = sampled_kind.has(BxDFKind::SPECULAR);

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
                    beta,
                    radiance
                }

                if bounce_count > 3 && (1.0 - max_value3(beta).max(0.7)) < scalar::rand() {
                    debugger::ray_print!("Russian Roulette Miss");
                    break;
                }

                ray = Ray::new(intersection.point, wi);
            }
            PossibleIntersection::Ignored => {
                debugger::ray_print!("Ignored");
                break;
            }
            PossibleIntersection::Miss => {
                if bounce_count == 0 || specular_bounce {
                    debugger::ray_print!("Sky Specular");
                    // let light = scene.camera.hdri.le(&ray);
                    // radiance.add_assign_element_wise(light.mul_element_wise(beta));
                } else {
                    debugger::ray_print!("Sky Ignored");
                }
                break;
            }
        }
    }

    radiance
}
