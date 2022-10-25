use crate::intersect::Intersection;
use crate::scene::{SampledMaterial, Scene};
use crate::types::color::BLACK;
use crate::types::scalar::consts::PI;
use crate::types::{color, Vec3};
use crate::util::random_unit_vec;
use crate::{debugger, scalar, Color, Ray};
use cgmath::{Array, ElementWise, EuclideanSpace, InnerSpace};

fn reflect(vec: Vec3, reflector: Vec3) -> Vec3 {
    -vec + 2.0 * reflector * vec.dot(reflector)
}

pub fn diffuse_brdf(intersection: &Intersection, ray_in: &Ray, ray_out: &Ray) -> Color {
    // Material
    let SampledMaterial {
        base_color,
        roughness,
        ..
    } = intersection.sampled_material;

    // Common values
    let half_vector = (-ray_in.direction + ray_out.direction).normalize();
    let cos_theta_d = ray_out.direction.dot(half_vector);
    let cos_theta_l = ray_out.direction.dot(intersection.normal);
    let cos_theta_v = -ray_in.direction.dot(intersection.normal);

    let cos2_theta_d = cos_theta_d * cos_theta_d;

    // Diffuse
    let f_d90 = 0.5 + 2.0 * roughness * cos2_theta_d;
    (base_color / PI)
        * (1.0 + (f_d90 - 1.0) * (1.0 - cos_theta_l).powi(5))
        * (1.0 + (f_d90 - 1.0) * (1.0 - cos_theta_v).powi(5))
}

pub fn specular_brdf(intersection: &Intersection, ray_in: &Ray, ray_out: &Ray) -> Color {
    // Material
    let SampledMaterial {
        base_color,
        roughness,
        metallic,
        specular,
        ..
    } = intersection.sampled_material;

    // Common values
    let half_vector = -ray_in.direction + ray_out.direction;
    let half_vector = if half_vector.magnitude2() <= 0.00001 {
        ray_out.direction
    } else {
        half_vector.normalize()
    };
    let cos_theta_d = ray_out.direction.dot(half_vector);
    let cos_theta_l = ray_out.direction.dot(intersection.normal);
    let cos_theta_v = -ray_in.direction.dot(intersection.normal);
    let cos_theta_h = intersection.normal.dot(half_vector);

    let cos2_theta_h = cos_theta_h * cos_theta_h;

    let alpha_g = (0.5 + roughness / 2.0).powi(2);
    let alpha = roughness * roughness;
    let alpha2 = alpha * alpha;

    // Microfacet specular
    let f_0 = color::mix(color(specular, specular, specular), base_color, metallic).to_vec();

    let k = alpha_g;

    let d_tr = alpha2.max(0.1)
        / (PI * ((alpha2.max(0.1) - 1.0) * cos2_theta_h + 1.0).powi(2)).max(0.00001);
    let f_schlick = f_0 + (Vec3::from_value(1.0) - f_0) * (1.0 - cos_theta_d).powi(5);
    let g_schlick_ggx = (cos_theta_v / (cos_theta_v * (1.0 - k) + k))
        * (cos_theta_l / (cos_theta_l * (1.0 - k) + k));

    let dfg = d_tr * f_schlick * g_schlick_ggx;
    let mut denominator = 4.0 * cos_theta_l * cos_theta_v;
    if denominator == 0.0 {
        denominator = 0.0001;
    }

    debugger::ray_debug! {
        ray_in.direction,
        ray_out.direction,
        half_vector,
        cos_theta_d,
        d_tr,
        f_0,
        f_schlick,
        g_schlick_ggx,
        denominator
    }

    Color::from_vec(dfg / denominator)
}

pub fn ray_color(ray_in: &Ray, scene: &Scene, depth: usize) -> Color {
    debugger::begin_ray!();
    let color = if depth >= scene.camera.bounce_limit {
        BLACK
    } else if let Some(intersection) = scene.intersect(ray_in) {
        let SampledMaterial {
            base_color,
            metallic,
            specular_tint,
            roughness,
            ..
        } = intersection.sampled_material;

        if intersection.normal.dot(-ray_in.direction) < 0.0 {
            return BLACK;
        }

        let ray_out = if scene.camera.hdri_bias.is_some()
            && scalar::rand() < 0.1
            && scene.camera.hdri_bias.unwrap().dot(intersection.normal) >= 0.0
        {
            Ray::new(intersection.point, scene.camera.hdri_bias.unwrap())
        } else {
            Ray::new(intersection.point, intersection.normal + random_unit_vec())
        };

        let ray_out_specular = Ray::new(
            intersection.point,
            reflect(-ray_in.direction, intersection.normal) + roughness * random_unit_vec(),
        );

        let specular_radiance_in = ray_color(&ray_out_specular, scene, depth + 1);
        let specular = specular_brdf(&intersection, ray_in, &ray_out_specular);

        let diffuse_radiance_in = ray_color(&ray_out, scene, depth + 1);
        let diffuse = diffuse_brdf(&intersection, ray_in, &ray_out);

        debugger::ray_debug! { diffuse, specular }

        let k_d = 1.0 - metallic;
        let k_s = 1.0;

        let diffuse_radiance = diffuse.mul_element_wise(diffuse_radiance_in)
            * intersection.normal.dot(ray_out.direction)
            * k_d;

        let specular_radiance = specular
            .mul_element_wise(specular_radiance_in)
            .mul_element_wise(color::mix(color(1.0, 1.0, 1.0), base_color, specular_tint))
            * intersection.normal.dot(ray_out_specular.direction)
            * k_s;

        diffuse_radiance.add_element_wise(specular_radiance)
    } else {
        debugger::ray_print!("Sky");
        scene
            .camera
            .hdri
            .in_direction(ray_in.direction)
            .map(|v| (v + 1.0).log2())
    };
    debugger::end_ray!(color)
}
