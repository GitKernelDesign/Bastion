//! Polices embarquées : IBM Plex Sans (interface), IBM Plex Mono (données
//! techniques), Material Icons (icônes). Aucune dépendance système.

use egui::{FontData, FontDefinitions, FontFamily};

macro_rules! font_bytes {
    ($file:expr) => {
        include_bytes!(concat!("../../../assets/fonts/", $file))
    };
}

/// Installe les polices et retourne les noms de familles nommées.
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let add = |fonts: &mut FontDefinitions, name: &str, bytes: &'static [u8]| {
        fonts
            .font_data
            .insert(name.into(), FontData::from_owned(bytes.to_vec()).into());
    };

    add(
        &mut fonts,
        "plex_sans",
        font_bytes!("IBMPlexSans-Regular.ttf"),
    );
    add(
        &mut fonts,
        "plex_sans_medium",
        font_bytes!("IBMPlexSans-Medium.ttf"),
    );
    add(
        &mut fonts,
        "plex_sans_semibold",
        font_bytes!("IBMPlexSans-SemiBold.ttf"),
    );
    add(&mut fonts, "plex_sans_bold", font_bytes!("IBMPlexSans-Bold.ttf"));
    add(
        &mut fonts,
        "plex_mono",
        font_bytes!("IBMPlexMono-Regular.ttf"),
    );
    add(
        &mut fonts,
        "plex_mono_medium",
        font_bytes!("IBMPlexMono-Medium.ttf"),
    );
    add(
        &mut fonts,
        "material_icons",
        font_bytes!("MaterialIcons-Regular.ttf"),
    );

    // Familles par défaut : la police embarquée passe en tête, la police
    // système d'egui reste en repli (symboles manquants…).
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "plex_sans".into());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "plex_mono".into());

    // Graisses et familles spéciales.
    fonts.families.insert(
        FontFamily::Name("Medium".into()),
        vec!["plex_sans_medium".into()],
    );
    fonts.families.insert(
        FontFamily::Name("SemiBold".into()),
        vec!["plex_sans_semibold".into()],
    );
    fonts.families.insert(
        FontFamily::Name("Bold".into()),
        vec!["plex_sans_bold".into()],
    );
    fonts.families.insert(
        FontFamily::Name("MonoMedium".into()),
        vec!["plex_mono_medium".into()],
    );
    fonts.families.insert(
        FontFamily::Name("Icon".into()),
        vec!["material_icons".into()],
    );

    ctx.set_fonts(fonts);
}
