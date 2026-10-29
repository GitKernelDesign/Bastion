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
    // système d'egui reste en repli (symboles manquants…). Material Icons
    // termine la liste : egui choisit la police par glyphe, ce qui permet
    // d'insérer une icône au milieu d'un texte.
    let prop = fonts.families.entry(FontFamily::Proportional).or_default();
    prop.insert(0, "plex_sans".into());
    prop.push("material_icons".into());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "plex_mono".into());

    // Graisses : police principale + repli regular (glyphe manquant) puis
    // icônes (mixage icône + libellé dans un même texte).
    let weight = |fonts: &mut FontDefinitions, family: &str, main: &str, fallback: &str| {
        fonts.families.insert(
            FontFamily::Name(family.into()),
            vec![main.into(), fallback.into(), "material_icons".into()],
        );
    };
    weight(&mut fonts, "Medium", "plex_sans_medium", "plex_sans");
    weight(&mut fonts, "SemiBold", "plex_sans_semibold", "plex_sans");
    weight(&mut fonts, "Bold", "plex_sans_bold", "plex_sans");
    fonts.families.insert(
        FontFamily::Name("MonoMedium".into()),
        vec![
            "plex_mono_medium".into(),
            "plex_mono".into(),
            "material_icons".into(),
        ],
    );
    // Famille « Icon » : uniquement des icônes (aucun texte latin).
    fonts.families.insert(
        FontFamily::Name("Icon".into()),
        vec!["material_icons".into()],
    );

    ctx.set_fonts(fonts);
}
