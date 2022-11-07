use crate::bxdf::distribution::TrowbridgeReitzDistribution;
use crate::bxdf::{
    BxDF, FresnelDielectric, FresnelSchlick, Lambertian, MicrofacetReflection, MirrorSpecular,
    TransmissionSpecular, BSDF,
};
use crate::intersect::Intersection;
use crate::scene::{DisneyMaterial, SampledDisneyMaterial};
use crate::types::color::WHITE;
use crate::types::{color, Color, Pt2};
use bumpalo::Bump;
use cgmath::{point2, Array};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransportMode {
    Radiance,
    Importance,
}

pub trait Material {
    type Sampled;

    fn sample(&self, uv: Pt2) -> Self::Sampled;

    fn compute_scattering<'arena>(
        si: &Intersection<Self::Sampled>,
        arena: &'arena Bump,
        mode: TransportMode,
        allow_multiple_lobes: bool,
    ) -> BSDF<'arena>;
}

impl Material for DisneyMaterial {
    type Sampled = SampledDisneyMaterial;

    fn sample(&self, uv: Pt2) -> Self::Sampled {
        SampledDisneyMaterial {
            base_color: self.base_color.get(uv),
            subsurface: self.subsurface.get(uv),
            metallic: self.metallic.get(uv),
            specular: self.specular.get(uv),
            specular_tint: self.specular_tint.get(uv),
            roughness: self.roughness.get(uv),
            anisotropic: self.anisotropic.get(uv),
            sheen: self.sheen.get(uv),
            sheen_tint: self.sheen_tint.get(uv),
            clearcoat: self.clearcoat.get(uv),
            clearcoat_gloss: self.clearcoat_gloss.get(uv),
            transmission: self.transmission.get(uv),
            ior: self.ior.get(uv),
        }
    }

    fn compute_scattering<'arena>(
        si: &Intersection<Self::Sampled>,
        arena: &'arena Bump,
        transport_mode: TransportMode,
        allow_multiple_lobes: bool,
    ) -> BSDF<'arena> {
        let SampledDisneyMaterial {
            base_color,
            metallic,
            specular: specular_level,
            specular_tint,
            roughness,
            clearcoat,
            clearcoat_gloss,
            anisotropic,
            transmission,
            ior,
            ..
        } = si.sampled_material;
        let mut bsdf = BSDF::new(si);

        if transmission > 0.0 {
            let transmission = arena.alloc(TransmissionSpecular {
                color: base_color,
                eta_a: 1.0,
                eta_b: ior,
                fresnel: FresnelDielectric {
                    eta_i: 1.0,
                    eta_t: ior,
                },
                transport_mode,
            });
            bsdf.add(transmission);
            return bsdf;
        }

        if metallic != 1.0 {
            let lambert = arena.alloc(Lambertian(base_color).scale(1.0 - metallic));
            bsdf.add(lambert);
        }

        let alpha = roughness.powi(2);
        let aspect = (1.0 - 0.9 * anisotropic).sqrt();
        let alpha = point2(alpha / aspect, alpha * aspect);

        let fresnel = FresnelSchlick(color::mix(
            Color::from_value(specular_level),
            base_color,
            metallic,
        ));

        let distribution = TrowbridgeReitzDistribution::new(alpha);
        let specular = arena.alloc(MicrofacetReflection {
            color: color::mix(WHITE, base_color, specular_tint),
            distribution,
            fresnel,
        });
        bsdf.add(specular);

        if allow_multiple_lobes && clearcoat != 0.0 {
            // TODO: use isotropic Trowbridge-Reitz with gamma=1
            let alpha = (0.5 - clearcoat_gloss * 0.5).powi(2);
            let distribution = TrowbridgeReitzDistribution::new(Pt2::from_value(alpha));
            let clearcoat = arena.alloc(MicrofacetReflection {
                color: Color::from_value(1.0),
                distribution,
                fresnel,
            });
            bsdf.add(clearcoat);
        }
        // TODO: sheen
        bsdf
    }
}

pub struct EmptyMaterial;

impl Material for EmptyMaterial {
    type Sampled = ();

    fn sample(&self, _uv: Pt2) -> Self::Sampled {}

    fn compute_scattering<'arena>(
        si: &Intersection<Self::Sampled>,
        _arena: &'arena Bump,
        _mode: TransportMode,
        _allow_multiple_lobes: bool,
    ) -> BSDF<'arena> {
        BSDF::new(si)
    }
}
