#!/usr/bin/env bash
# Build one launcher game that carries BOTH this port and its parent, then hand its paths to
# `tools/e2e.sh`. A one-mod game is what `ports/tools/launcher-game.sh build` makes, and this port
# cannot run in one: the Soul Laser Base's catalyst slot reads
# `industrialforegoing:blue_laser_lens`, the parent mod's item, exactly as upstream's
# `catalyst.getStackInSlot(0).getItem().equals(ModuleCore.LASER_LENS[11].get())` does. A game
# without the parent answers `mod industrialforegoingsouls requested unknown item
# industrialforegoing:blue_laser_lens` at load and stops — the required-dependency refusal, since
# `horizon-mod.toml` has no dependency field to declare it in.
#
# The launcher builds "a folder of them" (`native-build <folder>`), so the two ports are copied into
# one folder, built, and the resulting version is exported the way `launcher-game.sh` exports a
# single one (executable, `lib*.so`, `horizon-native-mods/`, hard links).
#
#   ports/industrialforegoingsouls/tools/e2e-two-mod.sh build      # build + export, print the env
#   ports/industrialforegoingsouls/tools/e2e-two-mod.sh run        # build (if needed) + leased e2e
#
# `run` goes through `ports/tools/build-slot.sh run --step e2e`, so the campaign's concurrency
# accounting sees it: the launcher compiles no cargo workspace, one job, no remote forwarding.
set -uo pipefail

TOOLS=$(cd "$(dirname "$0")" && pwd)
PORT=$(cd "$TOOLS/.." && pwd)
ROOT=$(cd "$PORT/.." && pwd)
CAMPAIGN=$(cd "$ROOT/.." && pwd)
PARENT=$ROOT/industrialforegoing
MOD=industrialforegoingsouls
BUNDLE_NAME=$MOD-bundle

LAUNCHER_VERSION=0.2.92
LAUNCHER_DIR=$ROOT/_shared/bin/launcher-$LAUNCHER_VERSION
SDK_PIN=0a875da91dbb0fdcb7ba8c42c11341aa229af51a
MIXINS_PIN=e59da9094f0c2fdda6c627fe9eff22dec6a906f0
PIN=${SDK_PIN:0:7}

E2E_ROOT=${HORIZON_E2E_ROOT:-$HOME/.cache/horizon-e2e}
DATA=$E2E_ROOT/root/data
CONFIG=$E2E_ROOT/root/config
HOME_DIR=$E2E_ROOT/root/home
VERSIONS=$DATA/horizon-launcher/versions
INSTANCE=${HORIZON_E2E_INSTANCE:-e2e}
RES_VERSION=26.3
OUT=$E2E_ROOT/$PIN/$BUNDLE_NAME

say() { printf '%s: %s\n' "${0##*/}" "$*" >&2; }
die() { printf '%s: %s\n' "${0##*/}" "$*" >&2; exit "${2:-1}"; }

case "$E2E_ROOT" in
    /tmp/*|/tmp) die "HORIZON_E2E_ROOT=$E2E_ROOT is under /tmp (a 7.5 GB tmpfs that has been full)" 2 ;;
esac

launcher() {
    env XDG_DATA_HOME="$DATA" XDG_CONFIG_HOME="$CONFIG" HOME="$HOME_DIR" \
        "$LAUNCHER_DIR/horizon-launcher" "$@"
}

tags_now() {
    launcher versions --installed --json 2>/dev/null \
        | python3 -c 'import json,sys; print(" ".join(json.load(sys.stdin).get("installed",[])))'
}

nightly_tag() {
    tags_now | tr ' ' '\n' | grep -m1 -- "-$PIN\$"
}

# The newest installed native build whose manifest carries this mod *and* its parent.
newest_two_mod_tag() {
    local dir
    for dir in $(ls -1dt "$VERSIONS"/native-* 2>/dev/null); do
        [ -f "$dir/native-mods.json" ] || continue
        if python3 - "$dir/native-mods.json" "$MOD" <<'PY' >/dev/null 2>&1
import json, sys
mods = {m["id"] for m in json.load(open(sys.argv[1]))["mods"]}
sys.exit(0 if sys.argv[2] in mods and "industrialforegoing" in mods else 1)
PY
        then
            basename "$dir"
            return 0
        fi
    done
    return 1
}

# The folder of both mods the launcher builds. `--exclude .horizon`: the launcher compiles against
# its own copy of the SDK, and a port's fetched one is 34 MB of duplicate.
bundle() {
    local dir=$E2E_ROOT/scratch/$BUNDLE_NAME
    rm -rf "$dir"
    mkdir -p "$dir" || die "cannot create $dir" 3
    for mod in industrialforegoing "$MOD"; do
        [ -f "$ROOT/$mod/horizon-mod.toml" ] || die "no $mod port at $ROOT/$mod" 2
        mkdir -p "$dir/$mod"
        rsync -a --exclude .horizon --exclude target --exclude '*.hmod' --exclude '*.hnmod' \
            "$ROOT/$mod/" "$dir/$mod/" || die "cannot copy $mod into the bundle" 3
    done
    echo "$dir"
}

export_version() { # tag outdir
    local tag=$1 out=$2 f
    rm -rf "$out"
    mkdir -p "$out" || die "cannot create $out" 3
    for f in "$VERSIONS/$tag"/horizon-client "$VERSIONS/$tag"/horizon-ctl \
             "$VERSIONS/$tag"/lib*.so "$VERSIONS/$tag"/horizon-wasm-std.wasm \
             "$VERSIONS/$tag"/research-lock.json "$VERSIONS/$tag"/PLAY.txt; do
        [ -e "$f" ] || continue
        ln -f "$f" "$out/" 2>/dev/null || cp -a "$f" "$out/"
    done
    cp -a "$VERSIONS/$tag/horizon-native-mods" "$out/" || die "the build has no horizon-native-mods" 3
    cp -a "$VERSIONS/$tag/native-mods.json" "$out/" || die "the build has no native-mods.json" 3
    if ldd "$out/horizon-client" 2>/dev/null | grep -q 'not found'; then
        die "the exported game in $out misses a library: $(ldd "$out/horizon-client" | grep 'not found' | tr '\n' ' ')" 9
    fi
}

env_block() {
    printf 'export GAME=%q\n' "$OUT/horizon-client"
    printf 'export CTL=%q\n' "$OUT/horizon-ctl"
    printf 'export CONTROL_GAME=%q\n' "$E2E_ROOT/$PIN/control/horizon-client"
    printf 'export RES=%q\n' "$VERSIONS/$(nightly_tag)/.local/horizon-client/$RES_VERSION"
    printf 'export HORIZON_RES=%q\n' "$VERSIONS/$(nightly_tag)/.local/horizon-client/$RES_VERSION"
    printf 'export E2E_TMP=%q\n' "$E2E_ROOT/scratch"
    printf 'export SOCK=%q\n' "$E2E_ROOT/scratch/$MOD-e2e.sock"
    printf 'export GSOCK=%q\n' "$E2E_ROOT/scratch/$MOD-e2e-guest.sock"
}

build() {
    [ -x "$LAUNCHER_DIR/horizon-launcher" ] || die "no launcher at $LAUNCHER_DIR (run ports/tools/launcher-game.sh ensure)" 2
    local tag
    tag=$(nightly_tag)
    [ -n "$tag" ] || die "the Nightly at $SDK_PIN is not installed (run ports/tools/launcher-game.sh build $MOD once)" 5
    [ -f "$VERSIONS/$tag/.local/horizon-client/$RES_VERSION/horizon-resources.json" ] \
        || die "$tag is not prepared (run ports/tools/launcher-game.sh build $MOD once)" 7
    local dir before built
    dir=$(bundle)
    before=$(tags_now)
    say "native-build both mods against $tag"
    launcher native-build "$dir" --instance "$INSTANCE" --trust-native-code >&2 \
        || die "horizon-launcher native-build failed for the bundle" 10
    # A new tag, or — when the launcher reuses a matching build, which it does for unchanged
    # sources — the newest tag whose manifest carries both mods.
    built=$(tags_now | tr ' ' '\n' | while read -r t; do
        case " $before " in *" $t "*) continue ;; esac
        case "$t" in native-*) echo "$t"; break ;; esac
    done)
    if [ -z "$built" ]; then
        built=$(newest_two_mod_tag)
    fi
    [ -n "$built" ] || die "native-build reported success but no version carries both mods" 10
    export_version "$built" "$OUT"
    say "built $built; exported to $OUT"
    say "the build carries: $(python3 -c "
import json,sys
mods=json.load(open('$OUT/native-mods.json')).get('mods',[])
print(', '.join(m['id']+' '+m['version'] for m in mods))
")"
}

case "${1:-build}" in
    build) build; env_block ;;
    env)   env_block ;;
    run)
        # Always: the launcher reuses a matching build in a second, and a stale binary here would
        # report yesterday's code.
        build >&2 || exit $?
        eval "$(env_block)"
        export GAME CTL CONTROL_GAME RES HORIZON_RES E2E_TMP SOCK GSOCK
        [ -x "$CONTROL_GAME" ] || say "no control export at $CONTROL_GAME: the control leg needs ports/tools/launcher-game.sh build $MOD once"
        exec "$ROOT/tools/build-slot.sh" run --job "e2e-$MOD" --port "ports/$MOD" --step e2e -- \
            bash "$PORT/tools/e2e.sh"
        ;;
    *) die "unknown command $1 (build | env | run)" 64 ;;
esac
