#!/usr/bin/env bash
# Construit GUFW.AppImage à partir du workspace Rust.
#
# Prérequis :
#   - toolchain Rust + environnement local (source ./env.sh si nécessaire)
#   - appimagetool-x86_64.AppImage (téléchargé automatiquement dans packaging/)
#
# Usage : packaging/build.sh [--debug]   (release par défaut)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Prérequis : toolchain Rust + gcc (build-essential) installés sur le système.

PROFILE="release"
if [ "${1:-}" = "--debug" ]; then
    PROFILE="debug"
fi
[ "$PROFILE" = "release" ] && CARGO_FLAGS="--release" || CARGO_FLAGS=""

echo "── Compilation (profile : $PROFILE)…"
cargo build -p gufw $CARGO_FLAGS -j"$(nproc)"

BIN="$ROOT/target/$PROFILE/gufw"
VERSION="$(awk -F ' = ' '/^version/ {print $2; exit}' "$ROOT/Cargo.toml" | tr -d '"')"
OUT="$ROOT/dist"
APPDIR="$OUT/GUFW.AppDir"

echo "── Assemblage de l'AppDir…"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/512x512/apps"

install -m 0755 "$BIN" "$APPDIR/usr/bin/gufw"
install -m 0644 "$ROOT/packaging/gufw.desktop" "$APPDIR/usr/share/applications/gufw.desktop"
install -m 0644 "$ROOT/assets/icon.png" "$APPDIR/usr/share/icons/hicolor/512x512/apps/gufw.png"
install -m 0644 "$ROOT/assets/icon.png" "$APPDIR/gufw.png"
install -m 0644 "$ROOT/assets/icon.png" "$APPDIR/.DirIcon"
install -m 0644 "$ROOT/packaging/gufw.desktop" "$APPDIR/gufw.desktop"
install -m 0755 "$ROOT/packaging/AppRun" "$APPDIR/AppRun"

# Métadonnées : le fichier .desktop à la racine suffit pour l'intégration
# aux menus ; les métadonnées AppStream complètes (id rdns, captures…)
# pourront être ajoutées lors d'une publication en magasin d'applications.

echo "── Téléchargement d'appimagetool si nécessaire…"
TOOL="$ROOT/packaging/appimagetool-x86_64.AppImage"
if [ ! -f "$TOOL" ]; then
    curl -fL --retry 3 -o "$TOOL" \
        "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"
    chmod +x "$TOOL"
fi

echo "── Empaquetage de GUFW-$VERSION-x86_64.AppImage…"
mkdir -p "$OUT"
ARCH=x86_64 "$TOOL" --appimage-extract-and-run "$APPDIR" "$OUT/GUFW-$VERSION-x86_64.AppImage"

echo "── Vérification…"
"$OUT/GUFW-$VERSION-x86_64.AppImage" --version

echo ""
echo "✅ AppImage prêt : $OUT/GUFW-$VERSION-x86_64.AppImage"
ls -lh "$OUT/GUFW-$VERSION-x86_64.AppImage"
