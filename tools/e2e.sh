#!/usr/bin/env bash
# In-game check for Industrial Foregoing: Souls 0.2.3 — the one soul economy, its efficiency
# augment, its waterlogging, and the parent laser machinery's use of its ore recipe.
#
# The Soul Laser Base seats `industrialforegoing:blue_laser_lens` in its catalyst slot and, with a
# warden inside the box above it, turns 20 ticks into 4 damage and 2 souls; the Soul Pipe pulls 4 a
# tick out of it into a network of 4 souls a pipe; the Soul Surge spends one soul on 300 accelerated
# ticks and random-ticks the block it is mounted on four times at a 3% chance a tick, while it is not
# powered.
#
# The differential this scenario turns over is the lens: **with it the machine makes souls, without it
# it makes none** — the same machine, the same warden, 60 ticks each — which is the gate the whole mod
# hangs on (`catalyst.getStackInSlot(0).getItem().equals(ModuleCore.LASER_LENS[11].get())`). The
# surge's differential is the control crop: `random_tick_speed 0` means nothing grows on its own, so a
# wheat that grew did so because the surge random-ticked it.
#
# Every mod command is typed with `ctl chat "/cmd"` because `ctl command` runs the console path and
# bypasses the native `perform_command` hook.
#
# Phase R proves the waterlogging upstream's `SimpleWaterloggedBlock` gives both the pipe and the
# surge: the state is declared (`waterlogged`), the engine reads water from it, and the block's own
# click fills and drains it with a bucket — the interface itself is a template's, which a content
# block cannot attach (`notes.md` §3 has the measurement).
#
# The package is one flat `.hmod` (code and content together), so the game must be one the launcher
# built with it — no `--dev-mod`:
#
#   eval "$(ports/tools/launcher-game.sh env industrialforegoingsouls)"
#   build-slot.sh run --job W1-industrialforegoingsouls --port ports/industrialforegoingsouls \
#       --step e2e -- bash ports/industrialforegoingsouls/tools/e2e.sh
#
# The **control leg** is the same script on `GAME=$CONTROL_GAME`: the same Nightly, unmodified, with
# no `libindustrialforegoingsouls.so` and no `horizon-native-mods/industrialforegoingsouls/`. It fails
# at the first item assertion, which is what makes the modded run evidence.
#
# SELFTEST=1 flips phase B's "no lens, no souls" and SELFTEST=grow flips phase G's control crop (the
# unaccelerated wheat must have stayed at age 0); SELFTEST=augment asserts the plain bar's rate on a
# machine with the augment seated, and SELFTEST=laser asserts the parent's base yielded no echo
# shard. Each must FAIL against the 0.2.2 package, which is what makes the bearing leg evidence; a
# run that fails never reaches PASS.
#
# SELFTEST=carry is slice 3's own leg: phase P asserts that a machine broken with its lens and its
# efficiency addon comes back **empty** — souls 0, lens none — which must fail against a package
# that carries them. SELFTEST=replace inverts phase Q: a second lens must void the first one, which
# must fail against a package that hands it back. SELFTEST=waterlog inverts phase R: the water bucket
# must have left the pipe dry, which fails against a package whose click fills it.
set -Eeuo pipefail
export LC_ALL=C
[ -z "${IF_DEBUG:-}" ] || set -x

CLONE=${CLONE:-/home/adlf/Desktop/all_the_mods_10_rust_rewrite}
PORT=$CLONE/ports/industrialforegoingsouls
CTL=${CTL:-}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/industrialforegoingsouls-e2e.sock}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
WORLD=IndustrialForegoingSoulsE2E
# The machine stands here, in the End's void far from the central island.
X0=194
BASE_POS="$X0 101 0"
# The crop plots: A is accelerated by a surge, B is the control.
AX=202
BX=206
CY=102

[ -n "$CTL" ] || {
    echo "CTL is unset: eval \"\$(ports/tools/launcher-game.sh env industrialforegoingsouls)\"" >&2
    exit 1
}
[ -n "$GAME" ] || {
    echo "GAME is unset: the SDK package is compiled into the game by the launcher." >&2
    echo "  ports/tools/launcher-game.sh build industrialforegoingsouls" >&2
    exit 1
}
[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

# /tmp is a 7.5 GB tmpfs that has run full and killed legs; E2E_TMP is on /home.
TMP_ROOT=${E2E_TMP:-/home/adlf/.cache/horizon-e2e/scratch}
mkdir -p "$TMP_ROOT"
SCRATCH=$(mktemp -d "$TMP_ROOT/industrialforegoingsouls-home.XXXXXX")
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" --timeout 180 "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() {
    echo "FAIL: $*" >&2
    echo "  inventory: $(ctl inventory 2>/dev/null | jq -c '.counts' 2>/dev/null || true)" >&2
    ctl logs --lines 60 >&2 || true
    exit 1
}
on_error() { echo "FAIL: line $1: '$2' exited $?" >&2; ctl logs --lines 40 >&2 || true; }
trap 'on_error "$LINENO" "$BASH_COMMAND"' ERR

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then echo "kept scratch: $SCRATCH" >&2; else rm -rf "$SCRATCH" 2>/dev/null || true; fi
    # Last line of every run: `check-e2e-logs.py` reads it as the verdict.
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch [WORLD]
    local world=${1:-$WORLD}
    rm -f "$SOCK"
    cd "$SCRATCH"
    "$CTL" --socket "$SOCK" launch --game "$GAME" --world "$world" --seed 20261105 --creative \
        --offscreen --wait --resources "$RES" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

# One property of the state string `ctl block` returns. The properties are sorted, so the key asked
# for is not always the first one: split the bracket body on commas and match the name (the 0.2.3
# run found this the hard way — the old `sed` only ever answered for the first property, which
# `facing` and `age` happened to be).
block_prop() { # block_prop X Y Z PROPERTY
    ctl block "$1" "$2" "$3" | jq -r '.state // ""' \
        | tr -d ']' | tr ',' '\n' \
        | awk -F= -v k="$4" '{ sub(/^.*\[/, "", $1); if ($1 == k) print $2 }'
}
block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }
held() { ctl inventory | jq -r --arg item "$1" '.counts[$item] // 0'; }

# The End's void at X0: a stone floor and clear air above it. The player is teleported first, so the
# chunks the fills touch are loaded.
arena() {
    ctl command "execute in minecraft:the_end run tp @s $((X0 - 6)).5 104 0.5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$X0" 100 0 | jq -r '.loaded // false')" = true ]; then loaded=true; break; fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill 186 100 -14 212 100 14 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill 186 101 -14 212 112 14 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
    ctl command "execute in minecraft:the_end run tp @s $((X0 - 2)).5 101 0.5" >/dev/null
    ctl wait 20 >/dev/null
    [ "$(block_name "$X0" 100 0)" = "minecraft:stone" ] || fail "the arena floor is not stone"
}

# `/industrialforegoingsouls info X Y Z`: the line, then its parsed fields.
info() { # info X Y Z
    say "industrialforegoingsouls info $1 $2 $3"
    ctl wait 10 >/dev/null
    INFO_LINE=$(ctl messages 12 | jq -r '.chat[]' | grep -m1 -E '^Soul (Laser Base|Pipe|Surge) at' || true)
    [ -n "$INFO_LINE" ] || fail "no /industrialforegoingsouls info reply"
    eval "$(python3 "$PORT/tools/parse_info.py" "$INFO_LINE")"
    [ "${ok:-0}" = 1 ] || fail "the info line did not parse: $INFO_LINE"
    echo "  $INFO_LINE" >&2
}

expect() { # expect NAME ACTUAL WANT
    [ "$2" = "$3" ] || fail "$1 is $2, expected $3"
}

# Summon a warden that cannot move: `NoAI:1b` freezes it completely at this pin (no AI, no gravity).
summon_warden() { # summon_warden X Y Z
    ctl command "execute in minecraft:the_end run summon minecraft:warden $1.5 $2 $3.5 {NoAI:1b,Health:500f}" >/dev/null
    ctl wait 10 >/dev/null
}

# Stand two blocks west of the machine and aim at its west face: a right-click there is a
# `use_item_on`, which is where the lens seat runs.
aim_machine() {
    ctl command "execute in minecraft:the_end run tp @s $((X0 - 2)).5 101 0.5" >/dev/null
    ctl wait 6 >/dev/null
    ctl look-at-block "$X0" 101 0 west >/dev/null 2>&1 || fail "cannot aim at the machine at $X0"
    ctl wait 6 >/dev/null
}

# --- the parent's laser machinery (phase M2) -------------------------------------------------
# The ore base and the drill are `industrialforegoing`'s blocks: this port only supplies the
# `laser_drill_ore` recipe they run. The helpers are the parent port's own (`tools/e2e.sh`).

# Place an item from the hotbar on the block under X Y Z, looking up at its floor.
place_any() { # place_any X Y Z ITEM
    local x=$1 y=$2 z=$3 item=$4
    ctl command "item replace entity @s hotbar.0 with $item 1" >/dev/null
    ctl wait 6
    ctl select 1 >/dev/null
    ctl command "execute in minecraft:the_end run tp @s $((x - 2)).5 $y $z.5" >/dev/null
    ctl wait 6
    ctl look-at-block "$x" $((y - 1)) "$z" up >/dev/null 2>&1 || fail "cannot aim at the floor under $x,$y,$z"
    ctl wait 8
    ctl place "$x" "$y" "$z" --item "$item" >/dev/null || fail "the $item did not place at $x,$y,$z"
    ctl wait 6
    [ "$(block_name "$x" "$y" "$z")" = "$item" ] \
        || fail "the $item did not place at $x,$y,$z (got '$(block_name "$x" "$y" "$z")')"
}

# The drill one block above the base at X Y Z, placed **from its item while looking down**, which
# is what makes its six-way `facing` `up` and puts the base in its working area below.
place_drill_above() { # place_drill_above X Y Z (of the base)
    local x=$1 y=$2 z=$3 item=industrialforegoing:laser_drill
    ctl command "item replace entity @s hotbar.0 with $item 1" >/dev/null
    ctl wait 6
    ctl select 1 >/dev/null
    # One east and five up: inside vanilla's reach, and the player does not land on the block
    # being placed before the placement lands.
    ctl command "execute in minecraft:the_end run tp @s $((x + 1)).5 $((y + 5)) $z.5" >/dev/null
    ctl wait 4
    ctl look-at-block "$x" "$y" "$z" up >/dev/null 2>&1 || fail "cannot aim at the base's top face"
    ctl wait 2
    ctl place "$x" $((y + 1)) "$z" --item "$item" >/dev/null || fail "the drill did not place above the base"
    ctl wait 4
    [ "$(block_name "$x" $((y + 1)) "$z")" = "$item" ] \
        || fail "the drill did not place at $x $((y + 1)) $z (got '$(block_name "$x" $((y + 1)) "$z")')"
}

# One `/industrialforegoing` command of a laser machine at its position.
lasay() { # lasay X Y Z SUB [ARGS...]
    local x=$1 y=$2 z=$3 sub=$4
    shift 4
    say "industrialforegoing $sub $x $y $z${*:+ $*}"
}

# `/industrialforegoing info` of a laser machine, read from the game's own `[CHAT]` log and then
# from the live message window (the two readers the parent's slice needed).
laser_info() { # laser_info X Y Z base|drill
    local x=$1 y=$2 z=$3 kind=$4 pattern before now line="" attempt
    if [ "$kind" = base ]; then
        pattern='Ore Laser Base at .*, biome '
    else
        pattern='Laser Drill at .*, target '
    fi
    before=$(ctl logs --lines 400 | grep -c "\[CHAT\] $pattern" || true)
    say "industrialforegoing info $x $y $z"
    for attempt in $(seq 1 12); do
        ctl wait 2 >/dev/null
        now=$(ctl logs --lines 400 | grep -c "\[CHAT\] $pattern" || true)
        if [ "$now" -gt "$before" ]; then
            line=$(ctl logs --lines 400 | grep "\[CHAT\] $pattern" \
                | tail -1 | sed 's/^.*\[CHAT\] //' || true)
            break
        fi
    done
    if [ -z "$line" ]; then
        for attempt in $(seq 1 10); do
            say "industrialforegoing info $x $y $z"
            ctl wait 3 >/dev/null
            line=$(ctl messages 60 | jq -r '.chat[]' | grep -E "$pattern" | tail -1 || true)
            [ -n "$line" ] && break
        done
    fi
    [ -n "$line" ] || fail "no fresh /industrialforegoing info reply for the laser machine"
    eval "$(python3 "$PORT/tools/parse_laser.py" "$line")"
    [ "${ok:-0}" = 1 ] || fail "the laser machine's info line did not parse: $line"
    echo "  $line" >&2
}

# One slot of the parent base's container: 0-5 are the lens slots, 6-20 the output slots.
base_slot() { # base_slot X Y Z N
    ctl container "$1" "$2" "$3" | jq -r --argjson n "$4" \
        '.slots[$n] | if . == null then "empty" else "\(.item) x\(.count)" end'
}

# --- a machine's contents on its own item (phase P) ------------------------------------------

# Stand two blocks west of X Y Z and aim at its west face, so a `use-item` is a `use_item_on`.
aim_block() { # aim_block X Y Z
    ctl command "execute in minecraft:the_end run tp @s $(($1 - 2)).5 $2 $3.5" >/dev/null
    ctl wait 6 >/dev/null
    ctl look-at-block "$1" "$2" "$3" west >/dev/null 2>&1 || fail "cannot aim at $1,$2,$3"
    ctl wait 6 >/dev/null
}

# A survival break with the pickaxe, then the drop collected. `block_drops` does not fire for a
# creative break (nothing drops), so this is the only path that carries anything.
break_and_collect() { # break_and_collect X Y Z ITEM
    local x=$1 y=$2 z=$3 item=$4
    ctl command "execute in minecraft:the_end run tp @s $((x - 2)).5 $y $z.5" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "item replace entity @s hotbar.0 with minecraft:iron_pickaxe 1" >/dev/null
    ctl select 1 >/dev/null
    ctl wait 4 >/dev/null
    if ! BROKEN=$(ctl break "$x" "$y" "$z" 2>&1); then
        fail "breaking $item at $x,$y,$z failed: $BROKEN"
    fi
    ctl wait 10 >/dev/null
    [ "$(block_name "$x" "$y" "$z")" = "minecraft:air" ] \
        || fail "the block at $x,$y,$z survived the break"
    # The drop is an item entity: `ctl collect` walks to it where the build has that verb, and
    # standing on the broken block's own position picks it up as the fallback.
    ctl collect 8 >/dev/null 2>&1 || true
    ctl wait 10 >/dev/null
    if ! ctl inventory | jq -e "[.hotbar[], .main[] | select(.item == \"$item\")] | length >= 1" >/dev/null 2>&1; then
        ctl command "execute in minecraft:the_end run tp @s $x.5 $y $z.5" >/dev/null
        ctl wait 20 >/dev/null
    fi
    ctl inventory | jq -e "[.hotbar[], .main[] | select(.item == \"$item\")] | length >= 1" >/dev/null \
        || fail "the broken $item was not picked up: $(ctl inventory | jq -c '{hotbar,main}')"
}

# The `/data get entity @s Inventory` reply: an item's own components ride with it, in whichever
# slot collected it (`SelectedItem` reads the hotbar only, and a collected drop can land in the
# main inventory).
carry_line() {
    ctl command "data get entity @s Inventory" >/dev/null
    ctl wait 6 >/dev/null
    ctl messages 8 | jq -r '.chat[]' | grep -m1 'has the following entity data' || true
}

# `ctl place --item` places from the hotbar only, so a stack that landed in the main inventory is
# moved into the last hotbar slot with the game's own `item` command (`from` copies, which is all a
# placement needs). Never `/item replace` with a freshly given machine: that proves nothing.
hold_carried() { # hold_carried ITEM
    local item=$1 slot
    if ctl inventory | jq -e "[.hotbar[] | select(.item == \"$item\")] | length >= 1" >/dev/null 2>&1; then
        return 0
    fi
    slot=$(ctl inventory | jq -r --arg i "$item" '[.main[] | select(.item == $i)][0].slot // empty')
    [ -n "$slot" ] || fail "the carried $item is in no inventory slot: $(ctl inventory | jq -c '{hotbar,main}')"
    ctl command "item replace entity @s hotbar.8 with minecraft:air" >/dev/null
    ctl wait 4 >/dev/null
    ctl command "item replace entity @s hotbar.8 from entity @s container.$slot" >/dev/null
    ctl wait 6 >/dev/null
    ctl inventory | jq -e "[.hotbar[] | select(.item == \"$item\")] | length >= 1" >/dev/null \
        || fail "the carried $item did not reach the hotbar: $(ctl inventory | jq -c '{hotbar,main}')"
}

# Place the carried stack at X Y Z, on the block below it (the player stands two blocks west).
place_kept() { # place_kept X Y Z ITEM
    local x=$1 y=$2 z=$3 item=$4 attempt reply=""
    ctl command "execute in minecraft:the_end run tp @s $((x - 2)).5 $y $z.5" >/dev/null
    ctl wait 6 >/dev/null
    hold_carried "$item"
    for attempt in 1 2 3; do
        ctl select "$item" >/dev/null 2>&1 || true
        ctl wait 3 >/dev/null
        ctl look-at-block "$x" $((y - 1)) "$z" up >/dev/null 2>&1 || true
        ctl wait 2 >/dev/null
        if reply=$(ctl place "$x" "$y" "$z" --item "$item" 2>&1); then
            ctl wait 6 >/dev/null
            [ "$(block_name "$x" "$y" "$z")" = "$item" ] \
                || fail "the carried $item did not place at $x,$y,$z (got '$(block_name "$x" "$y" "$z")')"
            return 0
        fi
        echo "  place attempt $attempt refused: $(tr -d '\n' <<<"$reply" | cut -c1-200)" >&2
        ctl command "execute in minecraft:the_end run kill @e[type=!player,x=$x,y=$y,z=$z,distance=..2]" \
            >/dev/null 2>&1 || true
        ctl wait 10 >/dev/null
    done
    fail "ctl place $item at $x,$y,$z was refused three times: $(ctl inventory | jq -c '{hotbar,main}')"
}


echo "== launch (The End arena, one package with content and code) =="
launch
arena
LOAD_ERRORS=$(ctl logs --lines 500 | grep -c -E "\[load-items/ERROR\]|\[horizon-recipes/ERROR\]|\[horizon-advancements/ERROR\]|\[content/ERROR\]" || true)
expect "the recipe/advancement/block load errors in the game's log" "${LOAD_ERRORS:-0}" 0

echo "== A: the three blocks register, place and read back; an unknown id is refused =="
for id in soul_laser_base soul_network_pipe soul_surge; do
    ctl give "industrialforegoingsouls:$id" 1 >/dev/null || fail "the game refused industrialforegoingsouls:$id"
done
ctl wait 8 >/dev/null
for id in soul_laser_base soul_network_pipe soul_surge; do
    [ "$(held "industrialforegoingsouls:$id")" -ge 1 ] \
        || fail "industrialforegoingsouls:$id did not register: $(ctl inventory | jq -c '.counts')"
done
echo "  the three blocks' items read back out of the inventory"
REFUSED=$(ctl give industrialforegoingsouls:not_a_real_item 1 2>&1 || true)
case "$REFUSED" in
    *"unknown item industrialforegoingsouls:not_a_real_item"*) ;;
    *) fail "an unregistered item id was not refused with the game's own message: $REFUSED" ;;
esac
REFUSED=$(ctl setblock 190 101 -6 industrialforegoingsouls:not_a_real_block 2>&1 || true)
case "$REFUSED" in
    *'"ok":true'*) fail "an unregistered block id was accepted: $REFUSED" ;;
esac
[ "$(block_name 190 101 -6)" = "minecraft:air" ] || fail "an unregistered block id changed the world"
echo "  an unregistered item and block id are refused"

ctl command "execute in minecraft:the_end run setblock $BASE_POS industrialforegoingsouls:soul_laser_base" >/dev/null
ctl wait 6 >/dev/null
[ "$(block_name $BASE_POS)" = "industrialforegoingsouls:soul_laser_base" ] \
    || fail "the laser base did not place: '$(block_name $BASE_POS)'"
summon_warden "$X0" 102 0
info $BASE_POS
expect "the block the readout is about" "$KIND" "base"
expect "the empty tank" "$SOULS" 0
expect "the seated lens" "$LENS" "none"
case "$WARDEN" in
    none|*[!0-9.]*) fail "the warden is not in the machine's box: warden=$WARDEN" ;;
    *) ;;
esac
[ "$(python3 -c "print(1 if float('$WARDEN') >= 400 else 0)")" = 1 ] \
    || fail "the summoned warden's health reads $WARDEN"
echo "  warden at $WARDEN health inside the box, the machine empty and lensless"

echo "== B: no lens, a warden in range, 60 ticks: no souls =="
ctl wait 60 >/dev/null
info $BASE_POS
if [ "$SELFTEST" = 1 ]; then
    echo "(selftest: asserting the soul tank is not empty)"
    [ "$SOULS" != 0 ] || fail "selftest: the soul tank is $SOULS, expected anything but 0"
fi
expect "the tank after 60 ticks without a lens" "$SOULS" 0
expect "the warden's health, untouched" "$WARDEN" "500.0"
echo "  60 ticks, no lens: souls 0, the warden at 500.0 health"

echo "== C: seat the lens: 60 ticks store two souls an operation and hurt the warden 4 =="
ctl command "give @s industrialforegoing:blue_laser_lens 1" >/dev/null
ctl wait 8 >/dev/null
ctl select industrialforegoing:blue_laser_lens >/dev/null 2>&1 || fail "the lens is not in the hotbar"
aim_machine
ctl use-item >/dev/null || fail "the right-click with the lens on the machine failed"
ctl wait 8 >/dev/null
info $BASE_POS
expect "the seated lens" "$LENS" "industrialforegoing:blue_laser_lens"
expect "the lens the seat took from the hand" "$(held industrialforegoing:blue_laser_lens)" 0
ctl wait 60 >/dev/null
info $BASE_POS
[ "$SOULS" -gt 0 ] || fail "60 ticks with the lens and a warden stored no souls"
OPERATIONS=$(python3 -c "print(round((500.0 - float('$WARDEN')) / 4))")
[ "$OPERATIONS" -ge 1 ] || fail "the warden was never hurt (health $WARDEN)"
[ "$SOULS" = "$((OPERATIONS * 2))" ] \
    || fail "the tank holds $SOULS souls for $OPERATIONS operations, expected $((OPERATIONS * 2))"
[ "$(python3 -c "print(1 if float('$WARDEN') < 500.0 else 0)")" = 1 ] \
    || fail "the warden is still at $WARDEN health after the operations"
echo "  the lens: $OPERATIONS operations, $SOULS souls, warden $WARDEN health"

echo "== D: no lens again, same warden, 60 more ticks: the tank does not move =="
ctl command "clear @s" >/dev/null
ctl wait 4 >/dev/null
aim_machine
ctl use-item >/dev/null || fail "the right-click with an empty hand on the machine failed"
ctl wait 8 >/dev/null
info $BASE_POS
expect "the lens the empty hand took back" "$LENS" "none"
TAKEN=$(held industrialforegoing:blue_laser_lens)
[ "$TAKEN" -ge 1 ] || fail "the empty hand did not take the lens back"
BEFORE=$SOULS
ctl wait 60 >/dev/null
info $BASE_POS
expect "the tank after 60 ticks with the lens out" "$SOULS" "$BEFORE"
echo "  the lens taken back: the tank stays at $SOULS souls for 60 ticks"

echo "== D2: the efficiency augment is seated by a click, and speed is refused =="
# Titanium's augment inventory is two slots; `canAcceptAugment` on this machine refuses Speed and
# accepts Efficiency, and `onWork` rescales the bar to `floor(20 x efficiency)` after an operation.
ctl command "give @s industrialforegoing:efficiency_addon_tier_2 1" >/dev/null
ctl wait 8 >/dev/null
ctl select industrialforegoing:efficiency_addon_tier_2 >/dev/null 2>&1 \
    || fail "the efficiency addon is not in the hotbar"
aim_machine
ctl use-item >/dev/null || fail "the right-click with the efficiency addon on the machine failed"
ctl wait 8 >/dev/null
info $BASE_POS
expect "the efficiency the seated augment reads" "$EFFICIENCY" "0.80"
expect "the addon the seat took from the hand" "$(held industrialforegoing:efficiency_addon_tier_2)" 0
# The bar rescales at the end of the next operation, with or without the lens seated (upstream's
# `onWork` runs on the bar, not on the harvest), so one bar later it is 16 and not 20.
ctl wait 20 >/dev/null
info $BASE_POS
expect "the bar the augment rescales" "$PROGRESS_MAX" 16
echo "  efficiency 0.80 seated; the bar is $PROGRESS_MAX after one operation"
# A speed addon is refused: it stays in the hand and no slot takes it.
ctl command "give @s industrialforegoing:speed_addon_tier_1 1" >/dev/null
ctl wait 8 >/dev/null
ctl select industrialforegoing:speed_addon_tier_1 >/dev/null 2>&1 \
    || fail "the speed addon is not in the hotbar"
aim_machine
ctl use-item >/dev/null 2>&1 || true
ctl wait 8 >/dev/null
info $BASE_POS
expect "the efficiency after the refused speed addon" "$EFFICIENCY" "0.80"
expect "the refused speed addon is still held" "$(held industrialforegoing:speed_addon_tier_1)" 1
echo "  a speed addon is refused and stays in the hand"

echo "== D3: the shortened bar (16 ticks) ten times over one 160-tick window =="
ctl command "give @s industrialforegoing:blue_laser_lens 1" >/dev/null
ctl wait 8 >/dev/null
ctl select industrialforegoing:blue_laser_lens >/dev/null 2>&1 || fail "the lens is not in the hotbar"
aim_machine
ctl use-item >/dev/null || fail "the right-click with the lens on the machine failed"
ctl wait 8 >/dev/null
info $BASE_POS
expect "the lens back in with the augment seated" "$LENS" "industrialforegoing:blue_laser_lens"
ctl wait 24 >/dev/null
info $BASE_POS
expect "the bar still the augmented one" "$PROGRESS_MAX" 16
SOULS_BEFORE=$SOULS
ctl wait 160 >/dev/null
info $BASE_POS
GAINED=$((SOULS - SOULS_BEFORE))
# The two readouts cost a couple of ticks each, so the window is ~170 ticks: ten or eleven 16-tick
# operations. The plain 20-tick bar fits eight in the same window (16 souls), so the bound is what
# separates the two rates; the unit test pins the exact 10-in-160.
if [ "$SELFTEST" = augment ]; then
    echo "(selftest: asserting the plain bar's rate)"
    [ "$GAINED" -le 16 ] \
        || fail "selftest: the shortened bar gained $GAINED souls, expected no more than the plain bar's 16"
fi
[ "$GAINED" -ge 20 ] \
    || fail "the shortened bar stored $GAINED souls in the window, less than ten 16-tick operations (the plain bar fits eight)"
echo "  efficiency 0.80: bar $PROGRESS_MAX, $GAINED souls in the window (the plain bar fits 8 operations)"

echo "== E: the pipe pulls 4 a tick into a network of 4 souls a pipe =="
# The tank must not move on its own while the pipe is measured: the lens comes back out (D3 left it
# in, and a seated lens keeps harvesting a warden every 16 ticks).
ctl command "clear @s" >/dev/null
ctl wait 4 >/dev/null
aim_machine
ctl use-item >/dev/null || fail "the right-click with an empty hand on the machine failed"
ctl wait 8 >/dev/null
info $BASE_POS
expect "the lens out again for the pipe measurement" "$LENS" "none"
say "industrialforegoingsouls souls $BASE_POS 50"
ctl wait 6 >/dev/null
info $BASE_POS
expect "the seeded tank" "$SOULS" 50
ctl command "execute in minecraft:the_end run setblock $((X0 + 1)) 101 0 industrialforegoingsouls:soul_network_pipe" >/dev/null
ctl wait 12 >/dev/null
info $BASE_POS
PIPE_SOULS=$SOULS
# One pipe is a network of four souls, so it takes exactly four out of the machine and stops: the
# pull is one transfer a tick, and the network has nowhere else to put them.
expect "the machine after the first pipe pulled" "$PIPE_SOULS" 46
info $((X0 + 1)) 101 0
expect "the readout's block" "$KIND" "pipe"
expect "the one-pipe network" "$NETWORK" 4
expect "the one-pipe capacity" "$NET_CAP" 4
ctl command "execute in minecraft:the_end run setblock $((X0 + 2)) 101 0 industrialforegoingsouls:soul_network_pipe" >/dev/null
ctl wait 20 >/dev/null
info $((X0 + 1)) 101 0
expect "the two-pipe capacity" "$NET_CAP" 8
expect "the two-pipe network" "$NETWORK" 8
info $BASE_POS
expect "the machine after the second pipe pulled" "$SOULS" 42
echo "  the machine at $PIPE_SOULS souls after one pipe, 42 after two; the network 4/4 then 8/8"

echo "== F: a placed surge takes its facing from the clicked face =="
# A full block under the click, so the ray hits a shape the harness can name: the surge goes on its
# top face and takes `facing=up`, which is `state_for_placement`'s whole rule.
ctl command "execute in minecraft:the_end run setblock 198 101 2 minecraft:stone" >/dev/null
ctl command "execute in minecraft:the_end run tp @s 196.5 101 2.5" >/dev/null
ctl wait 6 >/dev/null
ctl command "item replace entity @s hotbar.1 with industrialforegoingsouls:soul_surge 1" >/dev/null
ctl wait 4 >/dev/null
ctl select 2 >/dev/null
if ! AIM=$(ctl look-at-block 198 101 2 up 2>&1); then fail "cannot aim at the stone top: $AIM"; fi
ctl wait 8 >/dev/null
if ! PLACED=$(ctl place 198 102 2 --item industrialforegoingsouls:soul_surge 2>&1); then
    fail "the surge did not place on the stone: $PLACED"
fi
ctl wait 8 >/dev/null
expect "the placed surge's block" "$(block_name 198 102 2)" "industrialforegoingsouls:soul_surge"
expect "the facing a top-face placement takes" "$(block_prop 198 102 2 facing)" "up"
info 198 102 2
expect "the placed surge's facing" "$FACING" "up"
expect "the placed surge's network (no pipe)" "$NETWORK" 0
echo "  a top-face placement gives facing=up, and the readout agrees"
ctl command "execute in minecraft:the_end run setblock 198 102 2 minecraft:air" >/dev/null
ctl wait 6 >/dev/null

echo "== G: the surge spends a soul on 300 ticks and random-ticks the block it is mounted on =="
# Nothing grows on its own: every random tick the wheat gets below came from the surge.
REPLY=$(ctl command "gamerule random_tick_speed 0" 2>&1 || true)
case "$REPLY" in
    *random_tick_speed*|*"has been set"*) ;;
    *) fail "gamerule random_tick_speed 0 was refused: $REPLY" ;;
esac
ctl command "execute in minecraft:the_end run fill $((AX - 1)) 101 -2 $((AX + 1)) 101 1 minecraft:stone" >/dev/null
ctl command "execute in minecraft:the_end run fill $((BX - 1)) 101 -2 $((BX + 1)) 101 1 minecraft:stone" >/dev/null
for x in "$AX" "$BX"; do
    ctl command "execute in minecraft:the_end run setblock $x 101 0 minecraft:farmland[moisture=7]" >/dev/null
    ctl command "execute in minecraft:the_end run setblock $x 102 0 minecraft:wheat[age=0]" >/dev/null
    ctl command "execute in minecraft:the_end run setblock $((x - 1)) 102 0 minecraft:glowstone" >/dev/null
    ctl command "execute in minecraft:the_end run setblock $((x + 1)) 102 0 minecraft:glowstone" >/dev/null
done
# A crop's own shape is two pixels tall at age 0, so the harness cannot click its side; the surge
# goes in with its state (the placement rule itself is proven in phase F).
ctl command "execute in minecraft:the_end run setblock $AX 102 -1 industrialforegoingsouls:soul_surge[facing=north]" >/dev/null
ctl command "execute in minecraft:the_end run setblock $AX 102 -2 industrialforegoingsouls:soul_network_pipe" >/dev/null
ctl wait 10 >/dev/null
expect "the accelerated wheat's age" "$(block_prop "$AX" 102 0 age)" 0
expect "the control wheat's age" "$(block_prop "$BX" 102 0 age)" 0
expect "the surge's facing" "$(block_prop "$AX" 102 -1 facing)" "north"
say "industrialforegoingsouls souls $AX 102 -2 8"
ctl wait 6 >/dev/null
info "$AX" 102 -1
expect "the readout's block" "$KIND" "surge"
expect "the surge's facing in the readout" "$FACING" "north"
expect "the surge powered" "$POWERED" "false"
expect "the surge's network" "$NETWORK" 3
expect "the surge's network capacity (one pipe)" "$NET_CAP" 4
[ "$WAIT" -lt 300 ] || fail "the surge bought no accelerated ticks from the network: wait $WAIT"
say "industrialforegoingsouls wait $AX 102 -1 300"
# 1,200 game ticks, not 400: the surge's tick lands roughly one game tick in four (measured: 95
# decrements per 414 ticks), so a 400-tick window gives ~95 surge ticks and a 3% roll leaves a
# **6.4%** chance that nothing is accelerated at all — the 0.2.3 run hit exactly that window and the
# wheat stayed at age 0. 1,200 ticks is ~275 surge ticks, where the chance of no roll is ~0.02%.
ctl wait 1200 >/dev/null
info "$AX" 102 -1
[ "$WAIT" -lt 300 ] || fail "the surge's 300 ticks did not fall under acceleration: $WAIT"
AGE_A=$(block_prop "$AX" 102 0 age)
AGE_B=$(block_prop "$BX" 102 0 age)
echo "  after 1200 ticks: accelerated wheat age $AGE_A, control age $AGE_B, surge wait $WAIT/300"
[ "$AGE_A" -gt 0 ] || fail "the surged wheat never grew (age $AGE_A)"
if [ "$SELFTEST" = grow ]; then
    echo "(selftest: asserting the control wheat grew)"
    [ "$AGE_B" -gt 0 ] || fail "selftest: the control wheat's age is $AGE_B, expected it above 0"
fi
expect "the control wheat's age (random_tick_speed 0)" "$AGE_B" 0

echo "== H: the surge stops while it is powered =="
ctl command "execute in minecraft:the_end run setblock $((AX + 1)) 102 -1 minecraft:redstone_block" >/dev/null
ctl wait 10 >/dev/null
info "$AX" 102 -1
expect "the surge powered by the redstone block" "$POWERED" "true"
WAIT_BEFORE=$WAIT
ctl wait 30 >/dev/null
info "$AX" 102 -1
expect "the surge's ticks while powered" "$WAIT" "$WAIT_BEFORE"
ctl command "execute in minecraft:the_end run setblock $((AX + 1)) 102 -1 minecraft:air" >/dev/null
ctl wait 10 >/dev/null
info "$AX" 102 -1
expect "the surge powered again after the redstone is gone" "$POWERED" "false"
echo "  powered: wait frozen at $WAIT; unpowered again"

echo "== R: waterlogging on the pipe and the surge =="
# Both upstream blocks are `SimpleWaterloggedBlock`s: a water bucket fills the state, an empty one
# takes the water back, and the machine works either way. This port's interface cannot carry the
# engine's bucket path (a content block's interfaces are its template's), so the click is the block's
# own, exactly as the lens and the augment seat (`src/lib.rs` `waterlog_click`). The engine still
# reads the water from the state: `state_fluid_state` answers water for a `waterlogged=true` state.
#
# The differential is the network: a **waterlogged** pipe still pulls the laser base's souls, and the
# hand trades its bucket, which a state-only port (no interaction) could not do.
WX=208
BX=207
WZ=3
# The surge corner: a stone target to the surge's south, one pipe for its network.
TX=204
TZ=-6
ctl command "difficulty peaceful" >/dev/null
ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
ctl command "gamemode survival @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 8 >/dev/null
# The base goes in empty and the pipe beside it, so the pipe can be read dry and filled **before** the
# tank is seeded: one with souls would be pulled from in the same tick the pipe is placed.
ctl command "execute in minecraft:the_end run setblock $BX 101 $WZ industrialforegoingsouls:soul_laser_base" >/dev/null
ctl command "execute in minecraft:the_end run setblock $WX 101 $WZ industrialforegoingsouls:soul_network_pipe" >/dev/null
ctl wait 8 >/dev/null
expect "the waterlogging pipe's block" "$(block_name $WX 101 $WZ)" "industrialforegoingsouls:soul_network_pipe"
expect "the machine the pipe will pull from" "$(block_name $BX 101 $WZ)" "industrialforegoingsouls:soul_laser_base"
# The counterfactual first: a fresh pipe is dry.
expect "a fresh pipe's waterlogged state" "$(block_prop $WX 101 $WZ waterlogged)" "false"
info $WX 101 $WZ
expect "the dry pipe's readout" "$WATERLOGGED" "false"
expect "the one-pipe network" "$NETWORK" 0
expect "the one-pipe capacity" "$NET_CAP" 4
echo "  a fresh pipe is dry (waterlogged false), network 0/4"

# A full bucket fills it: `SimpleWaterloggedBlock.canPlaceLiquid` on a dry block.
ctl command "give @s minecraft:water_bucket 1" >/dev/null
ctl wait 6 >/dev/null
ctl select minecraft:water_bucket >/dev/null 2>&1 || fail "the water bucket is not in the hotbar"
ctl command "execute in minecraft:the_end run tp @s $WX.5 101 $((WZ - 2)).5" >/dev/null
ctl wait 6 >/dev/null
ctl look-at-block $WX 101 $WZ north >/dev/null 2>&1 || fail "cannot aim at the pipe at $WX,101,$WZ"
ctl wait 6 >/dev/null
ctl use-item >/dev/null || fail "the right-click with the water bucket on the pipe failed"
ctl wait 6 >/dev/null
expect "the waterlogged pipe's state" "$(block_prop $WX 101 $WZ waterlogged)" "true"
info $WX 101 $WZ
expect "the waterlogged pipe's readout" "$WATERLOGGED" "true"
if [ "$SELFTEST" = waterlog ]; then
    echo "(selftest: asserting the bucket left the pipe dry)"
    expect "selftest: the pipe's waterlogged state" "$WATERLOGGED" "false"
fi
echo "  hand after the pour: $(ctl inventory | jq -c '.counts')" >&2
expect "the empty bucket the pour left in the hand" "$(held minecraft:bucket)" 1
expect "the water bucket the pour used up" "$(held minecraft:water_bucket)" 0
echo "  a water bucket: waterlogged true, the hand now holds the empty bucket"

# The network still moves souls through the waterlogged pipe: 50 seeded, 4 in the pipe, 46 left. The
# seed's own reply ("souls 50/1350.") is in the log above the readback.
say "industrialforegoingsouls souls $BX 101 $WZ 50"
ctl wait 16 >/dev/null
info $BX 101 $WZ
expect "the machine after a waterlogged pipe pulled" "$SOULS" 46
info $WX 101 $WZ
expect "the waterlogged pipe's network" "$NETWORK" 4
expect "the pipe is still waterlogged after the transfer" "$WATERLOGGED" "true"
echo "  the waterlogged pipe pulled the seeded 50 down to $SOULS into a 4/4 network"

# An empty bucket takes the water back out: `BucketPickup.pickupBlock`.
ctl command "clear @s" >/dev/null
ctl wait 4 >/dev/null
ctl command "give @s minecraft:bucket 1" >/dev/null
ctl wait 6 >/dev/null
ctl select minecraft:bucket >/dev/null 2>&1 || fail "the empty bucket is not in the hotbar"
ctl command "execute in minecraft:the_end run tp @s $WX.5 101 $((WZ - 2)).5" >/dev/null
ctl wait 6 >/dev/null
ctl look-at-block $WX 101 $WZ north >/dev/null 2>&1 || fail "cannot aim at the pipe for the drain"
ctl wait 6 >/dev/null
ctl use-item >/dev/null || fail "the right-click with the empty bucket on the pipe failed"
ctl wait 6 >/dev/null
expect "the drained pipe's state" "$(block_prop $WX 101 $WZ waterlogged)" "false"
info $WX 101 $WZ
expect "the drained pipe's readout" "$WATERLOGGED" "false"
echo "  hand after the drain: $(ctl inventory | jq -c '.counts')" >&2
expect "the water bucket the drain gave back" "$(held minecraft:water_bucket)" 1
expect "the empty bucket the drain used up" "$(held minecraft:bucket)" 0
echo "  an empty bucket: waterlogged false, the water bucket back in the inventory"

# The surge: a state written with the property reads back, and it still spends a soul.
ctl command "execute in minecraft:the_end run setblock $TX 101 $((TZ + 1)) minecraft:stone" >/dev/null
ctl command "execute in minecraft:the_end run setblock $TX 101 $TZ industrialforegoingsouls:soul_surge[facing=north,waterlogged=true]" >/dev/null
ctl command "execute in minecraft:the_end run setblock $((TX + 1)) 101 $TZ industrialforegoingsouls:soul_network_pipe" >/dev/null
ctl wait 10 >/dev/null
expect "the surge's waterlogged state read back" "$(block_prop $TX 101 $TZ waterlogged)" "true"
expect "the surge's facing read back" "$(block_prop $TX 101 $TZ facing)" "north"
# A surge spends its first soul on the tick it sees one, so the network reads 3 of the 4 seeded and
# the wait is already falling: that is the soul a waterlogged surge spent (phase G's own shape).
say "industrialforegoingsouls souls $((TX + 1)) 101 $TZ 4"
ctl wait 6 >/dev/null
info $TX 101 $TZ
expect "the waterlogged surge's readout" "$WATERLOGGED" "true"
expect "the soul a waterlogged surge spent of the 4 seeded" "$NETWORK" 3
[ "$WAIT" -lt 300 ] || fail "the waterlogged surge bought no accelerated ticks: wait $WAIT"
WAIT_BEFORE=$WAIT
ctl wait 20 >/dev/null
info $TX 101 $TZ
[ "$WAIT" -lt "$WAIT_BEFORE" ] \
    || fail "the waterlogged surge's ticks stopped at $WAIT (was $WAIT_BEFORE)"
echo "  a waterlogged surge reads its state back, spends a soul and runs ($WAIT_BEFORE -> $WAIT/300)"

# The counterfactual for the surge state: the property is the state's, not the model's.
ctl command "execute in minecraft:the_end run setblock $((TX - 2)) 101 $TZ industrialforegoingsouls:soul_surge[facing=north,waterlogged=false]" >/dev/null
ctl wait 8 >/dev/null
expect "a dry surge's waterlogged state" "$(block_prop $((TX - 2)) 101 $TZ waterlogged)" "false"
info $((TX - 2)) 101 $TZ
expect "the dry surge's readout" "$WATERLOGGED" "false"

# The laser base is not waterloggable upstream, and this package declares no such state on it: the
# game's own parser refuses the property rather than silently taking a default state.
REFUSED=$(ctl setblock $BX 101 $WZ "industrialforegoingsouls:soul_laser_base[waterlogged=true]" 2>&1 || true)
case "$REFUSED" in
    *'"ok":true'*) fail "the laser base accepted a waterlogged state: $REFUSED" ;;
esac
expect "the laser base after the refused waterlogged state" "$(block_name $BX 101 $WZ)" "industrialforegoingsouls:soul_laser_base"
echo "  the laser base refuses a waterlogged state by the game's own parser"
# Back to creative for the crafting phase, which ran that way before this phase existed.
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 4 >/dev/null

echo "== K: the carried recipes craft, and the carry is not vacuous =="
ctl command "execute in minecraft:the_end run fill 216 100 6 222 100 12 minecraft:stone" >/dev/null
ctl command "execute in minecraft:the_end run fill 216 101 6 222 104 12 minecraft:air" >/dev/null
ctl command "execute in minecraft:the_end run setblock 219 101 10 minecraft:crafting_table" >/dev/null
ctl command "execute in minecraft:the_end run tp @s 218.5 101 10.5" >/dev/null
ctl wait 12 >/dev/null
[ "$(block_name 219 101 10)" = "minecraft:crafting_table" ] || fail "the crafting table did not place"
python3 "$PORT/tools/craft_recipe.py" "$SOCK" "$CTL" 219 101 10 \
    || fail "the carried shaped recipes did not craft"

echo "== L: each block drops itself when broken in survival =="
# Peaceful from here to the end of the breaking phases: an Enderman that wandered into the arena
# killed the player mid-break on 2026-11-22, which is a harness accident, not a port failure.
ctl command "difficulty peaceful" >/dev/null
ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
ctl command "gamemode survival @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl command "item replace entity @s hotbar.0 with minecraft:iron_pickaxe 1" >/dev/null
ctl wait 6 >/dev/null
ctl select 1 >/dev/null
# The pipe goes on a pedestal at eye level: its shape is the engine's chorus-plant cross, a six-pixel
# column, and a floor-level one is missed by a crosshair that has to drop 1.1 blocks over 1.5.
ctl command "execute in minecraft:the_end run setblock 191 101 -8 minecraft:stone" >/dev/null
ctl command "execute in minecraft:the_end run setblock 191 102 -8 industrialforegoingsouls:soul_network_pipe" >/dev/null
ctl wait 8 >/dev/null
expect "the pedestal pipe" "$(block_name 191 102 -8)" "industrialforegoingsouls:soul_network_pipe"
for spec in "soul_laser_base $X0 101 0 2" "soul_network_pipe 191 102 -8 2" "soul_surge $AX 102 -1 2"; do
    set -- $spec
    local_id="industrialforegoingsouls:$1"
    x=$2; y=$3; z=$4; back=$5
    ctl command "execute in minecraft:the_end run tp @s $((x - back)).5 $y $z.5" >/dev/null
    ctl wait 8 >/dev/null
    if ! BROKEN=$(ctl break "$x" "$y" "$z" 2>&1); then
        fail "breaking $local_id failed: $BROKEN"
    fi
    echo "  break $local_id: $(echo "$BROKEN" | jq -c '{block, drops}')"
    [ "$(echo "$BROKEN" | jq -r --arg id "$local_id" '.drops[$id] // 0')" -ge 1 ] \
        || fail "a broken $local_id dropped nothing: $(echo "$BROKEN" | jq -c '.drops')"
done
echo "  the three blocks drop themselves"

echo "== P: a broken machine carries its contents on its own item =="
# The gap 0.2.1 recorded: a Soul Laser Base used to lose its seated lens and efficiency addon when
# a player broke it. `block_drops` writes a fresh `minecraft:block_entity_data` payload on the drop
# and the item's placement hook merges it back, clamped. An empty machine's item stays plain.
RX=$((X0 - 4))
EX=$((X0 - 4))
PX=$((X0 - 1))
ctl command "gamemode survival @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
ctl wait 20 >/dev/null

ctl command "execute in minecraft:the_end run setblock $RX 101 9 industrialforegoingsouls:soul_laser_base" >/dev/null
ctl wait 8 >/dev/null
[ "$(block_name "$RX" 101 9)" = "industrialforegoingsouls:soul_laser_base" ] \
    || fail "the carrying machine did not place at $RX,101,9"
# The lens and the augment go in by click, as phases C and D2 seat them.
ctl command "give @s industrialforegoing:blue_laser_lens 1" >/dev/null
ctl wait 6 >/dev/null
ctl select industrialforegoing:blue_laser_lens >/dev/null 2>&1 || fail "the lens is not in the hotbar"
aim_block "$RX" 101 9
ctl use-item >/dev/null || fail "the right-click with the lens on the carrying machine failed"
ctl wait 6 >/dev/null
ctl command "give @s industrialforegoing:efficiency_addon_tier_2 1" >/dev/null
ctl wait 6 >/dev/null
ctl select industrialforegoing:efficiency_addon_tier_2 >/dev/null 2>&1 \
    || fail "the efficiency addon is not in the hotbar"
aim_block "$RX" 101 9
ctl use-item >/dev/null || fail "the right-click with the efficiency addon failed"
ctl wait 6 >/dev/null
say "industrialforegoingsouls souls $RX 101 9 30"
# One plain bar later the augment has rescaled the bar to 16; no warden is in range (they were
# killed above), so the tank keeps the seeded 30.
ctl wait 26 >/dev/null
info "$RX" 101 9
expect "the bar the augment rescales before the break" "$PROGRESS_MAX" 16
expect "the tank before the break" "$SOULS" 30
expect "the lens before the break" "$LENS" "industrialforegoing:blue_laser_lens"
expect "the augment before the break" "$EFFICIENCY" "0.80"
KEPT_SOULS=$SOULS
KEPT_PROGRESS=$PROGRESS
echo "  before the break: souls $SOULS, progress $PROGRESS/$PROGRESS_MAX, lens $LENS, efficiency $EFFICIENCY"
# The machine's own contents must not spill as items: the lens and the augment travel on the drop.
ctl command "clear @s" >/dev/null
ctl wait 4 >/dev/null
break_and_collect "$RX" 101 9 industrialforegoingsouls:soul_laser_base
CARRY=$(carry_line)
[ -n "$CARRY" ] || fail "no /data get reply for the picked-up machine: $(ctl messages 8 | jq -c '.chat')"
echo "  $CARRY" >&2
eval "$(python3 "$PORT/tools/parse_carry.py" "$CARRY" industrialforegoingsouls:soul_laser_base \
    "$KEPT_SOULS" 16 industrialforegoing:blue_laser_lens 0.8)" \
    || fail "the broken machine's item does not carry its contents: $CARRY"
[ "${ok:-0}" = 1 ] || fail "the carried payload did not parse: $CARRY"
expect "the lens the break did not spill" "$(held industrialforegoing:blue_laser_lens)" 0
expect "the addon the break did not spill" "$(held industrialforegoing:efficiency_addon_tier_2)" 0
echo "  the drop carries souls $souls, progress $progress/$bar, lens $lens, efficiency $efficiency"
# … and placing it back puts the machine together again.
place_kept "$RX" 101 9 industrialforegoingsouls:soul_laser_base
info "$RX" 101 9
if [ "$SELFTEST" = carry ]; then
    echo "(selftest: asserting the restored machine kept nothing)"
    expect "selftest: the restored tank" "$SOULS" 0
    expect "selftest: the restored lens" "$LENS" "none"
fi
expect "the restored tank" "$SOULS" "$KEPT_SOULS"
expect "the restored bar" "$PROGRESS_MAX" 16
expect "the restored lens" "$LENS" "industrialforegoing:blue_laser_lens"
expect "the restored augment" "$EFFICIENCY" "0.80"
[ "$PROGRESS" -ge 1 ] && [ "$PROGRESS" -le 16 ] \
    || fail "the restored progress is $PROGRESS, outside the 16-tick bar (a plain machine reads 20)"
echo "  placed back: souls $SOULS, progress $PROGRESS/16, lens $LENS, efficiency $EFFICIENCY"

# The counterfactual: a machine holding nothing drops a plain item, and places as a fresh one.
ctl command "execute in minecraft:the_end run setblock $EX 101 11 industrialforegoingsouls:soul_laser_base" >/dev/null
ctl wait 8 >/dev/null
info "$EX" 101 11
expect "the empty machine's tank" "$SOULS" 0
expect "the empty machine's lens" "$LENS" "none"
ctl command "clear @s" >/dev/null
ctl wait 4 >/dev/null
break_and_collect "$EX" 101 11 industrialforegoingsouls:soul_laser_base
CARRY=$(carry_line)
[ -n "$CARRY" ] || fail "no /data get reply for the picked-up empty machine"
echo "  $CARRY" >&2
eval "$(python3 "$PORT/tools/parse_carry.py" "$CARRY" none)" \
    || fail "an empty machine's item carries a payload: $CARRY"
[ "${ok:-0}" = 1 ] || fail "the empty drop did not parse: $CARRY"
place_kept "$EX" 101 11 industrialforegoingsouls:soul_laser_base
info "$EX" 101 11
expect "the empty machine's tank after its break" "$SOULS" 0
expect "the empty machine's lens after its break" "$LENS" "none"
expect "the empty machine's bar after its break" "$PROGRESS_MAX" 20
expect "the empty machine's augment" "$EFFICIENCY" "1.00"
echo "  an empty machine's item: no payload, and it places as a fresh machine"

# The same rule for the other two machines: a pipe keeps its own souls and a surge its bought ticks.
ctl command "execute in minecraft:the_end run setblock $PX 101 9 industrialforegoingsouls:soul_network_pipe" >/dev/null
ctl wait 10 >/dev/null
say "industrialforegoingsouls souls $PX 101 9 3"
ctl wait 6 >/dev/null
info "$PX" 101 9
expect "the pipe's souls before the break" "$NETWORK" 3
ctl command "clear @s" >/dev/null
ctl wait 4 >/dev/null
break_and_collect "$PX" 101 9 industrialforegoingsouls:soul_network_pipe
CARRY=$(carry_line)
echo "  $CARRY" >&2
eval "$(python3 "$PORT/tools/parse_carry.py" "$CARRY" industrialforegoingsouls:soul_network_pipe 3)" \
    || fail "the broken pipe's item does not carry its souls: $CARRY"
[ "${ok:-0}" = 1 ] || fail "the pipe's payload did not parse: $CARRY"
place_kept "$PX" 101 9 industrialforegoingsouls:soul_network_pipe
info "$PX" 101 9
expect "the pipe's souls after its break" "$NETWORK" 3
echo "  a broken pipe's item carries its $souls souls back"

ctl command "execute in minecraft:the_end run setblock $PX 101 12 industrialforegoingsouls:soul_surge[facing=north]" >/dev/null
ctl wait 10 >/dev/null
say "industrialforegoingsouls wait $PX 101 12 120"
ctl wait 6 >/dev/null
info "$PX" 101 12
expect "the surge's bought ticks before the break" "$WAIT" 120
ctl command "clear @s" >/dev/null
ctl wait 4 >/dev/null
break_and_collect "$PX" 101 12 industrialforegoingsouls:soul_surge
CARRY=$(carry_line)
echo "  $CARRY" >&2
eval "$(python3 "$PORT/tools/parse_carry.py" "$CARRY" industrialforegoingsouls:soul_surge - - - - 120)" \
    || fail "the broken surge's item does not carry its ticks: $CARRY"
[ "${ok:-0}" = 1 ] || fail "the surge's payload did not parse: $CARRY"
# A pedestal of one of the mod's own surges: a surge is in both `cant_accelerate` tags, so the
# restored one above it aims at a block that may not be accelerated and keeps every tick it carried
# (`a_surge_facing_nothing_keeps_its_soul`).
ctl command "execute in minecraft:the_end run setblock $PX 101 14 industrialforegoingsouls:soul_surge[facing=north]" >/dev/null
ctl wait 6 >/dev/null
place_kept "$PX" 102 14 industrialforegoingsouls:soul_surge
info "$PX" 102 14
expect "the restored surge's facing" "$FACING" up
expect "the surge's ticks after its break" "$WAIT" 120
echo "  a broken surge's item carries its $wait bought ticks back"

echo "== Q: a second lens replaces the first, and the first comes back =="
# Upstream's one catalyst slot swaps the old lens into the player's hand; the 0.2.1 port overtook
# the saved value and ate the new lens, losing the old one with it. The click must leave exactly
# one lens in the machine and exactly one back in the player's inventory.
QX=$((X0 - 4))
QZ=13
ctl command "execute in minecraft:the_end run setblock $QX 101 $QZ industrialforegoingsouls:soul_laser_base" >/dev/null
ctl wait 8 >/dev/null
[ "$(block_name "$QX" 101 "$QZ")" = "industrialforegoingsouls:soul_laser_base" ] \
    || fail "the lens-swap machine did not place at $QX,101,$QZ"
ctl command "clear @s" >/dev/null
ctl wait 4 >/dev/null
ctl command "give @s industrialforegoing:blue_laser_lens 1" >/dev/null
ctl wait 6 >/dev/null
ctl select industrialforegoing:blue_laser_lens >/dev/null 2>&1 || fail "the first lens is not in the hotbar"
aim_block "$QX" 101 "$QZ"
ctl use-item >/dev/null || fail "the first lens did not seat on the swap machine"
ctl wait 6 >/dev/null
info "$QX" 101 "$QZ"
expect "the first lens seated" "$LENS" "industrialforegoing:blue_laser_lens"
expect "the first lens taken from the hand" "$(held industrialforegoing:blue_laser_lens)" 0
ctl command "give @s industrialforegoing:blue_laser_lens 1" >/dev/null
ctl wait 6 >/dev/null
ctl select industrialforegoing:blue_laser_lens >/dev/null 2>&1 || fail "the second lens is not in the hotbar"
aim_block "$QX" 101 "$QZ"
ctl use-item >/dev/null || fail "the second lens's click failed"
ctl wait 6 >/dev/null
info "$QX" 101 "$QZ"
if [ "$SELFTEST" = replace ]; then
    echo "(selftest: asserting the second lens voided the first)"
    expect "selftest: the lens the swap returned" "$(held industrialforegoing:blue_laser_lens)" 0
fi
expect "the lens the swap returned" "$(held industrialforegoing:blue_laser_lens)" 1
expect "the lens still seated after the swap" "$LENS" "industrialforegoing:blue_laser_lens"
echo "  a second lens: the machine keeps one, the first is back in the inventory (held 1)"

echo "== N: the mod's readout refuses a block that is not its own =="
REFUSAL=$(ctl chat "/industrialforegoingsouls info 190 101 0" >/dev/null 2>&1; ctl wait 8 >/dev/null; ctl messages 8 | jq -r '.chat[]' | grep -m1 'not an industrialforegoingsouls block' || true)
case "$REFUSAL" in
    *"not an industrialforegoingsouls block"*) ;;
    *) fail "the readout did not refuse a vanilla position: '$REFUSAL'" ;;
esac
echo "  a vanilla position is refused by name"

echo "== M2: the parent's ore base runs this package's laser_drill_ore recipe =="
# The base and the drill are the parent package's blocks; what this package adds is the recipe the
# base's table resolves and the `deep_dark` biome tag it filters on. A one-mod game cannot host the
# package at all (the lens is the parent's), so this leg is the two-mod game `e2e-two-mod.sh` builds.
LX=200
LY=101
LZ=6
ctl command "gamemode creative @s" >/dev/null
ctl command "difficulty peaceful" >/dev/null
ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
ctl wait 8 >/dev/null
place_any $LX $LY $LZ industrialforegoing:ore_laser_base
place_drill_above $LX $LY $LZ
laser_info $LX $((LY + 1)) $LZ drill
expect "the drill's target (the base below it)" "$LD_TARGET" "[$LX, $LY, $LZ]"
expect "the drill's facing (placed while looking down)" "$LD_FACING" up
laser_info $LX $LY $LZ base
expect "the base's own table with this package's recipe" "$LA_RULES" 11
expect "the recipe files the game kept" "$LA_FILES" 11
expect "the ore in the End's own biome" "$LA_RECIPE" none
echo "  $LA_RULES/$LA_FILES recipes; no match in $LA_BIOME"
# The recipe's rarity whitelists `industrialforegoingsouls:deep_dark`, the tag this package ships,
# over a depth band; `fillbiome` makes the box deep dark and the depth setter picks the band.
ctl command "execute in minecraft:the_end run fillbiome $((LX - 8)) 96 $((LZ - 8)) $((LX + 8)) 112 $((LZ + 8)) minecraft:deep_dark" >/dev/null
lasay $LX $LY $LZ depth 0
ctl wait 8 >/dev/null
laser_info $LX $LY $LZ base
expect "the biome this package's tag names" "$LA_BIOME" minecraft:deep_dark
expect "the depth the rarity reads" "$LA_DEPTH" 0
expect "this package's recipe, resolved by the parent's own type" "$LA_RECIPE" "industrialforegoingsouls:laser_drill_ore/echo_shard"
expect "its weight" "$LA_WEIGHT" 1
echo "  $LA_BIOME at depth $LA_DEPTH: $LA_RECIPE, weight $LA_WEIGHT"
# One completed base bar, charged by one completed drill bar: 1,000 FE and the recipe's ore.
lasay $LX $LY $LZ progress 99
lasay $LX $((LY + 1)) $LZ energy 10000
lasay $LX $((LY + 1)) $LZ progress 1
ctl wait 10 >/dev/null
laser_info $LX $((LY + 1)) $LZ drill
expect "the charge the drill spent" "$LD_ENERGY" 9000
ORY=$(base_slot $LX $LY $LZ 6)
laser_info $LX $LY $LZ base
if [ "$SELFTEST" = laser ]; then
    echo "(selftest: asserting this package's ore did not appear)"
    [ "$ORY" = "empty" ] || fail "selftest: the base yielded $ORY, expected an empty output slot"
fi
expect "this package's ore in the base's output" "$ORY" "minecraft:echo_shard x1"
echo "  one base bar for the drill's 1,000 FE yielded $ORY"

echo "PASS: the three blocks register, place and read back and an unknown id is refused; the Soul Laser Base makes no souls in 60 ticks with a warden in range and no lens, and with the blue lens seated it turns every 20 ticks into 4 damage on the warden and 2 souls in a 1,350 tank, and taking the lens back freezes the tank; a seated efficiency addon of 0.80 rescales the bar to 16 after one operation and ten of them fit a 160-tick window where the plain bar fits eight, a speed addon is refused and stays in the hand, and the seated addon comes back out on an empty hand; the Soul Pipe pulls 4 souls a tick out of the machine into a network of 4 souls a pipe (4/4 with one, 8/8 with two); the Soul Surge on the wheat's north face spends one soul, burns 300 accelerated ticks, random-ticks the block it is mounted on until a wheat grows where random_tick_speed 0 means the control wheat cannot, and stops while powered; the three carried recipes craft from the parent's tags and this port's c:stones with the ingredients consumed and the grid empty, a grid of vanilla stand-ins crafts nothing, and each block drops itself when broken; a Soul Laser Base broken in survival with a blue lens and an efficiency addon inside drops an item whose minecraft:block_entity_data carries the tank, the bar, the lens and the augment, and placing it back restores souls 30, a 16-tick bar, lens industrialforegoing:blue_laser_lens and augments efficiency 0.80, while an empty machine's item carries no payload and places as a fresh 20-tick machine; a broken pipe's item carries its 3 souls and a broken surge's its 120 bought ticks back; a second blue lens swaps with the seated one, which comes back to the player's inventory instead of being voided; and the parent's ore laser base reads an 11-recipe table, resolves this package's echo-shard recipe only in the deep_dark this package's tag names, and produces one minecraft:echo_shard from one completed bar of the drill's 1,000 FE; and waterlogging is the state's: a fresh pipe reads waterlogged false, a water bucket fills it to true and leaves the empty bucket in the hand, the waterlogged pipe still pulls the laser base 50 -> 46 into a 4/4 network, an empty bucket drains it back to false and hands the water bucket over, a surge set to facing=north,waterlogged=true reads both properties back and still spends a soul on 300 ticks where a dry one reads false, and the laser base, which upstream does not waterlog, refuses the property by the game's own parser"
