# Validation — `industrialforegoingsouls` 0.2.1

The ladder actually run for the delivered package, cheapest first. Every compile went through
`ports/tools/build-slot.sh` (`CARGO_INCREMENTAL=0`, no `--target-dir`); `check`, both `fits` runs and
`cargo horizon-mod build` are the tool's own non-compiling checks, run in the port. The in-game legs
are launcher builds at the campaign pin **`0a875da91dbb0fdcb7ba8c42c11341aa229af51a`** + mixins
`e59da909…`, of a game carrying **this port and its parent** (`industrialforegoing` 0.2.8), because
the laser base's catalyst slot reads the parent's item and the ore recipe is the parent's type.

Evidence: `ports/_shared/evidence/industrialforegoingsouls/` and
`ports/_shared/evidence/industrialforegoingsouls-e2e/` (0.2.0 logs keep a `-0.2.0` suffix).

| step | command | result | evidence |
|---|---|---|---|
| red (rules) | `cargo test` with the five augment rules and `bar_max` at `todo!()` | **17 passed, 5 failed** (`not yet implemented: slice 2: (int) Math.floor(MAX_PROGRESS * efficiency)`) | `industrialforegoingsouls/sdk-test-red-slice2.log` |
| tests | `cargo test` | **22 passed, 0 failed** | `industrialforegoingsouls/sdk-test.log` |
| format | `cargo fmt --check` | clean (`FMT-CLEAN`) | `industrialforegoingsouls/sdk-clippy-slice2.log` |
| lints | `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`CLIPPY-CLEAN`) | `industrialforegoingsouls/sdk-clippy-slice2.log` |
| content | `python3 tools/assemble_content.py --check` | `differs or missing:` ×3 before, **`CONTENT OK: 56 files`** after | — |
| package | `rm -f Cargo.lock && cargo horizon-mod build` ×2 | `CMP-IDENTICAL`, sha256 `75ebd87afc83cf52197c522184b72f3968fbdf57ab607bcf1d76baf8b70b7c7c`, **83,745 B**, **63** entries, no packed `Cargo.lock` | `industrialforegoingsouls/sdk-build-slice2.log` |
| check | `cargo horizon-mod check industrialforegoingsouls.hmod` | `industrialforegoingsouls 0.2.1 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)`, **83,745 B**, sha256 `75ebd87a…c7c`, exit 0 | `industrialforegoingsouls/sdk-check-slice2.log` (prints the sha it ran against) |
| fits (pin) | `cargo horizon-mod fits industrialforegoingsouls.hmod --build 0a875da` | `0a875da  made for`, exit 0 | `industrialforegoingsouls/sdk-fits-slice2.log` (prints the sha) |
| fits (other) | `… --build 0b14f8a` | `0b14f8a  fits`, exit 0 | `industrialforegoingsouls/sdk-fits-slice2.log` (prints the sha) |
| package gate | `python3 ports/tools/verify-sdk-package.py industrialforegoingsouls` | **1/1** (22 tests) | — |
| launcher build | `tools/e2e-two-mod.sh build` on the 0.2.1 tree | `Compiling industrialforegoing (1 of 2)`, **`Compiling industrialforegoingsouls (2 of 2)`**, `the build carries: industrialforegoing 0.2.8, industrialforegoingsouls 0.2.1` | `industrialforegoingsouls/e2e-two-mod-build.log` |
| in-game, green | `tools/e2e-two-mod.sh run` on that build (`Up to date`, same tag) + `tools/e2e.sh` (A–N, D2–D3, M2) | **`PASS` + `EXIT:0`** | `industrialforegoingsouls-e2e/e2e-launcher.log` |
| control | the same script on `GAME=$CONTROL_GAME` (same Nightly, no package) | `FAIL: the game refused industrialforegoingsouls:soul_laser_base` → `EXIT:1` | `industrialforegoingsouls-e2e/e2e-launcher-control.log` |
| selftest, lens | `SELFTEST=1` (phase B asserts the tank is not empty without a lens) | `FAIL: selftest: the soul tank is 0, expected anything but 0` → `EXIT:1` | `…/e2e-launcher-selftest.log` |
| selftest, augment | `SELFTEST=augment` (phase D3 asserts the plain bar's rate on an augmented machine) | `FAIL: selftest: the shortened bar gained 22 souls, expected no more than the plain bar's 16` → `EXIT:1` | `…/e2e-launcher-selftest-augment.log` |
| selftest, ore | `SELFTEST=laser` (phase M2 asserts the output slot is empty) | `FAIL: selftest: the base yielded minecraft:echo_shard x1, expected an empty output slot` → `EXIT:1` | `…/e2e-launcher-selftest-laser.log` |
| selftest, growth | `SELFTEST=grow` (phase G asserts the unaccelerated wheat grew) | `FAIL: selftest: the control wheat's age is 0, expected it above 0` → `EXIT:1` | `…/e2e-launcher-selftest-grow.log` |
| log gate | `python3 ports/tools/check-e2e-logs.py industrialforegoingsouls-e2e --strict` | `logs=10 ok=10 missing-exit=0 bad-exit=0` | — |
| skills gate | `python3 ports/tools/check-skills.py --notes ports/industrialforegoingsouls` | PASS | — |

Artifact: **83,745 B**, SHA-256
`75ebd87afc83cf52197c522184b72f3968fbdf57ab607bcf1d76baf8b70b7c7c`, 63 entries, no packed
`Cargo.lock`, declared version **0.2.1**.

## What each in-game phase asserts

| phase | assertion |
|---|---|
| launch | 0 `load-items`/`recipes`/`advancements`/`content` errors in the game's log |
| A | the three block items register and read back; an unknown item and block id are refused by the game's own message; the machine and a `NoAI` warden read back through `/industrialforegoingsouls info` |
| B | 60 ticks, a warden in range, **no lens** → `souls 0`, `warden 500.0` |
| C | seat the blue lens by right-click (one taken from the hand); 60 ticks → `4 operations`, `8 souls` = `2 × ((500 − health) / 4)`, `warden 484.0` |
| D | an empty hand takes the lens back; 60 more ticks → the tank does not move |
| D2 | right-click with `efficiency_addon_tier_2` → `augments efficiency 0.80` and the addon gone from the hand; one operation later the bar reads `x/16` (and the plain phases above read `x/20`); a speed addon's click leaves it `held 1` and the efficiency unchanged |
| D3 | the lens back in: over a 160-tick window the 16-tick bar stores 20+ souls, where the plain bar fits eight operations (16) — the rate differential |
| E | `souls 50` seeded with the lens out; one pipe → the machine at 46 and the network 4/4; a second pipe → 8/8 and the machine at 42 |
| F | a surge placed on a full block's top face takes `facing=up`, and the readout agrees |
| G | `random_tick_speed 0`, a surge on the wheat's north face with one pipe: the network 4 → 3, `wait` 293 → 200 over 400 ticks, the accelerated wheat at **age 4** and the control at **age 0** |
| H | a redstone block beside the surge → `powered true` and `wait` frozen; removed → `false` and falling again |
| K | the three recipes craft in a real table (8 pipes), the ingredients are consumed, the grid is empty, and a grid of vanilla stand-ins crafts nothing |
| L | each block breaks in survival with an iron pickaxe and drops itself |
| N | `/industrialforegoingsouls info` on a vanilla position answers `<pos> is not an industrialforegoingsouls block.` |
| M2 | the parent's ore laser base, placed above the parent's drill in the End: `target [200, 101, 6]`, `facing up`, **`recipes 11/11`**; in the End's own biome `recipe none`; after `fillbiome … minecraft:deep_dark` and `depth 0` → `biome minecraft:deep_dark`, `recipe industrialforegoingsouls:laser_drill_ore/echo_shard`, `weight 1`; one completed base bar for the drill's `10000 → 9000 FE` puts **`minecraft:echo_shard x1`** in the output |

## Notes on the run

- **Two mods, one game.** `tools/e2e-two-mod.sh` copies both ports into one folder, runs the
  launcher's `native-build <folder>` ("a folder of them"), and exports the resulting version the way
  `launcher-game.sh` exports a single one. A one-mod game refuses this package at load:
  `mod industrialforegoingsouls requested unknown item industrialforegoing:blue_laser_lens` — the
  required-dependency refusal, recorded in `notes.md` §1.1. The ore phase needs the parent's
  machines for the same reason.
- Every leg ran in a build-slot lease (`build-slot.sh run --step e2e`, one job) with
  `HORIZON_E2E_ROOT` under `~/.cache/horizon-e2e` and `E2E_TMP` honoured (`/tmp` is a 7.5 GB tmpfs).
- The scenario runs `--offscreen` for a renderer; `ctl chat` is what reaches `run_mod_command`
  (`ctl command` takes the console path). The parent's `/industrialforegoing` commands are reached
  the same way, and their readouts are parsed by the parent port's own `parse_laser.py`.
- One leg failed while the parent port's tree was mid-write by another session
  (`industrialforegoing:fluid_laser_base is not a block`); it was re-run unchanged once the tree was
  consistent. Nothing in this port's evidence depends on that run.
