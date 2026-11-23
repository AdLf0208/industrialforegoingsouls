# Validation — `industrialforegoingsouls` 0.2.4

The ladder actually run for the delivered package, cheapest first. Every compile went through
`ports/tools/build-slot.sh` (`CARGO_INCREMENTAL=0`, no `--target-dir`); `check`, both `fits` runs and
`cargo horizon-mod build` are non-compiling package checks and ran locally (`BUILD_LOCAL=1`) because
the remote box has no `cargo-horizon-mod`. The in-game legs are launcher builds at the campaign pin
**`0a875da91dbb0fdcb7ba8c42c11341aa229af51a`** + mixins `e59da909…`, of a game carrying **this port
and its parent** (`industrialforegoing` 0.2.10 in this run).

This is the **menu-and-screen** build: the Soul Laser Base declares a container of three slots (the
catalyst and Titanium's two augment slots) and a menu type of its own, opened by a plain right-click,
with a screen that draws the tank and the bar and three data slots that carry them.
`SLICE-PLAN-0.2.4.md` is the plan and the pin's API measurements; `notes.md` §1.5, §4.4 and §7
record the design, the four in-game defects it found, and the same-item click quirk.

Evidence: `ports/_shared/evidence/industrialforegoingsouls/` and
`ports/_shared/evidence/industrialforegoingsouls-e2e/` (0.2.0 logs keep a `-0.2.0` suffix, 0.2.1 a
`-0.2.1` one; the unsuffixed logs are this run).

| step | command | result | evidence |
|---|---|---|---|
| red (menu rules) | `cargo test` with the eight new rules at `todo!()` | **38 passed, 8 failed** (`not yet implemented: slice 0.2.4`) | `industrialforegoingsouls/sdk-test-red-0.2.4.log` |
| tests | `cargo test --offline` | **46 passed, 0 failed** | `industrialforegoingsouls/sdk-test.log` |
| format | `cargo fmt --check` | clean (`FMT-CLEAN`) | `industrialforegoingsouls/sdk-clippy-0.2.4.log` |
| lints | `cargo clippy --all-targets -- -D warnings` | `CLIPPY-EXIT:0`, 0 warnings | `industrialforegoingsouls/sdk-clippy-0.2.4.log` |
| content | `python3 tools/assemble_content.py --check` | red on the missing menu, then `CONTENT OK: 57 files` | — |
| package | `rm -f Cargo.lock && cargo horizon-mod build` ×2, the SDK-generated lock stripped from each archive | `CMP-IDENTICAL`, sha256 `79159d3fead5c82aec88e3526e17098e0c76511244b487147ccc046b36bda82b`, **111,572 B**, **64** entries, `PACKED-LOCK: 0` | `industrialforegoingsouls/sdk-build-0.2.4.log` |
| check | `cargo horizon-mod check industrialforegoingsouls.hmod` | `industrialforegoingsouls 0.2.4 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)`, **111,572 B**, sha256 `79159d3f…a82b`, exit 0 | `industrialforegoingsouls/sdk-check-0.2.4.log` |
| fits (pin) | `cargo horizon-mod fits industrialforegoingsouls.hmod --build 0a875da` | `0a875da  made for`, exit 0 | `industrialforegoingsouls/sdk-fits-0.2.4.log` |
| fits (other) | `… --build 0b14f8a` | `0b14f8a  fits`, exit 0 | `industrialforegoingsouls/sdk-fits-0.2.4.log` |
| package gate | `python3 ports/tools/verify-sdk-package.py industrialforegoingsouls` | **1/1** (46 tests; runtime byte-identical; every packed file is the tree's; no packed `Cargo.lock`) | — |
| launcher build | `tools/e2e-two-mod.sh build` (the three stale `native-0a875da-*` tags carrying this port and the export removed first) | `Compiling industrialforegoing (1 of 2)`, **`Compiling industrialforegoingsouls (2 of 2)`**, `the build carries: industrialforegoing 0.2.10, industrialforegoingsouls 0.2.4` | `industrialforegoingsouls/e2e-two-mod-build-0.2.4.log` |
| in-game, green | `tools/e2e.sh` on that build (A–N, D2–D3, R, S, K–L, P–Q, M2) | **`PASS` + `EXIT:0`** | `industrialforegoingsouls-e2e/e2e-launcher.log` |
| control | the same script on `GAME=$CONTROL_GAME` (same Nightly, no package) | `FAIL: the game refused industrialforegoingsouls:soul_laser_base` → `EXIT:1` | `…/e2e-launcher-control.log` |
| selftest, screen | `SELFTEST=screen` (phase S asserts the base opens **no** menu) | `FAIL: selftest: the base opened a menu (container), expected none` → `EXIT:1` | `…/e2e-launcher-selftest-screen.log` |
| selftest, lens | `SELFTEST=1` (phase B asserts the tank is not empty without a lens) | `FAIL: selftest: the soul tank is 0, expected anything but 0` → `EXIT:1` | `…/e2e-launcher-selftest.log` |
| selftest, augment | `SELFTEST=augment` (phase D3 asserts the plain bar's rate) | `FAIL: selftest: the shortened bar gained 22 souls, expected no more than the plain bar's 16` → `EXIT:1` | `…/e2e-launcher-selftest-augment.log` |
| selftest, ore | `SELFTEST=laser` (phase M2 asserts the output slot is empty) | `FAIL: selftest: the base yielded minecraft:echo_shard x1, expected an empty output slot` → `EXIT:1` | `…/e2e-launcher-selftest-laser.log` |
| selftest, growth | `SELFTEST=grow` (phase G asserts the unaccelerated wheat grew) | `FAIL: selftest: the control wheat's age is 0, expected it above 0` → `EXIT:1` | `…/e2e-launcher-selftest-grow.log` |
| selftest, carry | `SELFTEST=carry` (phase P asserts the restored machine kept nothing) | `FAIL: selftest: the restored tank is 30, expected 0` → `EXIT:1` | `…/e2e-launcher-selftest-carry.log` |
| selftest, replace | `SELFTEST=replace` (phase Q asserts the second lens voided the first) | `FAIL: selftest: the lens the swap returned is 1, expected 0` → `EXIT:1` | `…/e2e-launcher-selftest-replace.log` |
| selftest, waterlog | `SELFTEST=waterlog` (phase R asserts the bucket left the pipe dry) | `FAIL: selftest: the pipe's waterlogged state is true, expected false` → `EXIT:1` | `…/e2e-launcher-selftest-waterlog.log` |
| log gate | `python3 ports/tools/check-e2e-logs.py industrialforegoingsouls-e2e --strict` | `logs=17 ok=17 missing-exit=0 bad-exit=0` | — |
| skills gate | `python3 ports/tools/check-skills.py --notes ports/industrialforegoingsouls` | PASS | — |

Artifact: **111,572 B**, SHA-256
`79159d3fead5c82aec88e3526e17098e0c76511244b487147ccc046b36bda82b`, 64 entries, no packed
`Cargo.lock`, declared version **0.2.4**.

## What each in-game phase asserts

The phases 0.2.0–0.2.3 established are unchanged except where this slice moved their mechanism
(D and E take the lens out through the menu instead of an empty-hand click; D2 closes the menu a
refused click opens; P's payload carries `carried` instead of the old keys). In full:

| phase | assertion |
|---|---|
| launch | 0 `load-items`/`recipes`/`advancements`/`content` errors in the game's log |
| A | the three block items register and read back; an unknown item and block id are refused by the game's own message; the machine and a `NoAI` warden read back through `/industrialforegoingsouls info` |
| B | 60 ticks, a warden in range, **no lens** → `souls 0`, `warden 500.0` |
| C | seat the blue lens by right-click (one taken from the hand); 60 ticks → `4 operations`, `8 souls` = `2 × ((500 − health) / 4)`, `warden 484.0` |
| D | the lens comes out through the menu's catalyst slot (shift-click on 300) → `lens none`, the lens in the inventory; 60 more ticks → the tank does not move |
| D2 | right-click with `efficiency_addon_tier_2` → `augments efficiency 0.80` and the addon gone from the hand; one operation later the bar reads `x/16`; a speed addon's click leaves it `held 1` and the efficiency unchanged (and opens the menu, which the phase closes) |
| D3 | the lens back in: over a 160-tick window the 16-tick bar stores 20+ souls, where the plain bar fits eight operations (16) |
| E | the lens out through the menu; `souls 50` seeded; one pipe → the machine at 46 and the network 4/4; a second pipe → 8/8 and the machine at 42 |
| F | a surge placed on a full block's top face takes `facing=up`, and the readout agrees |
| G | `random_tick_speed 0`, a surge on the wheat's north face with one pipe: the network 4 → 3 → 2, `wait` 293 → 0 over 1,200 ticks, the accelerated wheat at **age 7** and the control at **age 0** |
| H | a redstone block beside the surge → `powered true` and `wait` frozen; removed → `false` and falling again |
| R | **waterlogging**: a fresh pipe reads `waterlogged=false` and `0/4`; a water-bucket click reads `true`; the waterlogged pipe pulls the seeded laser base `50 → 46` into `4/4`; an empty-bucket click reads `false` and puts the water bucket in the inventory; a surge at `[facing=north,waterlogged=true]` reads both properties back and spends a soul; a dry surge reads `false`; the laser base refuses the property by the game's own parser; `SELFTEST=waterlog` fails on `true` |
| S | **the menu**: a plain right-click opens it, `ctl screen` lists **3** container slots (301 holds D2's tier-2 addon); the lens goes in through slot 300 and the machine harvests (`souls 44 → 50`, `warden 496 → 484`); the data slots read `[50, 2, 16]` and agree with the line's tank and bar; a tier-1 efficiency addon clicked into 301 and 302 is refused by both and stays on the cursor; a vanilla barrel beside it opens **27** chest slots; `SELFTEST=screen` fails on the menu opening |
| K | the three recipes craft in a real table (8 pipes), the ingredients are consumed, the grid is empty, and a grid of vanilla stand-ins crafts nothing |
| L | each block breaks in survival with an iron pickaxe and drops itself (peaceful from here, so an Enderman cannot end the run mid-break) |
| P | a base with 30 souls, a lens and the tier-2 addon is broken in survival; **nothing spills**, and the drop's `minecraft:block_entity_data` carries `souls 30`, `progress`, `bar 16` and `carried [{Slot:0b, lens}, {Slot:1b, addon with its `titanium:augments`}]`; placing it back reads `souls 30`, a 16-tick bar, the lens and `augments efficiency 0.80`; an **empty** machine's item has no payload and places as a fresh 20-tick machine; a pipe's item carries its 3 souls and a surge's its 120 ticks |
| Q | a second blue lens on a base with one seated leaves the machine with one lens and the player holding one; `SELFTEST=replace` fails on `held 0` |
| N | `/industrialforegoingsouls info` on a vanilla position answers `<pos> is not an industrialforegoingsouls block.` |
| M2 | the parent's ore laser base in the End: `recipes 11/11`; `recipe none` in the End's own biome; `minecraft:deep_dark` at depth 0 → `industrialforegoingsouls:laser_drill_ore/echo_shard`, `weight 1`; one completed bar for the drill's `10000 → 9000 FE` puts **`minecraft:echo_shard x1`** in the output |

## Notes on the run

- **The dependency is declared.** `horizon-mod.toml` carries
  `industrialforegoing = { version = ">=0.2.7", … }`; a one-module bundle is refused by name before it
  starts (`sdk-dependency-refusal.log`, 0.2.3). The two-module bundle builds both, parent first.
- **Stale tags and export removed first.** The three `native-0a875da-*` builds carrying 0.2.3 and
  `~/.cache/horizon-e2e/0a875da/industrialforegoingsouls-bundle` were deleted, and the build log shows
  both crates compiling (`industrialforegoing (1 of 2)`, `industrialforegoingsouls (2 of 2)`).
- **`fits` is a two-build check.** `--build 0a875da` is the pin the package declares (it answers
  `made for`); `--build 0b14f8a` is a different Nightly and answers `fits`.
- **No packed `Cargo.lock`.** `cargo horizon-mod build` writes a probe lock once the manifest declares
  a dependency; the package's own `Cargo.toml` has only the SDK **path** dependency, so the lock is
  not a deliverable and each archive is stripped (`zip -q -d … Cargo.lock`).
- **Toolchain.** `cargo horizon-mod` at this pin needs rust **1.99.1** (`cargo +1.99.1 horizon-mod …`).
- **`ctl` does not expose a menu's data slots**, so phase S asserts them through
  `/industrialforegoingsouls info`'s `menu [...]` field (the block entity's `data`) — see `notes.md`
  §7 for the gap. The screen's *drawing* is not asserted: with `--offscreen` there is no pixel to read.
- Every leg ran in a build-slot lease; `HORIZON_E2E_ROOT` and `E2E_TMP` are under
  `~/.cache/horizon-e2e`, never `/tmp`. The scenario uses `--offscreen`; native commands go through
  `ctl chat`.
