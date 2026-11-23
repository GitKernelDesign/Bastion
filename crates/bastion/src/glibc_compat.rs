//! Compatibilité glibc de l'AppImage.
//!
//! Le binaire est compilé sur Debian testing (glibc 2.43), où les variantes
//! `float` de quelques fonctions mathématiques (`acosf`, `asinf`, `atan2f`,
//! `acoshf`, `coshf`, `sinhf`) ont reçu de nouvelles versions de symbole.
//! Sans ce module, l'AppImage exigerait `GLIBC_2.43` et refuserait de démarrer
//! sur Debian 13 (2.41) ou Ubuntu 24.04 (2.39) — alors que ces fonctions ne
//! servent qu'au calcul de formes (dépendance `naga`/`wgpu`).
//!
//! On fournit des définitions locales, exprimées en double précision : les
//! références se lient à ces symboles non versionnés, et le binaire revient à
//! une exigence `GLIBC_2.39` (les symboles `pidfd_*` faibles de la
//! bibliothèque standard). Le résultat est identique à l'arrondi `f32` près.

#[cfg(all(target_os = "linux", target_env = "gnu"))]
mod shim {
    // Chaque fonction est volontairement `#[no_mangle]` : elle remplace le
    // symbole versionné de la libm du système au moment de l'édition de liens.

    /// `acosf` en double précision.
    #[no_mangle]
    #[inline(never)]
    pub extern "C" fn acosf(x: f32) -> f32 {
        f64::acos(x as f64) as f32
    }

    /// `asinf` en double précision.
    #[no_mangle]
    #[inline(never)]
    pub extern "C" fn asinf(x: f32) -> f32 {
        f64::asin(x as f64) as f32
    }

    /// `atan2f` en double précision.
    #[no_mangle]
    #[inline(never)]
    pub extern "C" fn atan2f(y: f32, x: f32) -> f32 {
        f64::atan2(y as f64, x as f64) as f32
    }

    /// `acoshf` en double précision.
    #[no_mangle]
    #[inline(never)]
    pub extern "C" fn acoshf(x: f32) -> f32 {
        f64::acosh(x as f64) as f32
    }

    /// `coshf` en double précision.
    #[no_mangle]
    #[inline(never)]
    pub extern "C" fn coshf(x: f32) -> f32 {
        f64::cosh(x as f64) as f32
    }

    /// `sinhf` en double précision.
    #[no_mangle]
    #[inline(never)]
    pub extern "C" fn sinhf(x: f32) -> f32 {
        f64::sinh(x as f64) as f32
    }
}

#[cfg(all(test, target_os = "linux", target_env = "gnu"))]
mod tests {
    use super::shim::*;

    /// Les shims doivent rester de simples enveloppes (pas de récursion) et
    /// donner les valeurs attendues à 1e-6 près.
    #[test]
    fn shims_donnent_les_bonnes_valeurs() {
        assert!((acosf(0.5) - 1.047_197_6).abs() < 1e-6);
        assert!((asinf(0.5) - 0.523_598_8).abs() < 1e-6);
        assert!((atan2f(1.0, 1.0) - 0.785_398_2).abs() < 1e-6);
        assert!((acoshf(2.0) - 1.316_958).abs() < 1e-5);
        assert!((coshf(0.0) - 1.0).abs() < 1e-6);
        assert!(sinhf(0.0).abs() < 1e-6);
        assert!((coshf(1.0) - 1.543_080_6).abs() < 1e-6);
    }
}
