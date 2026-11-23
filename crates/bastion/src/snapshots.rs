//! Rendu hors écran (tests uniquement).
//!
//! Permet de relire l'interface sans session graphique : chaque écran est
//! dessiné dans un contexte egui, tessellé puis rastérisé en PNG dans
//! `/tmp/bastion-snapshots`. Utile pour comparer deux itérations de design.
//!
//! ```bash
//! cargo test --bin bastion captures -- --ignored --nocapture
//! ```

use crate::app::{ConnPhase, GuiApp, Screen};
use crate::config::AppConfig;
use crate::i18n::Lang;
use crate::theme::Palette;
use crate::worker::WorkerHandle;
use bastion_core::model::*;
use egui::{Color32, Pos2, Rect, TextureId, Vec2};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};

const DIR: &str = "/tmp/bastion-snapshots";

/// Cas de capture : nom, écran, thème, taille logique, langue.
struct Case {
    name: &'static str,
    screen: Screen,
    dark: bool,
    size: Vec2,
    lang: Lang,
}

fn cases() -> Vec<Case> {
    let compact = Vec2::new(620.0, 900.0);
    let wide = Vec2::new(1180.0, 860.0);
    let tall = Vec2::new(1180.0, 1400.0);
    vec![
        Case { name: "connexion", screen: Screen::Connect, dark: false, size: compact, lang: Lang::Fr },
        Case { name: "pare-feu", screen: Screen::Firewall, dark: false, size: compact, lang: Lang::Fr },
        Case { name: "pare-feu-large", screen: Screen::Firewall, dark: false, size: wide, lang: Lang::Fr },
        Case { name: "pare-feu-complet", screen: Screen::Firewall, dark: false, size: tall, lang: Lang::Fr },
        Case { name: "pare-feu-sombre", screen: Screen::Firewall, dark: true, size: wide, lang: Lang::Fr },
        Case { name: "fail2ban", screen: Screen::Fail2Ban, dark: false, size: compact, lang: Lang::Fr },
        Case { name: "fail2ban-large", screen: Screen::Fail2Ban, dark: true, size: wide, lang: Lang::Fr },
        Case { name: "antivirus", screen: Screen::Antivirus, dark: false, size: compact, lang: Lang::Fr },
        Case { name: "journal", screen: Screen::Journal, dark: false, size: wide, lang: Lang::Fr },
        Case { name: "reglages", screen: Screen::Settings, dark: false, size: compact, lang: Lang::Fr },
        Case { name: "reglages-sombre", screen: Screen::Settings, dark: true, size: compact, lang: Lang::Fr },
        Case { name: "connexion-en", screen: Screen::Connect, dark: false, size: compact, lang: Lang::En },
        Case { name: "pare-feu-de", screen: Screen::Firewall, dark: false, size: compact, lang: Lang::De },
    ]
}

#[test]
#[ignore = "génère des captures PNG dans /tmp/bastion-snapshots"]
fn captures() {
    std::fs::create_dir_all(DIR).expect("dossier de captures");
    for case in cases() {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        let theme = if case.dark {
            crate::config::ThemeKind::Dark
        } else {
            crate::config::ThemeKind::Light
        };
        let mut app = sample_app(&ctx, case.lang, theme);
        // Un état différent par écran pour illustrer les cas réels.
        match case.screen {
            Screen::Connect => {
                app.phase = ConnPhase::Disconnected;
                app.conn_desc.clear();
            }
            Screen::Fail2Ban => app.f2b.selected_jail = Some("sshd".into()),
            _ => {}
        }
        app.screen = case.screen;
        let (w, h) = (
            (case.size.x * PPP) as usize,
            (case.size.y * PPP) as usize,
        );
        let img = render(&ctx, &mut app, case.size, theme);
        let path = format!("{DIR}/{}.png", case.name);
        write_png(&path, &img, w, h).expect("écriture PNG");
        println!("{path} ({w}×{h})");
    }
}

/// Jeu de données réaliste pour les captures.
fn sample_app(ctx: &egui::Context, lang: Lang, theme: crate::config::ThemeKind) -> GuiApp {
    let mut cfg = AppConfig::default();
    cfg.language = lang;
    cfg.theme = theme;
    cfg.auto_refresh = false;
    cfg.refresh_secs = 5;
    let mut app = GuiApp::build(ctx, cfg, WorkerHandle::inert());
    app.phase = ConnPhase::Connected;
    app.conn_desc = "Local (assistant root)".into();
    app.probe = (
        Some("ufw 0.36.2".into()),
        Some("Fail2Ban v1.1.1".into()),
    );
    app.status_msg = "Pare-feu activé".into();

    let now = Instant::now();
    app.firewall.status = Some(FirewallStatus {
        active: true,
        logging_on: true,
        logging_level: Some(LogLevel::Low),
        defaults: Defaults {
            incoming: Some(Policy::Deny),
            outgoing: Some(Policy::Allow),
            routed: Some(Policy::Deny),
            routed_disabled: false,
        },
        raw: "Status: active\nLogging: on (low)\nDefault: deny (incoming), allow (outgoing), disabled (routed)\n".into(),
    });
    app.firewall.status_at = Some(now - Duration::from_secs(4));
    app.firewall.ufw_service = Some("active".into());
    app.firewall.ufw_service_at = Some(now - Duration::from_secs(4));
    app.firewall.rules = vec![
        rule(1, RuleAction::Allow, RuleDir::In, "22/tcp", "Anywhere", Some("SSH"), false),
        rule(2, RuleAction::Allow, RuleDir::In, "443/tcp", "Anywhere", Some("HTTPS"), false),
        rule(3, RuleAction::Limit, RuleDir::In, "80/tcp", "Anywhere", Some("HTTP"), false),
        rule(4, RuleAction::Allow, RuleDir::In, "22/tcp (v6)", "Anywhere (v6)", Some("SSH v6"), true),
        rule(5, RuleAction::Deny, RuleDir::In, "5432", "10.0.0.0/8", Some("Postgres admin"), false),
        rule(6, RuleAction::Allow, RuleDir::Out, "53", "Anywhere", Some("DNS sortant"), false),
        rule(7, RuleAction::Allow, RuleDir::In, "Nginx Full", "Anywhere", None, false),
    ];
    app.firewall.profiles = vec!["Nginx Full".into(), "OpenSSH".into(), "CUPS".into()];
    app.firewall.rules_at = Some(now - Duration::from_secs(4));

    app.f2b.service = Some("active".into());
    app.f2b.service_at = Some(now - Duration::from_secs(4));
    app.f2b.global = Some(F2bGlobal {
        jail_count: 3,
        jails: vec!["sshd".into(), "nginx-http-auth".into(), "postfix-sasl".into()],
    });
    app.f2b.global_at = Some(now - Duration::from_secs(4));
    app.f2b.jail_infos.insert(
        "sshd".into(),
        (
            JailInfo {
                name: "sshd".into(),
                currently_failed: Some(42),
                total_failed: Some(1284),
                currently_banned: Some(3),
                total_banned: Some(217),
                file_list: Some("/var/log/auth.log".into()),
                banned_ips: vec!["203.0.113.42".into(), "198.51.100.7".into(), "192.0.2.199".into()],
                bantime: Some("1h".into()),
                findtime: Some("10m".into()),
                maxretry: Some("5".into()),
            },
            now - Duration::from_secs(6),
        ),
    );
    app.f2b.jail_infos.insert(
        "nginx-http-auth".into(),
        (
            JailInfo {
                name: "nginx-http-auth".into(),
                currently_failed: Some(12),
                total_failed: Some(310),
                currently_banned: Some(2),
                total_banned: Some(64),
                file_list: Some("/var/log/nginx/error.log".into()),
                banned_ips: vec!["203.0.113.88".into(), "198.51.100.23".into()],
                bantime: Some("6h".into()),
                findtime: Some("10m".into()),
                maxretry: Some("4".into()),
            },
            now - Duration::from_secs(6),
        ),
    );
    app.f2b.jail_infos.insert(
        "postfix-sasl".into(),
        (
            JailInfo {
                name: "postfix-sasl".into(),
                currently_failed: Some(0),
                total_failed: Some(7),
                currently_banned: Some(0),
                total_banned: Some(2),
                file_list: Some("/var/log/mail.log".into()),
                banned_ips: vec![],
                bantime: Some("1d".into()),
                findtime: Some("1h".into()),
                maxretry: Some("3".into()),
            },
            now - Duration::from_secs(6),
        ),
    );
    app.f2b.ban_input = String::new();

    app.clam.daemon = Some("active".into());
    app.clam.daemon_at = Some(now - Duration::from_secs(5));
    app.clam.freshclam = Some("active".into());
    app.clam.freshclam_at = Some(now - Duration::from_secs(5));
    app.clam.db = Some(ClamDb {
        engine: "1.4.3".into(),
        sigs: 2_861_247,
        db_date: "Fri Oct 30 07:26:48 2026".into(),
    });
    app.clam.db_at = Some(now - Duration::from_secs(5));
    app.clam.scan_path = "/home/benjamin/Téléchargements".into();

    let base = SystemTime::now();
    for (i, (cmd, ok)) in [
        ("ufw status verbose", true),
        ("ufw status numbered", true),
        ("fail2ban-client status", true),
        ("fail2ban-client status sshd", true),
        ("systemctl is-active ufw", true),
        ("systemctl is-active fail2ban", true),
        ("ufw allow 443/tcp comment 'HTTPS'", true),
        ("fail2ban-client set sshd unbanip 203.0.113.42", true),
        ("ufw delete 5", false),
        ("clamscan --version", true),
    ]
    .iter()
    .enumerate()
    {
        app.journal.push_back(crate::worker::LogEntry {
            at: base - Duration::from_secs((10 - i as u64) * 37),
            command: cmd.to_string(),
            ok: *ok,
            code: if *ok { 0 } else { 1 },
            detail: if *ok {
                String::new()
            } else {
                "ERROR: Could not delete non-existent rule".into()
            },
        });
    }

    app
}

fn rule(
    number: u32,
    action: RuleAction,
    dir: RuleDir,
    to: &str,
    from: &str,
    comment: Option<&str>,
    ipv6: bool,
) -> Rule {
    Rule {
        number,
        action,
        dir,
        to: to.into(),
        from: from.into(),
        comment: comment.map(|c| c.to_string()),
        ipv6,
    }
}

// ─── Rastérisation ──────────────────────────────────────────────────────────

const PPP: f32 = 2.0;

fn render(
    ctx: &egui::Context,
    app: &mut GuiApp,
    size: Vec2,
    theme: crate::config::ThemeKind,
) -> Vec<u8> {
    let mut textures: HashMap<TextureId, Tex> = HashMap::new();
    // Première passe : egui dimensionne les Areas (toasts, infobulles) avant
    // de les ancrer ; la seconde est la bonne.
    // Le temps avance entre les deux passes pour que les animations d'entrée
    // (fondu des zones, interrupteurs) soient terminées sur la capture.
    let _ = pass(ctx, app, size, &mut textures, 0.0);
    let prims = pass(ctx, app, size, &mut textures, 1.0);

    let w = (size.x * PPP).round() as usize;
    let h = (size.y * PPP).round() as usize;
    let mut canvas = Canvas::new(w, h, Palette::new(theme).bg);
    for prim in &prims {
        if let egui::epaint::Primitive::Mesh(mesh) = &prim.primitive {
            canvas.mesh(mesh, textures.get(&mesh.texture_id), prim.clip_rect);
        }
    }
    canvas.to_rgba()
}

/// Une passe egui complète ; retourne les primitives tessellées et met à jour
/// l'atlas de textures.
fn pass(
    ctx: &egui::Context,
    app: &mut GuiApp,
    size: Vec2,
    textures: &mut HashMap<TextureId, Tex>,
    time: f64,
) -> Vec<egui::ClippedPrimitive> {
    let raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        time: Some(time),
        ..Default::default()
    };
    ctx.begin_pass(raw);
    let _ = ctx.root_ui(|ui| app.frame(ui));
    let mut output = ctx.end_pass();
    let prims = ctx.tessellate(std::mem::take(&mut output.shapes), PPP);
    for (id, deltas) in &output.textures_delta.set {
        for delta in deltas {
            apply_delta(textures, *id, delta);
        }
    }
    // Marque les deltas comme traités (sinon epaint panique à la destruction).
    output.textures_delta.clear();
    prims
}

struct Tex {
    w: usize,
    h: usize,
    px: Vec<Color32>,
}

fn apply_delta(map: &mut HashMap<TextureId, Tex>, id: TextureId, delta: &egui::epaint::ImageDelta) {
    let egui::epaint::ImageData::Color(img) = &delta.image;
    match delta.pos {
        None => {
            map.insert(
                id,
                Tex {
                    w: img.size[0],
                    h: img.size[1],
                    px: img.pixels.clone(),
                },
            );
        }
        Some([x0, y0]) => {
            if let Some(t) = map.get_mut(&id) {
                for y in 0..img.size[1] {
                    for x in 0..img.size[0] {
                        let (tx, ty) = (x0 + x, y0 + y);
                        if tx < t.w && ty < t.h {
                            t.px[ty * t.w + tx] = img.pixels[y * img.size[0] + x];
                        }
                    }
                }
            }
        }
    }
}

struct Canvas {
    w: usize,
    h: usize,
    /// Couleurs prémultipliées, composantes dans [0, 1].
    px: Vec<[f32; 4]>,
}

impl Canvas {
    fn new(w: usize, h: usize, bg: Color32) -> Canvas {
        Canvas { w, h, px: vec![premul(bg); w * h] }
    }

    fn mesh(&mut self, mesh: &egui::epaint::Mesh, tex: Option<&Tex>, clip: Rect) {
        let (cx0, cy0) = (clip.min.x * PPP, clip.min.y * PPP);
        let (cx1, cy1) = (clip.max.x * PPP, clip.max.y * PPP);
        for tri in mesh.indices.chunks_exact(3) {
            let v = [
                &mesh.vertices[tri[0] as usize],
                &mesh.vertices[tri[1] as usize],
                &mesh.vertices[tri[2] as usize],
            ];
            let p: [Pos2; 3] = [
                Pos2::new(v[0].pos.x * PPP, v[0].pos.y * PPP),
                Pos2::new(v[1].pos.x * PPP, v[1].pos.y * PPP),
                Pos2::new(v[2].pos.x * PPP, v[2].pos.y * PPP),
            ];
            let det = (p[1].y - p[2].y) * (p[0].x - p[2].x) + (p[2].x - p[1].x) * (p[0].y - p[2].y);
            if det.abs() < 1e-6 {
                continue;
            }
            let minx = p.iter().map(|q| q.x).fold(f32::INFINITY, f32::min).max(cx0).max(0.0);
            let maxx = p.iter().map(|q| q.x).fold(f32::NEG_INFINITY, f32::max).min(cx1).min(self.w as f32);
            let miny = p.iter().map(|q| q.y).fold(f32::INFINITY, f32::min).max(cy0).max(0.0);
            let maxy = p.iter().map(|q| q.y).fold(f32::NEG_INFINITY, f32::max).min(cy1).min(self.h as f32);
            if !(minx.is_finite() && miny.is_finite()) || maxx <= minx || maxy <= miny {
                continue;
            }
            let col: [[f32; 4]; 3] = [premul(v[0].color), premul(v[1].color), premul(v[2].color)];
            for y in miny.floor() as usize..maxy.ceil() as usize {
                for x in minx.floor() as usize..maxx.ceil() as usize {
                    if x >= self.w || y >= self.h {
                        continue;
                    }
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    let w0 = ((p[1].y - p[2].y) * (fx - p[2].x) + (p[2].x - p[1].x) * (fy - p[2].y)) / det;
                    let w1 = ((p[2].y - p[0].y) * (fx - p[2].x) + (p[0].x - p[2].x) * (fy - p[2].y)) / det;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                        continue;
                    }
                    let uv = egui::Pos2::new(
                        v[0].uv.x * w0 + v[1].uv.x * w1 + v[2].uv.x * w2,
                        v[0].uv.y * w0 + v[1].uv.y * w1 + v[2].uv.y * w2,
                    );
                    let t = tex.map(|t| t.sample(uv)).unwrap_or([1.0, 1.0, 1.0, 1.0]);
                    let mut src = [0.0f32; 4];
                    for k in 0..4 {
                        src[k] = (col[0][k] * w0 + col[1][k] * w1 + col[2][k] * w2) * t[k];
                    }
                    self.blend(x, y, src);
                }
            }
        }
    }

    fn blend(&mut self, x: usize, y: usize, src: [f32; 4]) {
        let dst = &mut self.px[y * self.w + x];
        let inv = 1.0 - src[3].clamp(0.0, 1.0);
        for k in 0..4 {
            dst[k] = src[k] + dst[k] * inv;
        }
    }

    fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.w * self.h * 4);
        for px in &self.px {
            let a = px[3].clamp(0.0, 1.0);
            let inv = if a > 0.0 { 1.0 / a } else { 0.0 };
            for k in 0..3 {
                out.push(((px[k] * inv).clamp(0.0, 1.0) * 255.0).round() as u8);
            }
            out.push((a * 255.0).round() as u8);
        }
        out
    }
}

impl Tex {
    fn sample(&self, uv: egui::Pos2) -> [f32; 4] {
        if self.w == 0 || self.h == 0 {
            return [1.0, 1.0, 1.0, 1.0];
        }
        let x = (uv.x * self.w as f32 - 0.5).clamp(0.0, self.w as f32 - 1.0);
        let y = (uv.y * self.h as f32 - 0.5).clamp(0.0, self.h as f32 - 1.0);
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.w - 1), (y0 + 1).min(self.h - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let p00 = premul(self.px[y0 * self.w + x0]);
        let p10 = premul(self.px[y0 * self.w + x1]);
        let p01 = premul(self.px[y1 * self.w + x0]);
        let p11 = premul(self.px[y1 * self.w + x1]);
        let mut out = [0.0f32; 4];
        for k in 0..4 {
            let top = p00[k] * (1.0 - fx) + p10[k] * fx;
            let bot = p01[k] * (1.0 - fx) + p11[k] * fx;
            out[k] = top * (1.0 - fy) + bot * fy;
        }
        out
    }
}

fn premul(c: Color32) -> [f32; 4] {
    let [r, g, b, a] = c.to_array();
    let a = a as f32 / 255.0;
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a]
}

fn write_png(path: &str, rgba: &[u8], w: usize, h: usize) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(path)?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header()?;
    writer.write_image_data(rgba)?;
    Ok(())
}

