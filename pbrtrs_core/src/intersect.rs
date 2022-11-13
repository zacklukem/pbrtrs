use crate::light::{AreaLight, Light};
use crate::material::{EmptyMaterial, Material};
use crate::scene::{SampledDisneyMaterial, Scene, Shape};
use crate::types::scalar::consts::PI;
use crate::types::{Mat4, Pt2, Pt3, Quaternion, Ray, Scalar, Vec3};
use cgmath::{
    point2, point3, vec3, EuclideanSpace, InnerSpace, MetricSpace, Rotation, Transform, Zero,
};

pub struct Intersection<M> {
    pub distance: Scalar,
    pub normal: Vec3,
    pub tangent: Vec3,
    pub point: Pt3,
    pub sampled_material: M,
    pub uv: Pt2,
}

impl Intersection<()> {
    pub const fn dummy() -> Self {
        Self {
            distance: 0.0,
            normal: vec3(0.0, 0.0, 0.0),
            tangent: vec3(0.0, 0.0, 0.0),
            point: point3(0.0, 0.0, 0.0),
            sampled_material: (),
            uv: point2(0.0, 0.0),
        }
    }
}
impl<M> Intersection<M> {
    pub fn map<T, F>(self, f: F) -> Intersection<T>
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
        } = self;
        Intersection {
            distance,
            normal,
            tangent,
            point,
            uv,
            sampled_material: f(sampled_material),
        }
    }
}

pub enum PossibleIntersection<'a, M> {
    Hit(Intersection<M>),
    HitLight(Intersection<&'a AreaLight>),
    Miss,
    Ignored,
}

impl<'a, M> PossibleIntersection<'a, M> {
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

    pub fn unwrap(&self) -> &Intersection<M> {
        match self {
            PossibleIntersection::Hit(i) => i,
            _ => panic!("unwrap called on a miss or ignored intersection"),
        }
    }

    pub fn unwrap_into(self) -> Intersection<M> {
        match self {
            PossibleIntersection::Hit(i) => i,
            _ => panic!("unwrap called on a miss or ignored intersection"),
        }
    }
}

impl Shape {
    pub fn intersect<'mat, M: Material>(
        &self,
        ray: &Ray,
        rotate: Quaternion,
        translate: Vec3,
        material: &'mat M,
    ) -> PossibleIntersection<M::Sampled> {
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
                        })
                    }
                }
            }
            Self::Rectangle { width, height } => {
                let o = rotate.rotate_point(ray.origin - translate);
                let l = rotate.rotate_point(ray.origin + ray.direction);
                let ray = Ray::new(o, l - o, ray.time);

                let t = ray.origin.z / -ray.direction.z;

                if t < 0.0 {
                    PossibleIntersection::Miss
                } else if t < T_MIN {
                    PossibleIntersection::Ignored
                } else {
                    let x0 = -width / 2.0;
                    let x1 = width / 2.0;
                    let y0 = -height / 2.0;
                    let y1 = height / 2.0;

                    let x = ray.origin.x + t * ray.direction.x;
                    let y = ray.origin.y + t * ray.direction.y;

                    if x < x0 || x > x1 || y < y0 || y > y1 {
                        PossibleIntersection::Miss
                    } else {
                        let point = ray.at(t);

                        let normal = vec3(0.0, 1.0, 0.0);
                        let tangent = vec3(0.0, 0.0, 1.0);

                        // let normal = rotate.rotate_vector(normal);
                        // let tangent = rotate.rotate_vector(tangent);

                        let uv = point2((x - x0) / width, (y - y0) / height);

                        PossibleIntersection::Hit(Intersection {
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
    pub fn intersect(&self, ray: &Ray) -> PossibleIntersection<SampledDisneyMaterial> {
        let mut nearest = PossibleIntersection::Miss;
        for object in &self.objects {
            match object.shape.intersect(
                ray,
                object.rotation,
                object.position.to_vec() + object.motion * ray.time,
                &object.material,
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
                ) {
                    PossibleIntersection::Hit(intersection) => {
                        if nearest.is_miss() || intersection.distance < nearest.unwrap_distance() {
                            nearest = PossibleIntersection::HitLight(intersection.map(|_| area));
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

#[cfg(fixme)]
mod tests {
    use super::*;
    use crate::material::EmptyMaterial;
    use crate::types::Mat4;
    use cgmath::{point3, vec3};

    #[test]
    fn sphere_intersect() {
        let material = EmptyMaterial;
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
                &Ray::new(Pt3::origin(), vec3(0.0, 1.0, 0.0)),
                vec3(0.0, 2.0, 0.0),
                &material,
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
                &Ray::new(Pt3::origin(), vec3(0.0, 1.0, 0.0)),
                vec3(0.0, 4.0, 0.0),
                &material,
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
                    vec3(-0.11515933, 0.35110158, -0.9292287)
                ),
                vec3(0.0, -100.0, 0.0),
                &material,
            )
            .is_miss());

        let shape = Shape::Sphere { radius: 1.0 };
        assert!(shape
            .intersect(
                &Ray::new(point3(0.0, 1.0, 0.0), vec3(0.0, -1.0, 0.0)),
                vec3(0.0, 0.0, 0.0),
                &material,
            )
            .is_ignored());
    }
}
