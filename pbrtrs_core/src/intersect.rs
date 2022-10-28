use crate::material::Material;
use crate::scene::{DisneyMaterial, SampledDisneyMaterial, Scene, Shape};
use crate::types::scalar::consts::PI;
use crate::types::{Mat4, Pt2, Pt3, Ray, Scalar, Vec3};
use cgmath::{
    assert_abs_diff_eq, point2, vec3, EuclideanSpace, InnerSpace, Rad, Transform, Transform3,
};

pub struct Intersection<M> {
    pub distance: Scalar,
    pub normal: Vec3,
    pub tangent: Vec3,
    pub point: Pt3,
    pub sampled_material: M,
    pub uv: Pt2,
}

impl Shape {
    pub fn intersect<'mat, M: Material, T: Transform3<Scalar = Scalar>>(
        &self,
        ray: &Ray,
        shape_transform: T,
        material: &'mat M,
    ) -> Option<Intersection<M::Sampled>> {
        match self {
            Self::Sphere(radius) => {
                let sphere_center: Pt3 = shape_transform.transform_point(Pt3::origin());
                let oc = ray.origin - sphere_center;

                let a = ray.direction.magnitude2(); // can simplify to 1
                let h = oc.dot(ray.direction);
                let c = oc.magnitude2() - radius * radius;
                let discriminant = h * h - a * c;
                if discriminant < 0.0 {
                    None
                } else {
                    let t = (-h - discriminant.sqrt()) / a;
                    if t < 0.001 {
                        None
                    } else {
                        let point = ray.at(t);

                        let normal = (point - sphere_center).normalize();

                        let theta = normal.angle(vec3(0.0, 1.0, 0.0)).0;
                        let phi = normal.x.atan2(normal.z);

                        let tangent = if normal.z.abs() <= 1e-6 && normal.x.abs() <= 1e-6 {
                            vec3(1.0, 0.0, 0.0)
                        } else {
                            vec3(normal.z, 0.0, -normal.x).normalize()
                        };

                        let uv = point2(theta / PI, (phi + PI) / (2.0 * PI));

                        Some(Intersection {
                            distance: t,
                            point,
                            normal,
                            tangent,
                            sampled_material: material.sample(uv),
                            uv,
                        })
                    }
                }
            }
        }
    }
}

impl Scene {
    pub fn intersect(&self, ray: &Ray) -> Option<Intersection<SampledDisneyMaterial>> {
        let mut nearest: Option<Intersection<SampledDisneyMaterial>> = None;
        for object in &self.objects {
            if let Some(intersect) = object.shape.intersect(
                ray,
                Mat4::from_translation(object.position.to_vec()),
                &object.material,
            ) {
                if nearest.is_none() || nearest.as_ref().unwrap().distance > intersect.distance {
                    nearest = Some(intersect);
                }
            }
        }
        nearest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Mat4;
    use cgmath::{point3, vec3};

    #[test]
    fn sphere_intersect() {
        let material = DisneyMaterial::default();
        let shape = Shape::Sphere(1.0);
        // Sphere at (0, 2, 0), camera at origin, looking in +y
        let Intersection {
            normal,
            tangent,
            point,
            distance,
            ..
        } = shape
            .intersect(
                &Ray::new(Pt3::origin(), vec3(0.0, 1.0, 0.0)),
                Mat4::from_translation(vec3(0.0, 2.0, 0.0)),
                &material,
            )
            .unwrap();
        assert_eq!(point, point3(0.0, 1.0, 0.0));
        assert_eq!(normal, vec3(0.0, -1.0, 0.0));
        assert!(normal.is_perpendicular(tangent));
        assert_eq!(distance, 1.0);

        let shape = Shape::Sphere(2.0);
        // Sphere at (0, 2, 0), radius 2, camera at origin, looking in +y
        let Intersection {
            normal,
            point,
            tangent,
            distance,
            ..
        } = shape
            .intersect(
                &Ray::new(Pt3::origin(), vec3(0.0, 1.0, 0.0)),
                Mat4::from_translation(vec3(0.0, 4.0, 0.0)),
                &material,
            )
            .unwrap();
        assert_eq!(point, point3(0.0, 2.0, 0.0));
        assert_eq!(normal, vec3(0.0, -1.0, 0.0));
        assert!(normal.is_perpendicular(tangent));
        assert_eq!(distance, 2.0);

        let shape = Shape::Sphere(100.0);
        // Sphere at (0, -100, 0), radius 100, camera at 3.0, 1.5, 3.0, looking at 0.0, 0.0, 0.0
        assert!(shape
            .intersect(
                &Ray::new(
                    point3(3.0, 1.5, 3.0),
                    vec3(-0.11515933, 0.35110158, -0.9292287)
                ),
                Mat4::from_translation(vec3(0.0, -100.0, 0.0)),
                &material,
            )
            .is_none());
    }
}
