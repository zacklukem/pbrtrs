use crate::bxdf::{BxDFKind, BSDF};
use crate::intersect::Intersection;
use crate::scene::Scene;
use crate::types::color::BLACK;
use crate::types::scalar::consts::PI;
use crate::types::{color, scalar, Color, Pt2, Ray, Scalar, Vec3};
use cgmath::{point2, vec3, ElementWise, InnerSpace, Zero};
use image::{Pixel, Pixels, Rgb32FImage};
use std::fmt::{Debug, Formatter};

fn binary_search_cdf(cdf: &[Scalar], value: Scalar) -> usize {
    let mut low = 0;
    let mut high = cdf.len() - 1;
    while low < high {
        let mid = (low + high) / 2;
        if cdf[mid] <= value {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    (low as isize - 1).clamp(0, cdf.len() as isize - 1) as usize
}

pub struct Distribution1D {
    cdf: Vec<Scalar>,
    func: Vec<Scalar>,
    integral: Scalar,
}

impl Distribution1D {
    pub fn new(func: Vec<Scalar>) -> Self {
        let n = func.len();
        let mut cdf = vec![0.0; n + 1];

        for i in 1..(n + 1) {
            cdf[i] = cdf[i - 1] + func[i - 1] / n as Scalar;
        }

        let integral = cdf[n];
        if integral == 0.0 {
            for i in 1..(n + 1) {
                cdf[i] = i as Scalar / n as Scalar;
            }
        } else {
            for i in 1..(n + 1) {
                cdf[i] /= integral;
            }
        }

        Self {
            cdf,
            integral,
            func,
        }
    }

    pub fn count(&self) -> usize {
        self.func.len()
    }

    pub fn sample_continuous(&self, u: Scalar, pdf: &mut Scalar) -> (usize, Scalar) {
        let offset = binary_search_cdf(&self.cdf, u);
        let mut du = u - self.cdf[offset];

        if (self.cdf[offset + 1] - self.cdf[offset]) > 0.0 {
            du /= self.cdf[offset + 1] - self.cdf[offset];
        }

        *pdf = self.func[offset] / self.integral;

        (offset, (offset as Scalar + du) / self.cdf.len() as Scalar)
    }

    pub fn sample_discrete(&self, u: Scalar) -> (usize, Scalar) {
        let offset = binary_search_cdf(&self.cdf, u);
        let u_prime = (u - self.cdf[offset]) / (self.cdf[offset + 1] - self.cdf[offset]);
        (offset, u_prime)
    }
}

pub struct Distribution2D {
    p_conditional_v: Vec<Distribution1D>,
    p_marginal: Distribution1D,
}

impl Distribution2D {
    pub fn new(f: impl ExactSizeIterator<Item = Vec<Scalar>>) -> Self {
        let p_conditional_v = f
            .map(|f| Distribution1D::new(f))
            .collect::<Vec<Distribution1D>>();

        let p_integral = p_conditional_v
            .iter()
            .map(|p| p.integral)
            .collect::<Vec<_>>();

        let p_marginal = Distribution1D::new(p_integral.iter().copied().collect::<Vec<_>>());

        Self {
            p_conditional_v,
            p_marginal,
        }
    }

    pub fn pdf(&self, u: Pt2) -> Scalar {
        let iu = ((u[0] * self.p_conditional_v[0].count() as Scalar) as usize)
            .clamp(0, self.p_conditional_v[0].count() - 1);
        let iv = ((u[1] * self.p_marginal.count() as Scalar) as usize)
            .clamp(0, self.p_marginal.count() - 1);
        self.p_conditional_v[iv].func[iu] / self.p_marginal.integral
    }

    pub fn sample_continuous(&self, u: Pt2, pdf: &mut Scalar) -> Pt2 {
        let (mut pdf_0, mut pdf_1) = (0.0, 0.0);
        let (v, d1) = self.p_marginal.sample_continuous(u[1], &mut pdf_1);
        let (_, d0) = self.p_conditional_v[v].sample_continuous(u[0], &mut pdf_0);

        *pdf = pdf_0 * pdf_1;

        point2(d0, d1)
    }
}

pub struct Hdri {
    pub image: Rgb32FImage,
    pub distribution: Distribution2D,
}

impl Hdri {
    pub fn new(image: Rgb32FImage) -> Self {
        let distribution = Distribution2D::new(
            image
                .rows()
                .map(|i| i.map(|p| p.0.iter().sum::<Scalar>()).collect::<Vec<_>>()),
        );

        Self {
            image,
            distribution,
        }
    }

    pub fn lookup(&self, uv: Pt2) -> Color {
        let x = ((self.image.width() as Scalar * uv.x) as u32).min(self.image.width() - 1);
        let y = ((self.image.height() as Scalar * uv.y) as u32).min(self.image.height() - 1);
        let [r, g, b] = self.image.get_pixel(x, y).0;
        color(r, g, b)
    }

    pub fn in_direction(&self, direction: Vec3) -> Color {
        let u = (direction.x.atan2(direction.z) + PI) / (2.0 * PI);
        let v = direction.angle(vec3(0.0, 1.0, 0.0)).0 / PI;

        self.lookup(point2(u, v))
    }

    pub fn sample_li<M>(
        &self,
        _intersection: &Intersection<M>,
        wi: &mut Vec3,
        pdf: &mut Scalar,
    ) -> Color {
        let u = point2(scalar::rand(), scalar::rand());

        let mut map_pdf = 0.0;
        let uv = self.distribution.sample_continuous(u, &mut map_pdf);

        if map_pdf == 0.0 {
            return BLACK;
        }

        let theta = u.x * PI;
        let phi = u.y * 2.0 * PI;
        let cos_theta = theta.cos();
        let sin_theta = theta.sin();
        let cos_phi = phi.cos();
        let sin_phi = phi.sin();
        *wi = vec3(sin_theta * cos_phi, cos_theta, sin_theta * sin_phi);

        *pdf = if sin_theta == 0.0 {
            0.0
        } else {
            map_pdf / (2.0 * PI * PI * sin_theta)
        };

        self.lookup(uv)
    }

    pub fn pdf_li<M>(&self, _intersection: &Intersection<M>, wi: Vec3) -> Scalar {
        let theta = wi.angle(vec3(0.0, 1.0, 0.0)).0;
        let phi = wi.x.atan2(wi.z) + PI;
        let sin_theta = theta.sin();
        if sin_theta == 0.0 {
            0.0
        } else {
            self.distribution.pdf(point2(phi / PI, theta / PI)) / (2.0 * PI * PI * sin_theta)
        }
    }
}

impl Debug for Hdri {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[hdri]")
    }
}

pub fn power_heuristic(nf: Scalar, f_pdf: Scalar, ng: Scalar, g_pdf: Scalar) -> Scalar {
    let f = nf * f_pdf;
    let g = ng * g_pdf;
    (f * f) / (f * f + g * g)
}

pub fn estimate_direct<M>(
    ray: &Ray,
    intersection: &Intersection<M>,
    bsdf: &BSDF,
    scene: &Scene,
    specular: bool,
) -> Color {
    let light = &scene.camera.hdri;

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

        let ray = Ray::new(intersection.point, wi);
        if scene.intersect(&ray).is_miss() {
            let f = bsdf.f(-ray.direction, wi, bxdf_kind);
            let f = f * wi.dot(intersection.normal).abs();
            scattering_pdf = bsdf.pdf(-ray.direction, wi, bxdf_kind);

            if f != BLACK {
                // TODO: Check light visibility
                // TODO: Handle delta lights
                let weight = power_heuristic(1.0, light_pdf, 1.0, scattering_pdf);
                ld.add_assign_element_wise(f.mul_element_wise(li) * weight / light_pdf);
            }
        }
    }

    // TODO: handle delta lights
    // TODO: handle medium interactions

    let mut sampled_kind = BxDFKind::ALL;

    let f = bsdf.sample_f(
        -ray.direction,
        &mut wi,
        &mut scattering_pdf,
        &mut sampled_kind,
        bxdf_kind,
    ) * wi.dot(intersection.normal).abs();
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

        let ray = Ray::new(intersection.point, wi);

        if scene.intersect(&ray).is_miss() {
            let li = light.in_direction(wi);
            if li != BLACK {
                ld.add_assign_element_wise(f.mul_element_wise(li) * weight / scattering_pdf);
            }
        }
    }

    ld
}
