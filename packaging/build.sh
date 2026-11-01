#!/usr/bin/env bash
# Construit Bastion.AppImage à partir du workspace Rust.
#
# Prérequis :
#   - toolchain Rust + gcc (build-essential) installés sur le système
#   - appimagetool-x86_64.AppImage (téléchargé automatiquement dans packaging/)
#
# Usage : packaging/build.sh [--debug]   (release par défaut)
#
# NOTE GLIBC : le binaire doit être compilé SUR la distribution cible la
# plus ancienne visée (Debian 13 stable = glibc 2.41). Un binaire compilé
# sur une distribution plus récente (glibc 2.43+) refuse de démarrer sur
# Debian 13 avec l'erreur « version GLIBC_2.43 not found ». Le contrôle
# ci-dessous bloque l'empaquetage d'un tel binaire.

set -euo pipefail

# Modes sains pour l'AppDir : un umask restrictif (ex. 0027) créerait des
# répertoires non lisibles par les autres utilisateurs, qu'appimagetool
# refuse d'empaqueter.
umask 022

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PROFILE="release"
if [ "${1:-}" = "--debug" ]; then
    PROFILE="debug"
fi
[ "$PROFILE" = "release" ] && CARGO_FLAGS="--release" || CARGO_FLAGS=""

echo "── Compilation (profile : $PROFILE)…"
cargo build -p bastion $CARGO_FLAGS -j"$(nproc)"

BIN="$ROOT/target/$PROFILE/bastion"
VERSION="$(awk -F ' = ' '/^version/ {print $2; exit}' "$ROOT/Cargo.toml" | tr -d '"')"
OUT="$ROOT/dist"
APPDIR="$OUT/Bastion.AppDir"

# ── Garde GLIBC : le binaire ne doit pas exiger un glibc plus récent que
#    celui de la machine de construction (sinon l'AppImage cassera sur les
#    distributions plus anciennes, cf. erreur « GLIBC_2.43 not found »).
if command -v objdump >/dev/null 2>&1; then
    # Capture complète puis extraction en bash pur : un « head -1 » en aval
    # ferme le tube et tue « ldd » par SIGPIPE sous pipefail (exit 141,
    # de façon aléatoire).
    LDD_OUT="$(ldd --version)" || LDD_OUT=""
    LDD_LINE1="${LDD_OUT%%$'\n'*}"
    if [[ $LDD_LINE1 =~ ([0-9]+\.[0-9]+) ]]; then
        HOST_GLIBC="${BASH_REMATCH[1]}"
    else
        HOST_GLIBC=""
    fi
    MAX_REQ="$(objdump -T "$BIN" 2>/dev/null \
        | grep -oE 'GLIBC_[0-9]+(\.[0-9]+)?' | sort -Vu \
        | tail -1 | sed 's/GLIBC_//')"
    if [ -n "${MAX_REQ:-}" ] && [ -n "${HOST_GLIBC:-}" ]; then
        NEWEST="$(printf '%s\n%s\n' "$HOST_GLIBC" "$MAX_REQ" | sort -V | tail -1)"
        if [ "$NEWEST" != "$HOST_GLIBC" ]; then
            echo "ERREUR : le binaire exige GLIBC_${MAX_REQ} mais le système fournit ${HOST_GLIBC}." >&2
            echo "        Reconstruisez sur la distribution cible la plus ancienne (Debian 13)." >&2
            exit 1
        fi
        echo "── Binaire compatible glibc ≥ ${MAX_REQ} (système : ${HOST_GLIBC}) ✓"
    fi
else
    echo "⚠ objdump indisponible : contrôle GLIBC ignoré." >&2
fi

echo "── Assemblage de l'AppDir…"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/512x512/apps" "$APPDIR/usr/share/icons/hicolor/scalable/apps"

install -m 0755 "$BIN" "$APPDIR/usr/bin/bastion"
install -m 0644 "$ROOT/packaging/bastion.desktop" "$APPDIR/usr/share/applications/bastion.desktop"
install -m 0644 "$ROOT/assets/icon.png" "$APPDIR/usr/share/icons/hicolor/512x512/apps/bastion.png"
install -m 0644 "$ROOT/assets/bastionIcon.svg" "$APPDIR/usr/share/icons/hicolor/scalable/apps/bastion.svg"
install -m 0644 "$ROOT/assets/icon.png" "$APPDIR/bastion.png"
install -m 0644 "$ROOT/assets/icon.png" "$APPDIR/.DirIcon"
install -m 0644 "$ROOT/packaging/bastion.desktop" "$APPDIR/bastion.desktop"
install -m 0755 "$ROOT/packaging/AppRun" "$APPDIR/AppRun"

# Métadonnées : le fichier .desktop à la racine suffit pour l'intégration
# aux menus ; les métadonnées AppStream complètes (id rdns, captures…)
# pourront être ajoutées lors d'une publication en magasin d'applications.

# L'AppDir doit être lisible/exécutable par tous (cf. recommandation
# d'appimagetool) : les ACL restrictives de certains répertoires personnels
# donnent sinon des modes 0750 qui cassent l'AppImage chez d'autres users.
chmod -R a+rX "$APPDIR"

echo "── Téléchargement d'appimagetool si nécessaire…"
TOOL="$ROOT/packaging/appimagetool-x86_64.AppImage"
if [ ! -f "$TOOL" ]; then
    curl -fL --retry 3 -o "$TOOL" \
        "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"
    chmod +x "$TOOL"
fi

echo "── Empaquetage de Bastion-$VERSION-x86_64.AppImage…"
mkdir -p "$OUT"
ARCH=x86_64 "$TOOL" --appimage-extract-and-run "$APPDIR" "$OUT/Bastion-$VERSION-x86_64.AppImage"

# L'AppImage doit être exécutable (certains transferts/copy suppriment le
# bit +x, ce qui donne « Permission non accordée » au lancement).
chmod 0755 "$OUT/Bastion-$VERSION-x86_64.AppImage"

echo "── Vérification…"
"$OUT/Bastion-$VERSION-x86_64.AppImage" --version

echo ""
echo "✅ AppImage prêt : $OUT/Bastion-$VERSION-x86_64.AppImage"
ls -lh "$OUT/Bastion-$VERSION-x86_64.AppImage"
