# dormyu adaptation for RustCore / DormLab

## Underwater handoff — 0.1.0+dormyu.18

DH source `3a8aa870971a3213f8436fe8e299eb95d30b4c73` corrects the reproduced
underwater transition without changing Immersive Landscapes 0.6.0 or Zoom 2 Rust.
Runtime SHA256: `33992c7848f91b2a3a730c5e0d267edf57e188d2461ed152aa73129b651a2047`.
See [validation-dormyu.18.md](docs/validation-dormyu.18.md) for scope and images.

Water skylight now follows depth in sampled water and its adjoining floor.
Capture retains raw kelp/seagrass states through generated, saved and native
summary paths. At width 1 in the existing Medium-or-higher handoff ring, plants
use native geometry, texture coordinates, ambient occlusion and light, with the
same resident-chunk cutout as far terrain. Derived caches/summary revisions
rebuild; saved chunks, edits and IL world recipes stay unchanged.

The full client, fresh capture hooks, aquatic models and native water-light
fixtures passed against the final client library. Earlier standalone checks
passed 70 tests and Clippy. Five native 16 → 8 → 16 / DH 64 captures and a
full-process reopen were reviewed: the original pale strip is absent there.
All five additional native 32 → 16 → 32 / DH 128 captures also retain aquatic
detail across the former bare strip. The two-client functional check passed.
Installation and actual Prism verification have separate delivery records;
these native checks do not substitute for them.

A separate distant blue/turquoise line remains visible. Four pixel pairs map
to deep lukewarm ocean and warm ocean surface data with different water colors;
the debug view distinguishes it from the native/DH and width1/2 boundaries.
This biome-tint transition currently has narrow blending and is not claimed fixed.

Current visual scope is vanilla Overworld smooth lighting without shaders.
Coarser detail remains approximate; other scenes and modes are not certified.
Performance tests were waived, and owner aesthetic approval remains separate.
Original DH/Rust-port authors, LGPL/GPL notices and Horizon/tooling MIT notices
remain unchanged. This is dormyu's adaptation, not an upstream release.

The entries below are historical and retain their original validation scope.

## Verified renderer changes — 0.1.0+dormyu.17

<!-- DH17-SELECTED-RENDERER-QA -->

The frozen DH source `47888d3dec2edda918c367a5a60d3afc8f9f2fd0` passed the selected native functional, two-client,
native 16 → 8 → 16 / DH 64 handoff and manual image checks described in
[validation-dormyu.17.md](docs/validation-dormyu.17.md). Runtime SHA256:
`a6a3fcfad445c5af67bf9a5e439d3512621e1cae296734b80e090c6f7ade6b07`. These results describe the compiled source; installation, Prism and
release-download verification are recorded separately.

Narrow predictive/native tree footprints retain supported stems independently of
majority terrain reduction. Water top tint interpolation stitches selected coarse
edges, tracks plan/data revisions and keeps uniform faces merged. Replanning skips
identical selected tile sets; comparison indices avoid per-tile sorting. Coarse
tint construction reads 512 boundary halo cells instead of 4356 full-field cells,
with identical tested output; the rendered tile's own tint field remains full.

The DHH2 cache format rebuilds derived DH data on first use while preserving saved
chunks, player edits and IL recipes. **FPS/performance tests were omitted at the
owner's request** (`performance_tested=false`), so this delivery has no new frame-rate
or loading-time results. Interrupted attempts retain their original status.
Visual QA uses native 16 → 8 → 16 / DH 64 Medium, 1080p and a 30 FPS cap. The owner's
native 32 / DH 128 settings are preserved. IL 0.6.0 and Zoom 2 Rust bytes are unchanged.

Source bundles require compilation into the pinned Horizon f42ef42 client, with
privately prepared game resources. For the combined IL set use the complete
[IL native-build tool instructions](https://github.com/dormyu/immersive-landscapes/blob/main/docs/native-build.md) and explicit `HORIZON_MIXINS_BIN`:
both inert `recursion_limit` support and concrete `Self` constructor HEAD fixes
are needed. The older DH-bundled patch covers only the former. Do not substitute
it for IL's full patch or infer installation from a source bundle download.

Visual review scope: Manual review of all twelve actual PNGs from the isolated same-candidate visual-02 world/cache: delta native16 to8 to16 and the original canopy camera, DH64 Medium,1920x1080,cap30. This is targeted renderer QA, not FPS or owner aesthetic acceptance.

Remaining observed limits:

- Underwater LOD still differs visibly in seabed detail, aquatic vegetation and lighting; the pale diagonal patch is a remaining renderer limitation, not a resolved water-tint defect.
- Coarse distant vegetation, terrain steps and native biome material/colour boundaries remain visible. This check covers the selected cameras and does not certify every seed, distance or scene.
- First-use DH cache rebuilding and newly explored terrain require preparation. Performance tests were explicitly omitted; no new FPS, frame-time,32/128 performance or heavy-pack guarantee is made.
- The owner has not given aesthetic approval of this exact delivered candidate.

The review covers the selected scenes; full-pack performance and owner aesthetic
approval remain unverified. Original DH/Rust-port notices and the Horizon MIT
notice remain intact. Historical entries below retain their original scope.


## Candidate dormyu.17 — narrow stems and water tint, 10 November 2026

Not installed or visually accepted yet. The verified fallback is DormLab IL0.6.0
with DH16, runtime4d7c4fec, published on10November. This renderer candidate keeps
IL's saved recipes and the native blocks unchanged.

Four observed jungle crowns were reproduced from the sampled generator and its
stride4 input: all7 contributing tall roots were missed before reduction.
Renderer-only metadata now retains exact predictive root positions around the
same native sampler call, independently of majority terrain columns. A preview
stem is one block wide (the existing predictor's conservative footprint), must
have its own surviving high crown and dry opaque support, and can adjust either
endpoint by at most one LOD cell. The known root whose own crown vanished is
rejected. Native cached/visited logs are captured before reduction; their exact
one-block footprints jointly retain native2x2 trees. Lighting bands do not spend
separate footprint slots. At most4 physical footprints per cell, through width16;
coarser terrain keeps the existing approximation. No broad wooden LOD pillars are
introduced by the extra faces. Existing majority-reduced geometry is unchanged.

The sparse metadata shares data completeness, near cutouts, storage and mesh
revision lifecycles. The new DHH2 derived cache/fingerprint requires initial LOD
repreparation; ordinary saved chunks and player edits are untouched. A reproduced
late SURFACE child replacing a LIGHT parent is now refused. Real edits at equal
or greater completeness remain allowed.

Water top colours interpolate existing data and retain merged uniform faces.
Fine boundaries stitch to the actually selected coarser tile's edge, with halo
and plan dependencies preventing stale colour meshes. This does not prove the
cause of every pale underwater patch. Reported face counts use emitted vertices.

Current checks:46 core tests,12 footprint regressions,3 capture/lifecycle tests
and Clippy pass;7 pure water-helper tests pass separately. On195 vertices of three
saved mixed-detail boundaries, RGB mismatch10/5/10 becomes<=0.5 from8-bit rounding,
with unchanged interiors and only small rectangle-count increases. These are
bounded code/data checks, not current gameplay/performance acceptance. New runtime,
matched screenshots, startup/restart/near-far/two-client and FPS checks still pending.
Local evidence: RustCore/qa/far-canopy-2026-11-10.

Attribution: the tree-choice observer in capture_vegetation.rs adapts the MIT
Horizon f42ef42 predictor by Ludovic Marie and Horizon contributors. The original
MIT notice is shipped as third-party/horizon-LICENSE. Dormyu wrote the sparse
metadata, integration and corrections. Original DH/Rust-port notices and LGPL
remain unchanged; this is not an upstream release.


## Verified IL 0.5.0 integration — 6 November 2026

Combined Horizon f42ef42 / IL224a831 / DH623dd71 / official Zoom725ff7a client
3bb2fb54 passed new-world/save/legacy, two Rust clients, same-process world
switching, distance32→8→32 screenshots and a cached near32/DH128 Medium run.
The original copied-world shoreline regression retains its original hashes;
623dd71 corrects only the displayed `.15` to `.16` byte from896318d. Actual final
client chat shows dormyu.16 and309/309 far sections. Deep shoreline curtains and
the detached strip are absent in the inspected fully covered views. Initial
far preparation remains asynchronous and may show partial coverage.

Final cached1080p:333.4 FPS stationary20s (P99 3.97ms,max6.61ms),319.2 FPS during
130blocks/30s aerial motion at walking speed (P99 6.30ms,max97.94ms). The latter
still includes a hitch. Earlier code-equivalent first-use fast flight measured
99.3 FPS at cap120; do not present the cached result as fresh exploration or a
performance improvement caused by the label fix. No heavy-pack/owner acceptance.
Full source/profile/hash evidence: IL docs/validation-0.5.0.md and local QA
release-0.5.0-final-2026-11-06. Installed-launcher delivery is a later receipt.

## Preserve partial terrain and cull invented walls — 0.1.0+dormyu.16

The real `prepare` path copied completeness flags into an empty update, skipped
sampling those visited columns, then merged their empty vectors back as LIGHT
data. This erased known terrain and left holes that appeared complete forever.
Preparation now produces a sparse update containing only sampled columns; a
concurrent store change also receives only that update. Existing terrain and
explicit air runs are preserved. Empty vectors remain unknown during loading,
same-detail merging, child reduction and parent fallback, so they can be filled.

Known opaque neighbors now hide occupied ranges regardless of skylight. The
old section-edge exception exposed buried compressed terrain down to bedrock.
Unavailable neighbors no longer imply an opaque cliff either; explicit known
air and genuine lower terrain still retain their exposed side faces.

The cache fingerprint changes because old erased columns could also produce
nonempty, damaged parent reductions. Only derived LOD data is rebuilt; real
world chunks, player edits and terrain generation are unchanged. First use
requires LOD preparation again. Publication revisions and background session
retirement from the previous adaptations remain in place.

44 standalone tests and locked offline all-target Clippy pass. Regressions
failed before the fixes for real partial preparation, empty LIGHT cache data,
lit equal-height opaque neighbors and unavailable dry neighbors. Additional
checks retain lower cliffs, known air, and data through all merge directions.
An offline reconstruction of the reported final cache reduced large border
faces from 173 to one genuine water-facing slope. This is causal mesh evidence,
not proof that new in-game images, loading or performance are accepted. Those
checks must run on the newly built client before installation.
Local adaptation by dormyu; LGPL notices and upstream attribution preserved.

## Reject obsolete worker meshes — 0.1.0+dormyu.15

Two workers could finish the same section out of order. Mesh-job deduplication
ended when a worker started, and publication checked only the near cutout.
A slow result using older data, neighbors or settings could therefore overwrite
a newer mesh with exactly the same cutout and remain visible while stationary.

Each requested section appearance now has a monotonically increasing revision,
including requests coalesced into a pending job. Publication rejects older
revisions and retains the last published mesh until the scheduled replacement
arrives. Rejected results are dropped normally; rejection does not requeue and
invalidate an already pending replacement. Revision bookkeeping is restricted
to wanted or queued sections, and cache clearing invalidates pending results.
Face counts now describe accepted meshes rather than late, rejected results.
Actual source replacements also invalidate their own and adjacent appearance
stamps while the store lock is held. This closes a second ordering case: a
previously queued Mesh may start under Section's new request stamp before
`prepare` writes its data. Request revisions alone do not distinguish these
two snapshots. Both direct preparation and CAS-merged chunk/refinement updates
use the same invalidation rule; existing scheduling still supplies replacements.

The two-worker regression deliberately holds an old mesh, publishes new chunk
data and settings, then releases the older result. It fails on dormyu.14 and
passes after the fix. A second regression holds a partial-data mesh while the
real `prepare` path replaces that source under the same request; it fails before
the write-time invalidation and passes afterward. All 39 standalone tests and
locked offline Clippy pass.
Existing neighbor-remesh,
coverage, cache-clear, retry and background-retirement checks remain included.
The background retirement behavior, terrain, mesh geometry, saved data format
and geometry cache fingerprint are unchanged.

Offline reconstruction of the reported shoreline's final saved data found no
large section-edge curtains with known neighbors, while absent neighbors can
produce deep exposed faces there. This supports testing stale publication but
does not prove that it explains every artifact in the captured frame. A fresh
same-camera visual check is required before installation or visual acceptance.
Local adaptation by dormyu; LGPL notices and upstream attribution preserved.

## Background session retirement — 0.1.0+dormyu.14

The dormyu.13 diagnostic client accepted `/dh off` immediately, but the next
geometry status arrived 20.276 seconds later with only 13 additional frames.
The frame path cleared the renderer, then synchronously dropped the engine:
joining workers, propagating pending coarse data and saving dirty cache files.
The relative cost of those phases and optimized-client latency were not measured.

Session retirement now runs on one background worker. Its capacity-one channel
uses nonblocking `try_send`; another session cannot start until the previous
destructor and cache save have finished. This also serializes rapid off/on and
world changes, so two DH engines cannot write the same cache concurrently.
Renderer cleanup remains on the frame thread. Final application exit explicitly
joins the retirement worker while the game's resources and Windows threads
still exist, so exiting may wait for the accepted save to finish.

A destructor panic resets the busy flag and latches a separate failure. New DH
sessions then remain disabled until client restart. An unexpected enqueue
failure retains ownership until final exit instead of dropping on a live frame.
37 standalone tests and locked offline all-target Clippy pass. New regressions
exercise a real busy Engine and its persisted parent data, bounded rejection of
a second session, final-exit draining, and panic recovery without unsafe restart.

No meshing, terrain, saved data format or geometry cache salt changes. Native
client off/on latency, world changes and exit still need runtime verification;
the standalone checks do not establish smoothness or installation acceptance.
Local adaptation by dormyu; LGPL notices and upstream attribution preserved.

## Coarse shore and canopy occupancy — 0.1.0+dormyu.13

Reducing loaded terrain to a coarser tile previously kept any non-air child,
even when the other three children contained known air. A lone shore protrusion
or leaf column could therefore grow across the whole parent cell. This differed
from Horizon's direct preview, which requires at least half the footprint to be
occupied. Parent reduction now uses that same two-of-four threshold. Existing
material tie ordering remains unchanged, and missing children remain missing.

The counterfactual fixture fails on dormyu.12 for one occupied child out of four.
It now passes for grass and opaque leaves at all four occupancy levels, while
retaining the continuous water below and the data-completeness flags. All 34
standalone tests and offline locked all-target Clippy with warnings denied pass.
The cache fingerprint changes so old derived coarse columns are rebuilt; the
saved data format, actual world chunks and terrain recipes are unchanged.
The runtime version label now matches the package and mixin manifests.

This establishes the reduction defect and its correction. It does not yet prove
that it explains the remaining pale surface patches in the dormyu.12 screenshot.
Fresh-world and reopened-world visual comparisons are required before accepting
that material boundary. Local adaptation by dormyu; LGPL notices preserved.

## Dry banks beside lit water — 0.1.0+dormyu.12

The dormyu.11 fresh-world check removed most submerged curtains but retained a
large shore-facing sheet after reducing ordinary distance from 32 to 8 chunks.
The source bank was dry: its neighboring solid layer was submerged, so checking
only water above the source did not suppress the legacy lit-border exception.

Known submerged solid neighbors now occlude the buried part of a dry bank too.
A counterfactual fixture reproduces a false Y=-64-to-70 wall against a lit bed
at Y=61 with water at Y=63; only the real Y=61-to-70 exposed bank remains after
the correction. This does not project unknown neighbors, alter terrain recipes,
or change the saved data format. All 33 standalone tests and locked offline
all-target Clippy pass. A new-world visual check is still required before this
checkpoint can be considered installable. Local adaptation by dormyu.

## Lit underwater borders in a fresh world — 0.1.0+dormyu.11

The dormyu.10 cold-cache check on existing chunks passed, but a fresh-world
32-to-8 transition still exposed rectangular underwater dirt/stone curtains.
Synthetic columns carry sky light through water to the bed. A legacy section
edge exception then drew the covered side of a lit opaque bed even when the
known neighboring bed was identical. Its full compressed run extended to the
world bottom; waiting for ordinary chunk data hid the defect on reopening.

That lighting exception now applies only above water. Known opaque neighbors
occlude submerged ground normally, while the uncovered height of a genuine
underwater cliff remains visible. Above-ground detail skirts, saved full-data
format, terrain recipes and generation work are unchanged.

The new counterfactual regression fails on dormyu.10 with four buried walls
from Y=-64 to Y=61 between equal lit lake beds. It passes after the fix and also
checks that a real Y=50-to-61 cliff remains. Fresh-world and existing-world
combined-client visual checks are required separately before installation;
the earlier existing-world result does not establish fresh-world acceptance.
All 32 standalone tests and offline locked all-target Clippy with warnings
denied pass on this source checkpoint.
Local adaptation by dormyu; LGPL notices and upstream attribution preserved.

## Cold-water tile borders — 0.1.0+dormyu.10

The combined dormyu.9 client removed the large missing near-terrain ring, but a
cold far-data cache still exposed rectangular dirt walls through water. A missing
neighbour was treated as open air: compressed underground runs gained visible
side faces. A near cutout was also incorrectly treated as an excavation.

Cutout columns now retain their side-occlusion data. Unknown neighbouring data
does not justify a wall below a covering water surface; known lower beds still
produce their real underwater cliffs. Existing same-level neighbour files can
provide occlusion even when that tile is not selected by the current detail plan.
Preparing a tile notifies its four wanted neighbours to rebuild stale borders.
This adds no frame/tick sampling and does not generate extra neighbour tiles.

31 standalone tests and offline locked Clippy with warnings denied pass. New
regressions cover unknown/empty water neighbours, a real underwater height step,
arrival of a generated neighbour, and an unselected neighbour stored on disk.
The previous delayed 32-to-8 handoff regressions remain green. Combined-client
cold-cache screenshots are still required; this checkpoint is not visual or
installation acceptance. No terrain recipe, full-data format, or quality curve
changes. LGPL notices remain intact; local adaptation by dormyu.

## First far mesh after a near-distance reduction — 0.1.0+dormyu.9

The combined dormyu.8 candidate exposed a large empty ring after ordinary render
distance changed from 32 to 8 chunks. Its earlier regression covered a partially
holed far tile. A tile wholly inside near terrain is skipped entirely, so it has
no published cutout to preserve while its first far mesh is being prepared.

The engine now retains previously published near columns while a current wanted
far tile covers them but has not yet published unmasked geometry or a verified
blank result. Releasing those columns remains part of the replacement upload
batch. Columns outside the current DH tile plan are released, so travel does not
retain old detailed terrain indefinitely. The lookup uses at most nine quadtree
levels per previous column, only when coverage changes. Horizon's existing
retirement, fade, CPU/GPU drawing and upload APIs need no changes.

29 standalone tests pass, including a fully populated near-32 view reduced to
near-8 with a delayed first mesh beyond the renderer's 600 ms fade. The regression
checks both coordinate signs, actual replacement geometry, eventual release and
bounded removal outside DH range. It fails at the missing near column when the
dormyu.8 retention behavior is restored in an isolated test copy. Offline locked
Clippy with all targets and warnings denied passes. Combined-client transition
screenshots and installation remain pending at this source checkpoint; these
tests do not establish that every DH artifact is fixed.

No terrain recipe, saved full-data format or quality-distance curve changes.
LGPL notices and upstream attribution are preserved; local adaptation by dormyu.
The dormyu.8 notes below describe the earlier, incomplete handoff implementation.

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
