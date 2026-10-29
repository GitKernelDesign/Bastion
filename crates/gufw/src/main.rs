//! GUFW — interface graphique de gestion UFW & Fail2Ban (local et SSH).
//!
//! Le même binaire sert de :
//!  - application graphique (défaut) ;
//!  - assistant root (`--serve`) lancé une fois via pkexec ;
//!  - sonde de version (`--version`).

mod app;
mod components;
mod config;
mod fonts;
mod icons;
mod modal;
mod theme;
mod toast;
mod ui;
mod worker;

use std::path::PathBuf;
use std::time::Duration;

fn main() {
    install_panic_hook();
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("GUFW v{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if args.iter().any(|a| a == "--serve") {
        serve(&args);
        return;
    }

    launch_gui();
}

/// Journalise toute panique dans ~/.config/gufw/panic.log : une panique dans
/// une tâche asynchrone est silencieuse sinon (tokio l'attrape), et l'app
/// paraît simplement « ne rien faire ».
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let dir = config::AppConfig::config_dir();
        let _ = std::fs::create_dir_all(&dir);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let line = format!(
            "[epoch {stamp}] panique dans le thread {:?} : {info}\n---\n",
            std::thread::current().name().unwrap_or("<anonyme>")
        );
        use std::io::Write;
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("panic.log"))
            .and_then(|mut f| f.write_all(line.as_bytes()));
        default_hook(info);
    }));
}

/// Mode assistant root : écoute la socket, exécute la liste blanche,
/// s'arrête après inactivité.
fn serve(args: &[String]) {
    let get = |flag: &str| -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };

    let uid: u32 = get("--uid")
        .or_else(|| std::env::var("PKEXEC_UID").ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            eprintln!("gufw --serve : --uid manquant");
            std::process::exit(2);
        });

    let socket = get("--socket")
        .map(PathBuf::from)
        .unwrap_or_else(|| gufw_core::helper::socket_path_for(uid));

    let timeout = get("--idle")
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(600));

    if let Err(e) = gufw_core::helper::run_helper_server(&socket, uid, timeout) {
        eprintln!("gufw --serve : {e}");
        std::process::exit(1);
    }
}

fn launch_gui() {
    let icon = egui::IconData {
        rgba: include_bytes!("../../../assets/icon64.rgba").to_vec(),
        width: 64,
        height: 64,
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("GUFW — Pare-feu & Fail2Ban")
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([1000.0, 640.0])
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };

    if let Err(e) = eframe::run_native(
        "GUFW",
        options,
        Box::new(|cc| {
            Ok(Box::new(app::GuiApp::new(cc)))
        }),
    ) {
        eprintln!("Erreur au démarrage de l'interface : {e}");
        std::process::exit(1);
    }
}
