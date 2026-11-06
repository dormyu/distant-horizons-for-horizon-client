# dormyu adaptation for RustCore / DormLab

## Published near-column handoff — 0.1.0+dormyu.8

Horizon f42ef42. Changing ordinary render distance now keeps departing detailed
chunks until the far meshes replacing them have uploaded. DH submits its actual
published cutout with the replacement mesh batch to Horizon's existing renderer
handoff; previously it always passed `None`, allowing the near chunks' 0.6-second
fade to expire while DH still had holes at their old positions.

Published masks remain attached to shown and retiring sections. Switching quality
back while replacements are pending therefore remeshes an old holed tile using
current near coverage. Bookkeeping is restricted to resident shown/blank sections,
and normal disable/shutdown still calls `clear_lods`. No terrain-recipe, full-data
cache format, or quality-distance curve change is included.

The stationary `/dh clear` path also reschedules sections whose old meshes remain
visible, rather than leaving an empty data store behind unchanged geometry.

28 standalone core tests and offline locked Clippy with warnings denied pass,
including a controlled delayed-remesh regression:
near32 shrinks, detail remains beyond the ordinary fade interval, and its cutout is
released in the replacement batch. The same test reverses quality while a tile is
retiring and verifies that its old hole is rebuilt. Combined client, visual
boundary, performance, and installation verification are still pending at this
source checkpoint. Vegetation simplification and other visual transitions remain
separate concerns; this is not a claim that all DH artifacts are solved.

LGPL-3.0-only notices and upstream attribution are unchanged. Local adaptation by
dormyu; no upstream publication is implied.

## Bounded boundary detail and native biome colors —0.1.0+dormyu.7

Horizon f42ef42, built code593671e, client `7292006b`. Immersive
Landscapes0.3.1/v2/descriptor6 and official Zoom2.0.0 are unchanged.

- Medium/High/Extreme retain one-block DH samples in a bounded band at the
  actual near boundary, including the server-limited radius for remote games.
  The section bounds intersecting radius-32 through radius+64 blocks are refined;
  the rest keeps its existing distance curve. Low/Lowest keep their original plan.
- Grass uses Horizon's own biome modifier and face-center world coordinates.
  Dark-forest darkening and positional swamp color now match the native API.
  Cherry/azalea/flowering-azalea/pale-oak leaves retain their precolored texture.
- No extra shader pass, generation recipe change or full cache reset is added.
  The existing water-volume-v2 full-data format/fingerprint is retained.

26 core tests and offline locked Clippy with warnings denied pass. The boundary
regression fails on the preceding renderer. Same source/cache copies were checked
while changing near32 ->8 ->2 ->16 ->32, including movement and held Zoom.
An edited flat grass platform separately compares native8 and distant near2;
this synthetic fixture is not evidence of natural terrain quality. In fixed regions
chosen before candidate captures, mean-RGB error fell by
98.8% for dark forest and
94.7% for swamp. Texture filtering,
native biome blending, vegetation simplification and other detail transitions remain.

The extra geometry has a cost. Medium near32 planning in the probe rises from230
to328 sections; the measured scene's resident far quads change from
5,406,894 to6,039,395. In a reopened same-cache
1080p/FOV90 near32/sim8/DH128 Medium/cap120 test (VSync/audio off):

| Measurement | Previous | This build |
| --- | ---: | ---: |
| Static CPU average, ms | 2.096 | 2.269 |
| Static GPU average, ms | 3.066 | 3.076 |
| Flight FPS | 119.60 | 119.53 |
| Flight P99 / maximum frame, ms | 9.53 / 16.53 | 9.41 / 18.43 |
| Process allocator commit, MB | 4221.3 | 4215.8 |
| Renderer GPU allocated / reserved, MB | 2195.0 / 2532.0 | 2259.1 / 2532.0 |

These short diagnostics do not establish cold near32, long-route or future heavy-pack
headroom. Low/Lowest avoid this band when that quality/cost tradeoff is unwanted.
RU/EN named creation, save/reopen, disabled-mod vanilla creation, unchanged old
v4/v5 recipes, a dedicated Rust server and two clients with shared block/independent
Zoom input pass. Installed Prism readback is recorded separately after installation.

LGPL-3.0-only notices remain. Original Rust port: Deniz Şahin/contributors;
original DH: James Seibel/contributors; local adaptation: dormyu. This is separate
from the MIT Immersive Landscapes terrain mod.

## Native water and near-column handoff —0.1.0+dormyu.6

Horizon f42ef42, built code0807fc6, client `5b599bb2`, with unchanged
Immersive Landscapes0.3.1/v2/descriptor6 and official Zoom2.0.0.

- Far terrain is cut out only for GPU-complete near columns, using the
  renderer's detail-column revision rather than CPU chunk membership alone.
- Exact column masks replace per-section counts. Moving an equal number of
  near columns now moves their holes; delayed workers cannot publish an old
  coverage snapshot. A deduplicated follow-up mesh survives a Section job.
- Exposed still-fluid tops and their side edges match the detailed interior
  height8/9 minus0.001. Water beneath a solid roof keeps its full height.
  Fluid material-face tags and the still-water top's V orientation match
  the detailed mesher. Shader-pack-specific visual acceptance is still open.
- The existing water-volume-v2 full-data format/fingerprint remains unchanged.
  This patch does not force a full cache rebuild or modify world recipes.

23 core tests and offline locked Clippy with warnings denied pass. Two new
regressions fail before the fixes; a third forces a delayed worker snapshot.
Same source-world/cache copies were compared while switching actual near
range32 ->8 ->2 ->16 ->32, with DH128 Medium, plus movement and near32 flight.
The reproduced black water-boundary line disappears in the checked scene:
723 dark pixels become0 in the fixed water rectangle. This local measure
is not a claim that every terrain/material/detail boundary is seamless.
Seabed and vegetation detail transitions remain visible in some views.

Native RU/EN creation UI, save/reopen, disabled-mod vanilla creation and
unchanged v4/v5 recipe copies pass. A dedicated Rust server and two Rust
clients share the world and an emerald-block update, receive DH geometry,
keep independent Zoom input, report zero decode errors and all exit0.
The network run is a functional near2 smoke, not remote far-quality acceptance.

Short same-cache offscreen1080p/FOV90 near32/sim8/DH128 Medium/cap120,
VSync/audio off: stationary 119.99FPS / max11.36ms;
flight 119.62FPS / P999.65ms / max14.77ms.
Fast profile, per-package horizon-client O2/64 codegen units/no incremental;
other engine libraries keep their existing optimized fast profile. One Cargo
job, all caches/temp on E:, build921.2s, minimum available RAM2.04GiB.
This is a bounded check, not long-exploration or future heavy-pack acceptance.
Installation/Prism readback is recorded separately in the lab native-build.json.

LGPL-3.0-only and all original copyright notices remain. Original port:
Deniz Şahin/contributors; original DH: James Seibel/contributors. This is the
local dormyu adaptation, separate from the MIT Immersive Landscapes mod.

## Water-volume correction —0.1.0+dormyu.5

Horizon f42ef42, built codeabae49b, combined runtime `c30f2439`.
Water/lava of the same fluid material no longer produce interior side walls
solely because their biome/color mapping IDs differ. Top colors remain distinct.
Ignored submerged vegetation does not split a contiguous water run; real air
gaps remain. Kelp/seagrass states project as their contained water volume in the
native bridge. The default fluid cache fingerprint advances to water-volume-v2,
rebuilding derived LODs only; ordinary blocks and generator recipes are retained.

Both new regressions fail on the previous code; all20 core tests now pass.
Offline locked Clippy passes with warnings denied. Same-world water captures
at30/60/90s, named generation/save checks, two Rust clients and independent
Zoom input checks pass. Short near32/DH128 Medium1080p offscreen diagnostic:
119.76FPS static, 118.39FPS flight, flight max20.59ms.
These targeted checks are not zero-seam, all-seed, long-exploration or heavy-pack
acceptance. Fractional fluid-level/material transitions remain under observation.
Actual installed Prism verification is recorded in native-build.json after delivery.
Upstream Deniz Şahin and original DH James Seibel/contributors credits and
LGPL-3.0-only/COPYING.GPL notices are retained. Historical adaptations follow.


## Rendering correction candidate — 2026-11-03

Local version `0.1.0+dormyu.2` addresses reported stepped cliffs and floating
foliage. It is not an upstream release or an accepted final renderer.

- Coarse sampling reduces a 2-by-2 footprint using native majority occupancy,
  rather than expanding a single rock/leaf sample over the whole cell.
- One shared native decoration builder supplies actual feature-complete columns
  around the player, bounded to the requested near ring and at most1024 blocks.
  Native request/cache bounds and idle-priority workers remain in force.
- Completed columns enter DH as FEATURES data; LIGHT data from gameplay chunks
  retains precedence. The engine drains completed work while stationary, including
  after a cached preview is reopened. Preview workers use independent forks.
- A sampling fingerprint revision rebuilds derived LOD files. Gameplay chunks,
  world-generator recipes and player edits are not migrated.
- `/dh status` includes the number of refined section updates. Queued DH jobs
  do not include every native decoration task; zero queued is not visual acceptance.
- Cache eviction is a deduplicated worker job. Large section arrays are released
  outside the store lock, rather than in the render thread's `replan` path.
- Shared feature polling is limited to4Hz; unchanged view coordinates skip cache
  pruning. Remote clients do not enqueue an unsupported full-feature protocol.

The new stationary-refinement test and17 existing core tests passed; core Clippy
passed with warnings denied, including after the cache-eviction correction.
The first combined engine compiled and rendered the reported camera, but a221ms
flight outlier was observed. The second build moves cache eviction off the game
thread. Three subsequent12-second flights on a copied warm-cache QA world had
maximum frame times26.20/36.35/44.70ms, averages110.56/109.08/109.45FPS and
P99 frame times18.56/20.56/19.69ms. Static average116.04FPS, P99 9.08ms.
Conditions:1080p offscreen, near16/simulation8/DH128 medium, cap120, VSync/audio
off, no competing Dota2 process at preflight. Each flight resets to its start,
so the measurement includes teleport/load work. This is a short diagnostic,
not visible-gameplay, near32/120, heavy-pack or long-exploration acceptance.

Final-binary light checks passed for RU/EN named world creation, exact recipes,
solo block persistence, disabled generator and an unchanged oldv4 recipe copy.
Two native clients on a loopback Rust server received49 chunks each, a shared
emerald-block update and DH geometry, with zero decode errors and all three
processes exiting0. Their short near2/cap15 run is not far-loading/performance
acceptance. The network reporter initially omitted its binary hash; the report
explicitly identifies its later comparison against the frozen candidate hash.

Same-camera images improve the distant forest, but coarse gray stepped faces
and far-water/material boundaries remain. This is a partial correction. No
renderer/landscape quality acceptance is claimed. Installed Prism readback and
delivery hashes are recorded separately in the lab manifest after they finish.

Limits: full decoration refinement uses the local native sampler in solo/LAN
host. The remote sampler's current API supplies grid previews and saved chunks;
it has no full-decoration request method. Remote clients receive the footprint
correction but do not gain a new feature-streaming protocol from this patch.
Cold generation is more expensive; bounded request metadata is not a claim that
the DH store is globally memory-bounded during unlimited exploration. Older-PC
settings and a long exploration run still require profiling.

The independent native Horizon far renderer is compared as another candidate.
Voxy's current Fabric/OpenGL implementation is not bundled or copied.

## Earlier adaptation and historical checks

Started 2026-11-02. This is a modified version of **Distant Horizons for Horizon
Client**, based on upstream commit `53e116bad8bbe55e3e4d76504af721f1260afb81` by
Deniz Şahin and contributors. Original Distant Horizons: James Seibel and
contributors. Original copyright notices, LICENSE and COPYING.GPL remain intact.
The code retains **LGPL-3.0-only**. It is separate from the MIT-licensed original
Immersive Landscapes generator.

Current compatibility/lifecycle candidate: `0.1.0+dormyu.4`, Horizon
`f42ef42ef6ceccb3c6a20bea2481d87949a51c81` (Minecraft 26.3).
Mixins tool: `ddd9700b21997c95d95ac6ee5ecafe3830cf622a` plus the inert
`recursion_limit` attribute fix, tested local tool commit `d4af4d8`.
Set `HORIZON_MIXINS_BIN` to that patched tool. Its source patch and MIT notice
are included separately. The unpatched tool cannot index this Nightly.

The preceding delivered `dormyu.2` build on c9d474d adds footprint reduction,
bounded local feature refinement and worker-side cache eviction. Its core has
18 passing tests. Three short warm-cache flights at near16/DH128 medium gave
109–111FPS averages and maximum frames26/36/45ms. This does not establish cold
world, near32/120, multiplayer load or final visual acceptance. Gray coarse
cliffs and water/material boundaries remain. Raising all LOD quality was not
adopted: the diagnostic's peak private commit reached about5.2GiB.

The f42 compatibility candidate also fingerprints generated caches by engine,
accepts a SHA256-verified local official source archive for the build, and
builds `horizon-ctl` from its new `horizon-client-lib` package. Generation and
rendering changes above are retained. The preceding dormyu.3 binary passed native
UI/persistence and near16 performance checks. A near32 diagnostic found5,183,671
resident LOD quads after disabling DH: one upload call starts fading, while later
DH frames are disabled and never release the faded tiles. Visibility of every
retained tile was not established. dormyu.4 uses the renderer's clear_lods API to
cancel pending uploads and release visible/fading tiles at shutdown. Native
checks on the final5dd3d0cd executable passed: normal IL world creation, exact
recipe, block save/reopen, unchanged old v4 recipe, and two Rust clients on a Rust
server with a shared block and three clean exits. Renderer cleanup was confirmed:
7,233,232 resident quads become0 after /dh off. At1080p offscreen, near16/sim8/
DH128 medium, cap120/VSync off/audio off: static120FPS, three short flights
116–119FPS, maximum frames22.60/25.68/20.16ms. First cold LOD fill reached288/288
by150s, job queue0 at180s, refinement may continue. Measured peak RSS1991MiB,
private commit4147MiB through the flights. These are limited diagnostics, not
32/120 or heavy-pack acceptance. A native-only32/full-detail scene with no far
geometry also shows genuine gray stone outcrops/cliffs; surface tuning and
remaining coarse LOD/water boundaries are separate open work. Local evidence:
RustCore/qa/dh-lifecycle-2026-11-03/reports; installation/Prism recorded separately.

The following initial adaptation notes and measurements are historical c9d474d
evidence; they do not establish f42 compatibility.

Changes by dormyu:

- `tools/horizon.py`: Windows executable names and incremental staging without
  requiring rsync. Paths remain under the local project target folder.
- `bridge.rs`, `chat_tree.rs`, `horizon-mixin.toml`: build DH command completions
  through the current connection chat-tree path, replacing removed Remote fields.
  Both `/dh` and `/distanthorizons` retain their subcommand grammar.
- `bridge.rs`: use each chunk's actual height layout; remote generation uses
  Horizon's server-provided far-terrain source with 16-by-16 tile requests.
  Cancel remote waits before joining workers. Cap generation workers at two and
  feed at most four loaded chunks per frame for the local pack's initial budget.
- `runtime.rs`: unavailable remote data stays missing rather than becoming air;
  retry with a two-second backoff and a bounded queue even for a stationary player.
  Added tests for temporary refusals and recovery.
- `compatibility.json`: pin the engine used by this adaptation.
- `fog.rs` / `runtime.rs`: extend the terrain fog range to the active DH range,
  while preserving fluid/effect fog. Regression coverage brings core tests to17.
- `bridge.rs`: local sampling now uses the same16-by16 footprint grid requests
  as remote sampling. The native API receives tile origins; the DH runtime
  already supplies cell centres, so no second half-stride offset is added.
- `tools/horizon.py`: allow an explicit `HORIZON_MIXINS_BIN` for the combined
  Immersive Landscapes native set. The pinned default remains available.
- `shutdown.rs` / `bridge.rs`: stop the thread-local DH engine at the beginning
  of `Game::shut_down`, while worker threads and the renderer still exist. On
  Windows, leaving it until process/thread-local destruction caused joins after
  ExitProcess had terminated workers. Manually disabling DH before exit avoided
  the observed failure; the explicit lifecycle hook is verified separately.

Validation so far: 16 core tests passed on Windows, including recovery tests;
the full optimized `fast` Horizon client and control tool built successfully.
Game checks passed on isolated data: solo rendered 140/140 sections, then reopened
from 169 persisted section files with zero newly generated sections; a placed
gold block survived reload. Two clients joined a loopback Rust dedicated server,
received remote LODs, saw a shared emerald-block change and reconnected successfully.
Disconnect plus control readback during background work took about 0.16 seconds.
The multiplayer sample reported 52/53 sections drawn with pending jobs; blank or
incomplete sections and steady-state queue activity need further profiling.
This is functional QA, not a final latency/FPS/load guarantee. Prism installation
is verified separately in the local DormLab build manifest.

Initial lab settings: DH distance 64 chunks, low quality, built-in Horizon LOD
off. These are conservative QA settings, not the final pack configuration.
The native server must explicitly offer far terrain (`horizon-lod-distance`).
Remote caches remain session-local; single-player section files are persisted
under the world's `data/DistantHorizons` directory.

No compiled game resources, user accounts or worlds should be published. Any
redistributed modified native bundle must preserve these notes, the original
license texts, and access to its corresponding modified source.

2026-11-02 combined candidate0.2.0-dev.1: full Epic-based native world generation
was tested at near16/32 with DH128 medium. At the reviewed mountain pose DH
reported288/288 sections, no queued jobs. This is a scene observation, not proof
that all far-water/material seams are fixed. The engine's far shortcut and close
water rendering still have known discrepancies. Flight frame-time spikes remain
open; high static average FPS is not smoothness acceptance. The rejected
4sections/1MiB upload limit is not a default.

The shutdown hook's combined binary built in8m36. A persisted solo IL world
with DH128, plus two Rust clients with DH64 on a fresh Rust server, all exited
with code0; server exit was also0. The clients received far geometry and the
same emerald-block update, with zero chunk decode errors. Their short network
run did not settle every far section; it does not prove complete steady-state
far rendering or heavy-pack performance.
