use crate::bxdf::BxDFKind;
use crate::debugger;
use crate::intersect::PossibleIntersection;
use crate::light::hdri::Hdri;
use crate::light::{estimate_direct, sample_one_light, LightKind, LightTrait};
use crate::material::{EmptyMaterial, Material, TransportMode};
use crate::scene::{DisneyMaterial, Scene, Shape};
use crate::types::color::{BLACK, WHITE};
use crate::types::{color, scalar, Scalar, Vec3};
use crate::types::{Color, Ray};
use crate::util::max_value3;
use bumpalo::Bump;
use cgmath::{vec3, ElementWise, EuclideanSpace, InnerSpace, MetricSpace, Zero};

pub fn ray_color<'arena>(ray: &Ray, scene: &Scene, arena: &'arena Bump) -> Color {
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
                    let ld =
                        beta.mul_element_wise(sample_one_light(&ray, &intersection, &bsdf, scene));
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
                    radiance,
                    intersection.normal
                }

                if bounce_count > 3 && (1.0 - max_value3(beta).max(0.7)) < scalar::rand() {
                    debugger::ray_print!("Russian Roulette Miss");
                    break;
                }

                ray = Ray::new(intersection.point, wi, ray.time);
            }
            PossibleIntersection::HitLight(intersection) => {
                let area = intersection.sampled_material;
                radiance.add_assign_element_wise(area.le(&ray).mul_element_wise(beta));
                break;
            }
            PossibleIntersection::Ignored => {
                debugger::ray_print!("Ignored");
                break;
            }
            PossibleIntersection::Miss => {
                if bounce_count == 0 || specular_bounce {
                    debugger::ray_print!("Sky Specular");
                    for light in &scene.lights {
                        if !light.kind().has(LightKind::AREA) && !light.kind().has(LightKind::NO_BG)
                        {
                            let light = light.le(&ray);
                            radiance.add_assign_element_wise(light.mul_element_wise(beta));
                        }
                    }
                } else {
                    debugger::ray_print!("Sky Ignored");
                }
                break;
            }
        }
    }

    radiance
}
