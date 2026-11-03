# dormyu adaptation for RustCore / DormLab

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
flight outlier was observed. It is not a passed smoothness check. The correction
is being rebuilt and requires repeated flights before installation.
Same-camera inspection, frame measurements, persistence and two-client results
are recorded in the lab manifest only after the corresponding checks finish.

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

Target Horizon: `c9d474dd9116e84c243f8992d75c11314df56f06` (Minecraft 26.3).
Mixins tool: `0d6b103c70633d8fd9443d8d0acd3aadaacedb06`.

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
