use crate::scene::{Material, Scene, Shape};
use crate::types::{Mat4, Pt2, Pt3, Ray, Scalar, Vec3};
use cgmath::{point2, EuclideanSpace, InnerSpace, Transform3};

pub struct Intersection<'mat> {
    pub distance: Scalar,
    pub normal: Vec3,
    pub point: Pt3,
    pub material: &'mat Material,
    pub uv: Pt2,
}

impl Shape {
    pub fn intersect<'mat, T: Transform3<Scalar = Scalar>>(
        &self,
        ray: &Ray,
        shape_transform: T,
        material: &'mat Material,
    ) -> Option<Intersection<'mat>> {
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
                        Some(Intersection {
                            distance: t,
                            point,
                            normal: (point - sphere_center).normalize(),
                            material,
                            uv: point2(0.0, 0.0), // TODO
                        })
                    }
                }
            }
        }
    }
}

impl Scene {
    pub fn intersect(&self, ray: &Ray) -> Option<Intersection> {
        let mut nearest: Option<Intersection> = None;
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
        let material = Material::default();
        let shape = Shape::Sphere(1.0);
        // Sphere at (0, 2, 0), camera at origin, looking in +y
        let Intersection {
            normal,
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
        assert_eq!(distance, 1.0);

        let shape = Shape::Sphere(2.0);
        // Sphere at (0, 2, 0), radius 2, camera at origin, looking in +y
        let Intersection {
            normal,
            point,
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
