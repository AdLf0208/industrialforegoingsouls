# Validation — `industrialforegoingsouls` 0.2.3

The ladder actually run for the delivered package, cheapest first. Every compile went through
`ports/tools/build-slot.sh` (`CARGO_INCREMENTAL=0`, no `--target-dir`); `check`, both `fits` runs and
`cargo horizon-mod build` are non-compiling package checks and ran locally (`BUILD_LOCAL=1`) because
the remote box has no `cargo-horizon-mod`. The in-game legs are launcher builds at the campaign pin
**`0a875da91dbb0fdcb7ba8c42c11341aa229af51a`** + mixins `e59da909…`, of a game carrying **this port
and its parent** (`industrialforegoing` 0.2.10 in this run).

This is the **waterlogging** build: the Soul Pipe and the Soul Surge hold water as upstream's
`SimpleWaterloggedBlock` does (the state, the engine's water fluid state, and a bucket click that
fills and drains it). `SLICE-PLAN-0.2.3.md` is the plan and the pin measurements;
`notes.md` §1.4 and §7 record what the pin cannot express (a flowing fluid, a dispenser).

Evidence: `ports/_shared/evidence/industrialforegoingsouls/` and
`ports/_shared/evidence/industrialforegoingsouls-e2e/` (0.2.0 logs keep a `-0.2.0` suffix, 0.2.1 a
`-0.2.1` one; the unsuffixed logs are this run).

| step | command | result | evidence |
|---|---|---|---|
| red (waterlog rules) | `cargo test` with the four new rules at `todo!()` | **33 passed, 4 failed** (`not yet implemented: slice 0.2.3`) | `industrialforegoingsouls/sdk-test-red-waterlog.log` |
| red (drain destination) | `cargo test` with `bucket_destination` at `todo!()` | **37 passed, 1 failed** | `industrialforegoingsouls/sdk-test-red-waterlog-drain.log` |
| tests | `cargo test --locked --offline` | **38 passed, 0 failed** | `industrialforegoingsouls/sdk-test.log` |
| format | `cargo fmt --check` | clean (`FMT-CLEAN`) | `industrialforegoingsouls/sdk-clippy-waterlog.log` |
| lints | `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`CLIPPY-CLEAN`) | `industrialforegoingsouls/sdk-clippy-waterlog.log` |
| content | `python3 tools/assemble_content.py --check` | `CONTENT OK: 56 files` | — |
| dependency refusal | one-module `native-build` (only this port) | `industrialforegoingsouls requires missing mod industrialforegoing (… the reason and URL …)`, `NATIVE-BUILD-EXIT:1` | `industrialforegoingsouls/sdk-dependency-refusal.log` |
| package | `rm -f Cargo.lock && cargo horizon-mod build` ×2, the SDK-generated lock removed from each archive | `CMP-IDENTICAL`, sha256 `68b1485efc1539b665aeb623e986d042bb73626723bc3d9cd4c6b1d3124c88b8`, **101,380 B**, **63** entries, `PACKED-LOCK: 0` | `industrialforegoingsouls/sdk-build-waterlog.log` |
| check | `cargo horizon-mod check industrialforegoingsouls.hmod` | `industrialforegoingsouls 0.2.3 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)`, **101,380 B**, sha256 `68b1485e…88b8`, exit 0 | `industrialforegoingsouls/sdk-check-waterlog.log` |
| fits (pin) | `cargo horizon-mod fits industrialforegoingsouls.hmod --build 0a875da` | `0a875da  made for`, exit 0 | `industrialforegoingsouls/sdk-check-waterlog.log` |
| fits (other) | `… --build 0b14f8a` | `0b14f8a  fits`, exit 0 | `industrialforegoingsouls/sdk-check-waterlog.log` |
| package gate | `python3 ports/tools/verify-sdk-package.py industrialforegoingsouls` | **1/1** (38 tests; runtime byte-identical; every packed file is the tree's) | — |
| manifests | `python3 ports/tools/check-manifest-deps.py industrialforegoingsouls --strict` | `ok  declared=['industrialforegoing']` | — |
| launcher build | `tools/e2e-two-mod.sh build` (the stale `native-0a875da-f52cbec4dbd6` tag and the export removed first) | `Compiling industrialforegoing (1 of 2)`, **`Compiling industrialforegoingsouls (2 of 2)`**, `the build carries: industrialforegoing 0.2.10, industrialforegoingsouls 0.2.3` | `industrialforegoingsouls/e2e-two-mod-build.log` |
| in-game, green | `tools/e2e.sh` on that build (A–R, D2–D3, K–L, P–Q, M2) | **`PASS` + `EXIT:0`** | `industrialforegoingsouls-e2e/e2e-launcher.log` |
| control | the same script on `GAME=$CONTROL_GAME` (same Nightly, no package) | `FAIL: the game refused industrialforegoingsouls:soul_laser_base` → `EXIT:1` | `…/e2e-launcher-control.log` |
| selftest, lens | `SELFTEST=1` (phase B asserts the tank is not empty without a lens) | `FAIL: selftest: the soul tank is 0, expected anything but 0` → `EXIT:1` | `…/e2e-launcher-selftest.log` |
| selftest, augment | `SELFTEST=augment` (phase D3 asserts the plain bar's rate) | `FAIL: selftest: the shortened bar gained 22 souls, expected no more than the plain bar's 16` → `EXIT:1` | `…/e2e-launcher-selftest-augment.log` |
| selftest, ore | `SELFTEST=laser` (phase M2 asserts the output slot is empty) | `FAIL: selftest: the base yielded minecraft:echo_shard x1, expected an empty output slot` → `EXIT:1` | `…/e2e-launcher-selftest-laser.log` |
| selftest, growth | `SELFTEST=grow` (phase G asserts the unaccelerated wheat grew) | `FAIL: selftest: the control wheat's age is 0, expected it above 0` → `EXIT:1` | `…/e2e-launcher-selftest-grow.log` |
| selftest, carry | `SELFTEST=carry` (phase P asserts the restored machine kept nothing) | `FAIL: selftest: the restored tank is 30, expected 0` → `EXIT:1` | `…/e2e-launcher-selftest-carry.log` |
| selftest, replace | `SELFTEST=replace` (phase Q asserts the second lens voided the first) | `FAIL: selftest: the lens the swap returned is 1, expected 0` → `EXIT:1` | `…/e2e-launcher-selftest-replace.log` |
| selftest, waterlog | `SELFTEST=waterlog` (phase R asserts the bucket left the pipe dry) | `FAIL: selftest: the pipe's waterlogged state is true, expected false` → `EXIT:1` | `…/e2e-launcher-selftest-waterlog.log` |
| log gate | `python3 ports/tools/check-e2e-logs.py industrialforegoingsouls-e2e --strict` | `logs=16 ok=16 missing-exit=0 bad-exit=0` | — |
| skills gate | `python3 ports/tools/check-skills.py --notes ports/industrialforegoingsouls` | PASS | — |

Artifact: **101,380 B**, SHA-256
`68b1485efc1539b665aeb623e986d042bb73626723bc3d9cd4c6b1d3124c88b8`, 63 entries, no packed
`Cargo.lock`, declared version **0.2.3**.

## What each in-game phase asserts

| phase | assertion |
|---|---|
| launch | 0 `load-items`/`recipes`/`advancements`/`content` errors in the game's log |
| A | the three block items register and read back; an unknown item and block id are refused by the game's own message; the machine and a `NoAI` warden read back through `/industrialforegoingsouls info` |
| B | 60 ticks, a warden in range, **no lens** → `souls 0`, `warden 500.0` |
| C | seat the blue lens by right-click (one taken from the hand); 60 ticks → `4 operations`, `8 souls` = `2 × ((500 − health) / 4)`, `warden 484.0` |
| D | an empty hand takes the lens back; 60 more ticks → the tank does not move |
| D2 | right-click with `efficiency_addon_tier_2` → `augments efficiency 0.80` and the addon gone from the hand; one operation later the bar reads `x/16` (and the plain phases above read `x/20`); a speed addon's click leaves it `held 1` and the efficiency unchanged |
| D3 | the lens back in: over a 160-tick window the 16-tick bar stores 20+ souls, where the plain bar fits eight operations (16) |
| E | `souls 50` seeded with the lens out; one pipe → the machine at 46 and the network 4/4; a second pipe → 8/8 and the machine at 42 |
| F | a surge placed on a full block's top face takes `facing=up`, and the readout agrees |
| G | `random_tick_speed 0`, a surge on the wheat's north face with one pipe: the network 4 → 3 → 2, `wait` 293 → 0 over 1,200 ticks, the accelerated wheat at **age 7** and the control at **age 0** (1,200 ticks because a 3 % roll over a 400-tick window missed once; `notes.md` §4.3 has the measurement) |
| H | a redstone block beside the surge → `powered true` and `wait` frozen; removed → `false` and falling again |
| R | **waterlogging**: a fresh pipe reads `waterlogged=false` and `0/4`; a water-bucket click reads `true` and leaves the empty bucket in the hand; the waterlogged pipe pulls the seeded laser base `50 → 46` into `4/4`; an empty-bucket click reads `false` and puts the water bucket in the inventory; a surge at `[facing=north,waterlogged=true]` reads both properties back and spends a soul (`3/4`, `wait 292 → 256`); a dry surge reads `false`; the laser base, which upstream does not waterlog, refuses the property by the game's own parser; `SELFTEST=waterlog` fails on `true` |
| K | the three recipes craft in a real table (8 pipes), the ingredients are consumed, the grid is empty, and a grid of vanilla stand-ins crafts nothing |
| L | each block breaks in survival with an iron pickaxe and drops itself (peaceful from here, so an Enderman cannot end the run mid-break) |
| P | a base with 30 souls, a 16-tick bar, the blue lens and the tier-2 efficiency addon is broken in survival; the drop's `minecraft:block_entity_data` names `souls 30`, `bar 16`, `progress`, `lens`, `augments [{Efficiency: 0.8d}]` and `augment_item`; placing it back reads `souls 30`, a 16-tick bar, `lens industrialforegoing:blue_laser_lens`, `augments efficiency 0.80`; an **empty** machine's item has no payload and places as a fresh 20-tick machine; a pipe's item carries its 3 souls and a surge's its 120 ticks |
| Q | a second blue lens on a base with one seated leaves the machine with one lens and the player holding one (the first returned); `SELFTEST=replace` fails on `held 0` |
| N | `/industrialforegoingsouls info` on a vanilla position answers `<pos> is not an industrialforegoingsouls block.` |
| M2 | the parent's ore laser base, placed above the parent's drill in the End: `recipes 11/11`; in the End's own biome `recipe none`; after `fillbiome … deep_dark` and `depth 0` → `recipe industrialforegoingsouls:laser_drill_ore/echo_shard`, `weight 1`; one completed base bar for the drill's `10000 → 9000 FE` puts **`minecraft:echo_shard x1`** in the output |

## Notes on the run

- **The dependency is declared.** `horizon-mod.toml` carries
  `industrialforegoing = { version = ">=0.2.7", reason = "…", url = "https://…" }`; a one-module
  bundle is refused by name before it starts (`sdk-dependency-refusal.log`, re-run at 0.2.3). The
  two-module bundle builds both, parent first.
- **Stale tag and export removed first.** The 0.2.2 build `native-0a875da-f52cbec4dbd6` and
  `~/.cache/horizon-e2e/0a875da/industrialforegoingsouls-bundle` were deleted, and the build log
  shows both crates compiling.
- **`fits` is a two-build check.** `--build 0a875da` is the pin the package declares (it answers
  `made for`, a tautology); `--build 0b14f8a` is a different Nightly and answers `fits`.
- **No packed `Cargo.lock`.** `cargo horizon-mod build` writes a probe lock once the manifest
  declares a dependency; the package's own `Cargo.toml` has only the SDK **path** dependency, so the
  lock is not a deliverable and each archive is stripped (`zip -q -d … Cargo.lock`); the two builds
  stay `cmp`-identical.
- **Toolchain.** `cargo horizon-mod` at this pin needs rust **1.99.1** (`cargo +1.99.1 horizon-mod …`).
- **Two harness defects were found and fixed by this slice** (the scenario's, not the port's):
  `block_prop` answered only for the first state property (`awk` now matches any), and phase G's
  400-tick window left a 6.4 % chance of no 3 % roll (now 1,200 ticks). Both are in `notes.md` §4.3.
- Every leg ran in a build-slot lease; `HORIZON_E2E_ROOT` and `E2E_TMP` are under
  `~/.cache/horizon-e2e`, never `/tmp`. The scenario uses `--offscreen`; native commands go through
  `ctl chat`.
