#!/bin/sh
set -e
export PATH="$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")"
TAG="$(date +%Y%m%d-%H%M%S)"
HACHIMI_BUILD_TAG="exp-$(date +%H%M)" HACHIMI_IGNORE_DIRTY=true cargo build --release 2>&1 | tail -2
DLL="target/release/hachimi.dll"
MD5=$(md5sum "$DLL" | cut -d' ' -f1)
VER=$(grep -ao "v[0-9.]*-[a-z0-9]*-bv[^\"]*" "$DLL" | head -1)
OUT="C:/Users/Schwarz/projects/Hachimi-Edge-builds/${TAG}_${VER}_${MD5:0:8}.dll"
cp -f "$DLL" "$OUT"
echo "ARCHIVED: $OUT"
echo "MD5: $MD5"
