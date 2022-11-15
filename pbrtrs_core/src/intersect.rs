use crate::light::{AreaLight, Light};
use crate::material::{EmptyMaterial, Material};
use crate::scene::{Object, SampledDisneyMaterial, Scene, Shape};
use crate::types::scalar::consts::PI;
use crate::types::{Pt2, Pt3, Quaternion, Ray, Scalar, Vec3};
use cgmath::{point2, point3, vec3, EuclideanSpace, InnerSpace, Rotation};

pub struct Intersection<'a, M, O> {
    pub distance: Scalar,
    pub normal: Vec3,
    pub tangent: Vec3,
    pub point: Pt3,
    pub sampled_material: M,
    pub object: &'a O,
    pub uv: Pt2,
}

impl Intersection<'static, (), ()> {
    pub const fn dummy() -> Self {
        Self {
            distance: 0.0,
            normal: vec3(0.0, 0.0, 0.0),
            tangent: vec3(0.0, 0.0, 0.0),
            point: point3(0.0, 0.0, 0.0),
            sampled_material: (),
            object: &(),
            uv: point2(0.0, 0.0),
        }
    }
}

impl<'a, M, O> Intersection<'a, M, O> {
    pub fn map_material<T, F>(self, f: F) -> Intersection<'a, T, O>
    where
        F: FnOnce(M) -> T,
    {
        let Intersection {
            distance,
            normal,
            tangent,
            point,
            sampled_material,
            uv,
            object,
        } = self;
        Intersection {
            distance,
            normal,
            tangent,
            point,
            uv,
            sampled_material: f(sampled_material),
            object,
        }
    }
}

pub enum PossibleIntersection<'a, M, O> {
    Hit(Intersection<'a, M, O>),
    HitLight(Intersection<'a, (), AreaLight>),
    Miss,
    Ignored,
}

impl<'a, M, O> PossibleIntersection<'a, M, O> {
    pub fn is_miss(&self) -> bool {
        matches!(self, PossibleIntersection::Miss)
    }

    pub fn is_hit(&self) -> bool {
        matches!(self, PossibleIntersection::Hit(_))
    }

    pub fn is_ignored(&self) -> bool {
        matches!(self, PossibleIntersection::Ignored)
    }

    pub fn unwrap_distance(&self) -> Scalar {
        match self {
            PossibleIntersection::HitLight(i) => i.distance,
            PossibleIntersection::Hit(i) => i.distance,
            _ => panic!("unwrap called on a miss or ignored intersection"),
        }
    }

    pub fn unwrap(&self) -> &Intersection<'a, M, O> {
        match self {
            PossibleIntersection::Hit(i) => i,
            _ => panic!("unwrap called on a miss or ignored intersection"),
        }
    }

    pub fn unwrap_into(self) -> Intersection<'a, M, O> {
        match self {
            PossibleIntersection::Hit(i) => i,
            _ => panic!("unwrap called on a miss or ignored intersection"),
        }
    }
}

impl Shape {
    pub fn intersect<'mat, M: Material, O>(
        &self,
        ray: &Ray,
        rotate: Quaternion,
        translate: Vec3,
        material: &'mat M,
        object: &'mat O,
    ) -> PossibleIntersection<'mat, M::Sampled, O> {
        const T_MIN: Scalar = 0.001;
        match self {
            Self::Sphere { radius } => {
                let sphere_center: Pt3 = Pt3::from_vec(translate);
                let oc = ray.origin - sphere_center;

                let a = ray.direction.magnitude2(); // can simplify to 1
                let h = oc.dot(ray.direction);
                let c = oc.magnitude2() - radius * radius;
                let discriminant = h * h - a * c;
                if discriminant < 0.0 {
                    PossibleIntersection::Miss
                } else {
                    let t = (-h - discriminant.sqrt()) / a;
                    if t < 0.0 {
                        PossibleIntersection::Miss
                    } else if t < T_MIN {
                        PossibleIntersection::Ignored
                    } else {
                        let point = ray.at(t);

                        let normal = (point - sphere_center).normalize();

                        let tangent = if normal.z.abs() <= 1e-6 && normal.x.abs() <= 1e-6 {
                            vec3(1.0, 0.0, 0.0)
                        } else {
                            vec3(normal.z, 0.0, -normal.x).normalize()
                        };

                        // Compute UV
                        let rnormal = rotate.rotate_vector(normal);

                        let theta = rnormal.angle(vec3(0.0, 1.0, 0.0)).0;
                        let phi = rnormal.x.atan2(rnormal.z);

                        let uv = point2(theta / PI, (phi + PI) / (2.0 * PI));

                        PossibleIntersection::Hit(Intersection {
                            distance: t,
                            point,
                            normal,
                            tangent,
                            sampled_material: material.sample(uv),
                            uv,
                            object,
                        })
                    }
                }
            }
        }
    }
}

impl Scene {
    pub fn intersect(&self, ray: &Ray) -> PossibleIntersection<SampledDisneyMaterial, Object> {
        let mut nearest = PossibleIntersection::Miss;
        for object in &self.objects {
            match object.shape.intersect(
                ray,
                object.rotation,
                object.position.to_vec() + object.motion * ray.time,
                &object.material,
                object,
            ) {
                PossibleIntersection::Hit(intersection) => {
                    if nearest.is_miss() || intersection.distance < nearest.unwrap_distance() {
                        nearest = PossibleIntersection::Hit(intersection);
                    }
                }
                PossibleIntersection::Ignored => {
                    return PossibleIntersection::Ignored;
                }
                PossibleIntersection::Miss => {}
                PossibleIntersection::HitLight(_) => unreachable!(),
            }
        }
        for light in &self.lights {
            if let Light::Area(area) = light {
                match area.shape.intersect(
                    ray,
                    area.rotation,
                    area.position.to_vec(),
                    &EmptyMaterial,
                    area,
                ) {
                    PossibleIntersection::Hit(intersection) => {
                        if nearest.is_miss() || intersection.distance < nearest.unwrap_distance() {
                            nearest = PossibleIntersection::HitLight(intersection);
                        }
                    }
                    PossibleIntersection::Ignored => {
                        return PossibleIntersection::Ignored;
                    }
                    PossibleIntersection::Miss => {}
                    PossibleIntersection::HitLight(_) => unreachable!(),
                }
            }
        }
        nearest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::Zero;

    #[test]
    fn sphere_intersect() {
        let shape = Shape::Sphere { radius: 1.0 };
        // Sphere at (0, 2, 0), camera at origin, looking in +y
        let Intersection {
            normal,
            tangent,
            point,
            distance,
            ..
        } = shape
            .intersect(
                &Ray::new(Pt3::origin(), vec3(0.0, 1.0, 0.0), 0.0),
                Quaternion::zero(),
                vec3(0.0, 2.0, 0.0),
                &EmptyMaterial,
                &(),
            )
            .unwrap_into();
        assert_eq!(point, point3(0.0, 1.0, 0.0));
        assert_eq!(normal, vec3(0.0, -1.0, 0.0));
        assert!(normal.is_perpendicular(tangent));
        assert_eq!(distance, 1.0);

        let shape = Shape::Sphere { radius: 2.0 };
        // Sphere at (0, 2, 0), radius 2, camera at origin, looking in +y
        let Intersection {
            normal,
            point,
            tangent,
            distance,
            ..
        } = shape
            .intersect(
                &Ray::new(Pt3::origin(), vec3(0.0, 1.0, 0.0), 0.0),
                Quaternion::zero(),
                vec3(0.0, 4.0, 0.0),
                &EmptyMaterial,
                &(),
            )
            .unwrap_into();
        assert_eq!(point, point3(0.0, 2.0, 0.0));
        assert_eq!(normal, vec3(0.0, -1.0, 0.0));
        assert!(normal.is_perpendicular(tangent));
        assert_eq!(distance, 2.0);

        let shape = Shape::Sphere { radius: 100.0 };
        // Sphere at (0, -100, 0), radius 100, camera at 3.0, 1.5, 3.0, looking at 0.0, 0.0, 0.0
        assert!(shape
            .intersect(
                &Ray::new(
                    point3(3.0, 1.5, 3.0),
                    vec3(-0.11515933, 0.35110158, -0.9292287),
                    0.0
                ),
                Quaternion::zero(),
                vec3(0.0, -100.0, 0.0),
                &EmptyMaterial,
                &(),
            )
            .is_miss());

        let shape = Shape::Sphere { radius: 1.0 };
        assert!(shape
            .intersect(
                &Ray::new(point3(0.0, 1.0, 0.0), vec3(0.0, -1.0, 0.0), 0.0),
                Quaternion::zero(),
                vec3(0.0, 0.0, 0.0),
                &EmptyMaterial,
                &(),
            )
            .is_ignored());
    }
}
