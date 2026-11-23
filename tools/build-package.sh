#!/usr/bin/env bash
# The delivered package, built twice (the SDK's lock regenerated each time) and compared.
#
#   build-slot.sh run --job W1-industrialforegoingsouls --port ports/industrialforegoingsouls \
#       --step build -- bash tools/build-package.sh
#
# Each archive is stripped of the `Cargo.lock` the SDK writes once the manifest declares a
# dependency: the package's own `Cargo.toml` has only the SDK path dependency, so the lock is not a
# deliverable. The stripped archive is what lands in the port root and is compared byte for byte.
set -euo pipefail
PORT=$(cd "$(dirname "$0")/.." && pwd)
cd "$PORT"
NAME=industrialforegoingsouls
SCRATCH=${E2E_TMP:-$HOME/.cache/horizon-e2e/scratch}
mkdir -p "$SCRATCH"
WORK=$(mktemp -d "$SCRATCH/${NAME}-build.XXXXXX")
export CARGO_INCREMENTAL=0

for tag in A B; do
    echo "== build $tag =="
    rm -f Cargo.lock
    cargo +1.99.1 horizon-mod build 2>&1 | tail -4
    cp -f "target/horizon-mod/$NAME.hmod" "$NAME.hmod"
    zip -q -d "$NAME.hmod" Cargo.lock 2>/dev/null || true
    cp -f "$NAME.hmod" "$WORK/$tag.hmod"
    echo "$tag PACKED-LOCK: $(unzip -l "$WORK/$tag.hmod" | grep -c 'Cargo.lock' || true)"
done
if cmp "$WORK/A.hmod" "$WORK/B.hmod"; then
    echo CMP-IDENTICAL
else
    echo CMP-DIFFERENT
    exit 1
fi
sha256sum "$WORK/A.hmod"
ls -l "$WORK/A.hmod"
unzip -l "$WORK/A.hmod" | tail -3
echo "== the delivered file =="
sha256sum "$NAME.hmod"
ls -l "$NAME.hmod"
