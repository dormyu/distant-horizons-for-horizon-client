// Horizon's side of the Distant Horizons port, added to the game as
// `game::distant_horizons_bridge`: the block table, chunks and world
// generator `game::distant_horizons` reads, the meshes it makes in Horizon's
// far-terrain format, and the session that runs it for the world the player
// is in. Only this module knows Horizon's types; it compiles only inside the
// game (the mod's Cargo library is runtime.rs alone).
//
// Part of Distant Horizons for Horizon Client, LGPL-3.0 (see runtime.rs).

use crate::game::Game;
use crate::game::distant_horizons::{
    self as dh, BlockInfo, BlockTable, ChunkView, ColumnSampler, DetailPlan, Dir, FullDataSource, Host, Quad,
    RenderSettings, SampledRun, Settings,
    engine::{Engine, Near, Retirement, WorkSettings},
    material, pos,
};
use horizon_assets::model::Direction;
use horizon_client::{
    chunk::{Chunk, ColumnLayout},
    lod::TileKey,
    mesher::{OPAQUE_LAYER, SectionMesh, TerrainVertex, fluid_face},
    streaming::{LodSampler, LodSource, WorkerContext},
};
use horizon_core::position::BlockPos;
use horizon_world::registry::BlockStateId;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Chunks put in the LODs per frame at most; the rest wait for later frames.
const CHUNKS_PER_FRAME: usize = 4;
/// How often loaded chunks are checked for new and changed ones.
const SCAN_EVERY: Duration = Duration::from_millis(250);
/// How often one chunk may be read again after it changed.
const REREAD_EVERY: Duration = Duration::from_secs(5);
/// The far-terrain upload budget per frame, as Horizon's own far terrain.
const UPLOAD_BUDGET: Duration = Duration::from_millis(2);

// These plants contain water even though their block is non-colliding. Far
// terrain projects the fluid volume; detailed near chunks keep the plant mesh.
fn water_like(name: &str) -> bool {
    matches!(
        name.strip_prefix("minecraft:").unwrap_or(name),
        "water" | "bubble_column" | "kelp" | "kelp_plant" | "seagrass" | "tall_seagrass"
    )
}

/// What `BlockInfo` Horizon's block facts give each state.
fn block_table(facts: &horizon_client::blocks::BlockFacts) -> BlockTable {
    let states = (0..facts.states.len())
        .map(|i| {
            let state = BlockStateId(i as u32);
            let f = facts.get(state);
            let name = f.name.strip_prefix("minecraft:").unwrap_or(&f.name);
            let air = facts.is_air(state);
            let water = water_like(name);
            let liquid = water || name == "lava";
            let ice = name == "ice" || name == "frosted_ice";
            let see_through = water || ice || name.contains("glass") || name == "slime_block" || name == "honey_block";
            let material = if air {
                material::AIR
            } else if water {
                material::WATER
            } else if name == "lava" {
                material::LAVA
            } else if name == "snow" || name == "snow_block" || name == "powder_snow" {
                material::SNOW
            } else if name.ends_with("_leaves") {
                material::LEAVES
            } else if name.contains("sand") {
                material::SAND
            } else if name == "grass_block" {
                material::GRASS
            } else if f.emission > 0 {
                material::ILLUMINATED
            } else {
                material::UNKNOWN
            };
            // Leaves draw opaque far away, as Horizon's own far terrain does.
            let opaque = (f.solid_render || name.ends_with("_leaves")) && !see_through;
            BlockInfo {
                air,
                opacity: if air {
                    0
                } else if opaque {
                    16
                } else {
                    f.dampening.min(15)
                },
                // Chorus is thin, but a visible part of the far End.
                solid: !air && (f.full_collision || name.starts_with("chorus_")),
                liquid,
                material,
                alpha: if see_through { 0xA0 } else { 255 },
                snow_layer: name == "snow",
                ice,
            }
        })
        .collect();
    BlockTable { states }
}

/// The block states and biomes the section files were written with: files
/// from another set are made again.
fn fingerprint(facts: &horizon_client::blocks::BlockFacts) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    // Coarse parents now keep the same half-footprint occupancy threshold as
    // generated previews. Rebuild derived files made with the old air-losing
    // reduction; gameplay chunks and player edits stay intact.
    "dormyu-occupancy-v3-f42ef42".hash(&mut h);
    for s in &facts.states {
        s.name.hash(&mut h);
    }
    h.finish()
}

/// A loaded chunk, read by `LodDataBuilder`.
struct LoadedChunk {
    chunk: Arc<Chunk>,
    air: u32,
}

impl ChunkView for LoadedChunk {
    fn chunk_x(&self) -> i32 {
        self.chunk.x
    }
    fn chunk_z(&self) -> i32 {
        self.chunk.z
    }
    fn min_build_height(&self) -> i32 {
        self.chunk.layout().min_y()
    }
    fn max_build_height(&self) -> i32 {
        self.chunk.layout().max_y()
    }
    fn min_non_empty_height(&self) -> i32 {
        let air = self.air;
        let lowest = self.chunk.sections.iter().position(|s| !s.is_uniform(air));
        lowest.map_or(self.chunk.layout().max_y(), |s| {
            self.chunk.layout().min_y() + s as i32 * 16
        })
    }
    fn top(&self, x: i32, z: i32) -> i32 {
        let air = self.air;
        self.chunk
            .highest(air, |y| self.chunk.get(x, y, z) != air)
            .map_or(self.chunk.layout().min_y(), |y| y + 1)
    }
    fn state(&self, x: i32, y: i32, z: i32) -> u32 {
        if (self.chunk.layout().min_y()..self.chunk.layout().max_y()).contains(&y) {
            self.chunk.get(x, y, z)
        } else {
            self.air
        }
    }
    fn biome(&self, x: i32, y: i32, z: i32) -> u16 {
        self.chunk.biome(x, y, z)
    }
    fn light(&self, x: i32, y: i32, z: i32) -> (u8, u8) {
        let light = self.chunk.light(x, y, z);
        (light >> 4, light & 15)
    }
}

/// Horizon's far-terrain sampler of the world generator.
// dormyu adaptation: batch local and server sampling with matching footprints.
// RemoteSampler.column() is intentionally empty; its grid() is the network API.
struct Sampler {
    inner: Box<dyn LodSampler>,
    stride: i32,
    tiles: HashMap<(i32, i32), Vec<horizon_client::lod::Column>>,
    refinement: Arc<Mutex<Refinement>>,
    build_height: (i32, i32),
}

/// One shared native decoration builder, independent of the DH sampling workers.
/// Its own queue/cache are bounded by Horizon; do not start one builder per tile.
#[derive(Default)]
struct Refinement {
    enabled: bool,
    sampler: Option<Box<dyn LodSampler>>,
    pending: HashSet<(i32, i32)>,
    completed: HashSet<(i32, i32)>,
    center: [i32; 2],
    near_radius: i32,
    last_request: Option<Instant>,
    last_poll: Option<Instant>,
}

impl Refinement {
    fn wanted(&self, x: i32, z: i32) -> bool {
        let dx = (x * 16 + 8 - self.center[0]).abs();
        let dz = (z * 16 + 8 - self.center[1]).abs();
        dx.max(dz) <= 1024 && dx.max(dz) >= self.near_radius * 16
    }
    fn request_tile(&mut self, x: i32, z: i32, stride: i32) {
        if !self.enabled || stride > 4 {
            return;
        }
        for cz in z.div_euclid(16)..(z + stride * 16).div_euclid(16) {
            for cx in x.div_euclid(16)..(x + stride * 16).div_euclid(16) {
                if self.wanted(cx, cz) && !self.completed.contains(&(cx, cz)) {
                    self.pending.insert((cx, cz));
                }
            }
        }
    }
    fn drain(&mut self, min_y: i32, max_y: i32) -> Vec<FullDataSource> {
        if !self.enabled
            || self
                .last_poll
                .is_some_and(|last| last.elapsed() < Duration::from_millis(250))
        {
            return Vec::new();
        }
        self.last_poll = Some(Instant::now());
        let mut sections: HashMap<i64, FullDataSource> = HashMap::new();
        let Some(sampler) = &mut self.sampler else {
            return Vec::new();
        };
        for ((cx, cz), columns) in sampler.drain_details() {
            self.pending.remove(&(cx, cz));
            self.completed.insert((cx, cz));
            let bx = cx * 16;
            let bz = cz * 16;
            let key = pos::containing_block(pos::SECTION_MINIMUM_DETAIL_LEVEL, bx, bz);
            let source = sections.entry(key).or_insert_with(|| FullDataSource::empty(key));
            let ox = bx - pos::min_corner_block_x(key);
            let oz = bz - pos::min_corner_block_z(key);
            let mut runs = Vec::new();
            for z in 0..16 {
                for x in 0..16 {
                    runs.clear();
                    Sampler::runs(&columns[z * 16 + x], &mut runs);
                    let values = dh::column_from_runs(&mut source.mapping, &runs, min_y, max_y);
                    source.set_column(ox + x as i32, oz + z as i32, values, dh::gen_step::FEATURES);
                }
            }
        }
        if self
            .last_request
            .is_none_or(|last| last.elapsed() >= Duration::from_secs(1))
        {
            self.last_request = Some(Instant::now());
            let mut wanted: Vec<_> = self.pending.iter().copied().collect();
            wanted.sort_unstable_by_key(|&(x, z)| {
                let dx = i64::from(x * 16 + 8 - self.center[0]);
                let dz = i64::from(z * 16 + 8 - self.center[1]);
                dx * dx + dz * dz
            });
            wanted.truncate(128);
            sampler.request_detail(&wanted, self.center);
        }
        sections.into_values().collect()
    }
}
impl Sampler {
    fn runs(column: &horizon_client::lod::Column, out: &mut Vec<SampledRun>) {
        out.extend(column.spans.iter().map(|span| SampledRun {
            min: i32::from(span.min),
            max: i32::from(span.max),
            state: span.state.0,
            biome: span.biome,
            sky: ((span.light >> 4) & 15) as u8,
            block: ((span.light >> 12) & 15) as u8,
        }));
    }
}
impl ColumnSampler for Sampler {
    fn request_section(&mut self, section: i64) {
        let width = pos::data_width(section);
        if width > 4 {
            return;
        }
        let (x, z) = (pos::min_corner_block_x(section), pos::min_corner_block_z(section));
        let mut refine = self.refinement.lock().unwrap();
        for dz in 0..4 {
            for dx in 0..4 {
                refine.request_tile(x + dx * width * 16, z + dz * width * 16, width);
            }
        }
    }
    fn view(&mut self, center: [i32; 2], near_radius: i32) {
        let mut refine = self.refinement.lock().unwrap();
        if refine.center == center && refine.near_radius == near_radius {
            return;
        }
        refine.center = center;
        refine.near_radius = near_radius;
        // Bound bookkeeping as the player travels. Completed data itself is in DH's store.
        let in_range =
            |&(x, z): &(i32, i32)| (x * 16 + 8 - center[0]).abs().max((z * 16 + 8 - center[1]).abs()) <= 1152;
        refine.pending.retain(in_range);
        refine.completed.retain(in_range);
    }
    fn drain_refined(&mut self) -> Vec<FullDataSource> {
        self.refinement
            .lock()
            .unwrap()
            .drain(self.build_height.0, self.build_height.1)
    }
    fn begin_section(&mut self, stride: i32) {
        self.stride = stride;
        self.tiles.clear(); // at most 64 half-stride tiles for one DH section
    }
    fn column(&mut self, x: i32, z: i32, out: &mut Vec<SampledRun>) {
        Self::runs(&self.inner.column(x, z), out);
    }
    fn try_column(&mut self, x: i32, z: i32, out: &mut Vec<SampledRun>) -> bool {
        // Summarise an area, rather than inflating one arbitrary rock/leaf into
        // a full distant cell. The native reduction requires majority occupancy.
        let sample_stride = (self.stride / 2).max(1);
        let scale = self.stride / sample_stride;
        let tile_size = sample_stride * 16;
        let tx = x.div_euclid(tile_size) * tile_size;
        let tz = z.div_euclid(tile_size) * tile_size;
        let columns = self.tiles.entry((tx, tz)).or_insert_with(|| {
            self.refinement.lock().unwrap().request_tile(tx, tz, sample_stride);
            self.inner.grid(tx, tz, sample_stride, 16)
        });
        // A refusal or cancellation stays missing, never becomes permanent air.
        if columns.len() != 256 {
            return false;
        }
        // x/z are the cell centres supplied by DH, grids start at cell origins.
        let ix = ((x - self.stride / 2 - tx) / sample_stride) as usize;
        let iz = ((z - self.stride / 2 - tz) / sample_stride) as usize;
        let columns: &[horizon_client::lod::Column; 256] = columns.as_slice().try_into().unwrap();
        let reduced = horizon_client::lod::Column::reduced(columns, ix, iz, scale as usize);
        Self::runs(&reduced, out);
        true
    }
}

#[derive(Clone, Copy)]
enum Tint {
    None,
    Grass,
    Foliage,
    Dry,
    Water,
    Constant(u32),
}

/// A face's sprite: the block model's face looking that way, as Horizon's
/// own far terrain picks it (`lod::mesh::Mesher::material`).
#[derive(Clone, Copy)]
struct Style {
    layer: u16,
    tint: Tint,
    translucent: bool,
    fluid: bool,
}

struct HorizonHost {
    layout: ColumnLayout,
    context: Arc<WorkerContext>,
    blocks: BlockTable,
    /// The world generator, once the game has one to give.
    source: Mutex<Option<Arc<dyn LodSource>>>,
    styles: Mutex<HashMap<(u32, u8), [Option<Style>; 2]>>,
    refinement: Arc<Mutex<Refinement>>,
}

fn direction(dir: Dir) -> Direction {
    match dir {
        Dir::Down => Direction::Down,
        Dir::Up => Direction::Up,
        Dir::North => Direction::North,
        Dir::South => Direction::South,
        Dir::West => Direction::West,
        Dir::East => Direction::East,
    }
}

impl HorizonHost {
    fn styles(&self, state: u32, dir: Dir) -> [Option<Style>; 2] {
        let key = (state, dir as u8);
        if let Some(found) = self.styles.lock().unwrap().get(&key) {
            return *found;
        }
        let styles = self.resolve(state, direction(dir));
        self.styles.lock().unwrap().insert(key, styles);
        styles
    }
    fn resolve(&self, state: u32, direction: Direction) -> [Option<Style>; 2] {
        let assets = &self.context.assets;
        let render = &self.context.render;
        let name = assets
            .registry
            .state(BlockStateId(state))
            .map_or("", |s| s.name.as_str());
        let fluid = if water_like(name) {
            Some(("block/water_still", true))
        } else if name == "minecraft:lava" {
            Some(("block/lava_still", false))
        } else {
            None
        };
        if let Some((sprite, water)) = fluid {
            let layer = render
                .sprite_layers
                .get(sprite)
                .or_else(|| render.sprite_layers.get(&format!("minecraft:{sprite}")))
                .copied()
                .unwrap_or(0) as u16;
            let tint = if water { Tint::Water } else { Tint::None };
            return [
                Some(Style {
                    layer,
                    tint,
                    translucent: water,
                    fluid: true,
                }),
                None,
            ];
        }
        let leaves = name.ends_with("_leaves");
        let tint = match name.strip_prefix("minecraft:").unwrap_or(name) {
            "spruce_leaves" => Tint::Constant(0x61_9961),
            "birch_leaves" => Tint::Constant(0x80_A755),
            "cherry_leaves" | "azalea_leaves" | "flowering_azalea_leaves" | "pale_oak_leaves" => Tint::None,
            "leaf_litter" => Tint::Dry,
            _ if leaves => Tint::Foliage,
            _ => Tint::Grass,
        };
        let models = render
            .model_catalog
            .collect(BlockStateId(state), BlockPos::new(0, 0, 0))
            .ok();
        let mut matching = models
            .iter()
            .flatten()
            .flat_map(|(_, model)| &model.faces)
            .filter(|face| face.quad.facing == direction);
        let mut faces = [None; 2];
        for slot in &mut faces {
            let Some(face) = matching.next() else { break };
            *slot = Some(Style {
                layer: face.layer as u16 | if leaves { OPAQUE_LAYER } else { 0 },
                tint: if face.quad.tint_index >= 0 { tint } else { Tint::None },
                translucent: face.translucent && !leaves,
                fluid: false,
            });
            // Grass's tinted side overlay must follow its untinted base.
            if name != "minecraft:grass_block" || matches!(direction, Direction::Up | Direction::Down) {
                break;
            }
        }
        // A model with no face this way (a plant): its first face, so the
        // far box still shows its colour.
        if faces[0].is_none()
            && let Some(face) = models.iter().flatten().flat_map(|(_, model)| &model.faces).next()
        {
            faces[0] = Some(Style {
                layer: face.layer as u16 | if leaves { OPAQUE_LAYER } else { 0 },
                tint: if face.quad.tint_index >= 0 { tint } else { Tint::None },
                translucent: face.translucent && !leaves,
                fluid: false,
            });
        }
        faces
    }
}

impl Host for HorizonHost {
    type Mesh = SectionMesh;

    fn blocks(&self) -> &BlockTable {
        &self.blocks
    }
    fn build_height(&self) -> (i32, i32) {
        (self.layout.min_y(), self.layout.max_y())
    }
    fn sampler(&self) -> Option<Box<dyn ColumnSampler>> {
        let source = self.source.lock().unwrap().clone()?;
        let mut refine = self.refinement.lock().unwrap();
        if refine.enabled && refine.sampler.is_none() {
            refine.sampler = Some(LodSource::sampler(source.clone()));
        }
        drop(refine);
        let original = LodSource::sampler(source);
        let inner = original.fork().unwrap_or(original);
        Some(Box::new(Sampler {
            inner,
            stride: 1,
            tiles: HashMap::new(),
            refinement: self.refinement.clone(),
            build_height: (self.layout.min_y(), self.layout.max_y()),
        }))
    }
    fn mesh(&self, section: i64, source: &FullDataSource, quads: &[Quad], debug: bool) -> SectionMesh {
        let mut mesh = SectionMesh::default();
        let detail = usize::from(pos::detail_level(section) - pos::SECTION_MINIMUM_DETAIL_LEVEL);
        let debug_color = dh::DEBUG_DETAIL_LEVEL_COLORS[detail.min(dh::DEBUG_DETAIL_LEVEL_COLORS.len() - 1)];
        for q in quads {
            let (biome, state) = source.mapping.get(q.id);
            let info = self.context.biomes.get(biome);
            // Vanilla's directional shade (`ClientLevel.getShade`).
            let shade = match q.dir {
                Dir::Up => 1.0,
                Dir::Down => 0.5,
                Dir::North | Dir::South => 0.8,
                Dir::West | Dir::East => 0.6,
            };
            let light = u16::from(q.sky) << 4 | u16::from(q.block) << 12;
            for style in self.styles(state, q.dir).into_iter().flatten() {
                let rgb = if debug {
                    debug_color
                } else {
                    match style.tint {
                        Tint::None => 0xFF_FFFF,
                        Tint::Grass => {
                            let [x, z] = q.horizontal_center(section);
                            self.context.biomes.grass(biome, x, z)
                        }
                        Tint::Foliage => info.foliage,
                        Tint::Dry => info.dry_foliage,
                        Tint::Water => info.water,
                        Tint::Constant(c) => c,
                    }
                };
                let channel = |shift: u32| (((rgb >> shift) & 255) as f32 * shade) as u32;
                let color = channel(16) | channel(8) << 8 | channel(0) << 16 | 0xFF00_0000;
                // Keep the same fluid-face material tags as detailed chunks;
                // Horizon shader packs otherwise treat distant water as solid.
                let color = if style.fluid {
                    fluid_face(color, direction(q.dir) as usize)
                } else {
                    color
                };
                let (plane, rect) = q.geometry();
                let face = Face {
                    dir: q.dir,
                    plane,
                    layer: style.layer,
                    light,
                    color,
                    translucent: style.translucent || q.transparent,
                    fluid: style.fluid,
                };
                emit(&mut mesh, face, rect);
            }
        }
        mesh.sort_solid();
        mesh
    }
}

struct Face {
    dir: Dir,
    plane: f32,
    layer: u16,
    light: u16,
    color: u32,
    translucent: bool,
    fluid: bool,
}

/// The widest face in blocks: its tiled UVs must stay within
/// `TerrainVertex::tiled`'s range (Horizon's `lod::mesh::MAX_FACE`).
const MAX_FACE: f32 = 1024.0;

fn emit(mesh: &mut SectionMesh, face: Face, [u0, v0, u1, v1]: [f32; 4]) {
    let mut u = u0;
    while u < u1 {
        let u_end = u1.min(u + MAX_FACE);
        let mut v = v0;
        while v < v1 {
            let v_end = v1.min(v + MAX_FACE);
            emit_quad(mesh, &face, [u, v, u_end, v_end]);
            v = v_end;
        }
        u = u_end;
    }
}

/// One quad as Horizon's far terrain writes them (`lod::mesh::emit_quad`):
/// the same corner order and winding, and block-tiled UVs counted from the
/// face's corner.
fn emit_quad(mesh: &mut SectionMesh, face: &Face, [u0, v0, u1, v1]: [f32; 4]) {
    let p = face.plane;
    let mut points = match face.dir {
        Dir::Up | Dir::Down => [[u0, p, v1], [u1, p, v1], [u1, p, v0], [u0, p, v0]],
        Dir::North | Dir::South => [[u0, v1, p], [u1, v1, p], [u1, v0, p], [u0, v0, p]],
        Dir::West | Dir::East => [[p, v1, u1], [p, v1, u0], [p, v0, u0], [p, v0, u1]],
    };
    if matches!(face.dir, Dir::Down | Dir::South | Dir::East) {
        points.reverse();
    }
    let uvs = points.map(|pos| match face.dir {
        // Still fluid uses +Z, as the near mesher does. Mirroring its V
        // coordinate changed the texture phase at the near/far boundary.
        Dir::Up if face.fluid => [pos[0], pos[2]],
        Dir::Up | Dir::Down => [pos[0], -pos[2]],
        Dir::North | Dir::South => [pos[0], -pos[1]],
        Dir::West | Dir::East => [pos[2], -pos[1]],
    });
    let corner = [0, 1].map(|i| uvs.iter().map(|uv| uv[i]).fold(f32::INFINITY, f32::min).floor());
    let out = if face.translucent {
        &mut mesh.translucent
    } else {
        &mut mesh.solid
    };
    out.extend(points.into_iter().zip(uvs).map(|(pos, uv)| {
        TerrainVertex::tiled(
            pos,
            [uv[0] - corner[0], uv[1] - corner[1]],
            face.layer,
            face.color,
            face.light,
        )
    }));
}

/// A section's far-terrain tile: `TileKey` level `l` is `16 << l` blocks,
/// a section of detail `d` is `2^d`.
fn tile(section: i64) -> TileKey {
    TileKey {
        x: pos::x(section),
        z: pos::z(section),
        level: pos::detail_level(section) - 4,
    }
}

/// The world, dimension and connection a session is for.
#[derive(Clone, PartialEq)]
struct WorldKey {
    root: PathBuf,
    dimension: &'static str,
    remote: bool,
}

struct Session {
    key: WorldKey,
    /// Where the sections are kept; none: in memory.
    dir: Option<PathBuf>,
    last_generator: Instant,
    engine: Engine<HorizonHost>,
    /// Loaded chunks put in the LODs, and when.
    seen: HashMap<(i32, i32), Instant>,
    pending: Vec<(i32, i32)>,
    last_scan: Option<Instant>,
    near_revision: Option<(u64, u64)>,
    loaded: HashSet<(i32, i32)>,
}

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
    static RETIREMENT: RefCell<Option<Retirement<Session>>> = const { RefCell::new(None) };
    static RETIREMENT_ERROR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The settings, read once and kept up to date by the settings page.
    static SETTINGS: RefCell<Option<Settings>> = const { RefCell::new(None) };
    /// The LOD render distance in chunks while a session runs, else 0.
    static RADIUS: std::cell::Cell<i32> = const { std::cell::Cell::new(0) };
    static NOTICE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(super) fn settings(game: &Game<'_>) -> Settings {
    SETTINGS.with_borrow_mut(|s| *s.get_or_insert_with(|| Settings::read(|k| game.native_setting(dh::ID, k))))
}

pub(super) fn set_settings(game: &Game<'_>, settings: Settings) {
    for (key, value) in settings.stored() {
        game.native_set_setting(dh::ID, key, &value);
    }
    SETTINGS.with_borrow_mut(|s| *s = Some(settings));
}

fn work_settings(game: &Game<'_>, settings: &Settings) -> WorkSettings {
    let dimension = game.dim.id();
    WorkSettings {
        render: RenderSettings {
            // `!levelWrapper.hasCeiling() && !isTheEnd()`.
            cave_culling: settings.cave_culling && dimension == "minecraft:overworld",
            cave_culling_height: 60,
            min_y: game.world.layout().min_y(),
        },
        generation: settings.generation,
        debug: settings.debug,
    }
}

/// The world generator: on the single player server thread when there is
/// one; a server the player joined has none here.
fn generator(game: &Game<'_>) -> Option<Arc<dyn LodSource>> {
    let dim = game.dim;
    match (&game.integrated, game.server_far_terrain(), &game.remote) {
        (Some(server), ..) => server.call(move |g| g.lod_source_in(dim)).flatten(),
        (None, Some(offer), Some(remote)) => {
            let states = game.res.facts().states.len() as u32;
            let biomes = game.res.biomes().index.clone();
            Some(remote.session.far_terrain(offer, biomes, states))
        }
        (None, _, None) => Some(game.streaming.lod_source()),
        _ => None,
    }
}

fn start(game: &mut Game<'_>, key: WorldKey, settings: &Settings) -> Session {
    let facts = game.res.facts();
    let blocks = block_table(facts);
    let fingerprint = fingerprint(facts);
    // The generator lives with the world: on the single player server
    // thread when there is one. A server the player joined has none here.
    let source = generator(game);
    // In single player the world's folder is the server's: the window's own
    // save folder is a placeholder.
    let root = match &game.integrated {
        Some(server) => server.call(|g| g.save.root.clone()),
        None if game.remote.is_none() => Some(key.root.clone()),
        None => None,
    };
    let dir = root.filter(|_| !key.remote).map(|root| {
        let dimension = key.dimension.strip_prefix("minecraft:").unwrap_or(key.dimension);
        root.join("data").join("DistantHorizons").join(dimension)
    });
    let host = HorizonHost {
        layout: game.world.layout(),
        context: game.pack.context.clone(),
        blocks,
        source: Mutex::new(source),
        styles: Mutex::new(HashMap::new()),
        refinement: Arc::new(Mutex::new(Refinement {
            enabled: !key.remote,
            ..Refinement::default()
        })),
    };
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    let engine = Engine::new(
        host,
        dir.clone(),
        fingerprint,
        work_settings(game, settings),
        dh::engine::worker_threads(cores).min(2),
    );
    match &dir {
        Some(dir) => log::info!("Distant Horizons: started, keeping LODs in {}", dir.display()),
        None => log::info!("Distant Horizons: started, keeping LODs in memory"),
    }
    Session {
        key,
        dir,
        engine,
        seen: HashMap::new(),
        last_generator: Instant::now(),
        pending: Vec::new(),
        last_scan: None,
        near_revision: None,
        loaded: HashSet::new(),
    }
}

/// Takes the old session's tiles out on the renderer's thread. Its worker join,
/// pending reductions and cache save are separate from this visible cutover.
fn clear_session(game: &mut Game<'_>, session: &Session) {
    // Release network waits before Engine::drop joins its workers.
    if session.key.remote {
        if let Some(remote) = &game.remote {
            remote.session.cancel_far_terrain();
        }
    }
    if let Some(renderer) = &mut game.renderer {
        // Queued removals only start the renderer's fade. No DH frame runs
        // after shutdown to advance that fade or drain pending uploads.
        // DH owns far terrain while native LOD is disabled: cancel pending
        // uploads and release both visible and fading tiles immediately.
        let shown = session.engine.shown_sections().count();
        renderer.clear_lods();
        log::debug!("Distant Horizons: cleared renderer ({shown} registered sections)");
    }
    RADIUS.set(0);
}

fn retirement_ready() -> bool {
    if RETIREMENT_ERROR.get() {
        return false;
    }
    RETIREMENT.with_borrow_mut(|slot| {
        if slot.is_none() {
            match Retirement::new() {
                Ok(worker) => *slot = Some(worker),
                Err(error) => {
                    RETIREMENT_ERROR.set(true);
                    log::error!("Distant Horizons: cannot start cache-retirement worker: {error}");
                    return false;
                }
            }
        }
        let worker = slot.as_ref().expect("retirement worker initialized");
        if worker.failed() {
            RETIREMENT_ERROR.set(true);
            log::error!("Distant Horizons: cache retirement failed; restart the client before enabling DH again");
        }
        worker.ready()
    })
}

fn stop(game: &mut Game<'_>, session: Session) {
    clear_session(game, &session);
    let result = RETIREMENT.with_borrow_mut(|slot| {
        slot.as_mut().expect("retirement created before live session").try_retire(session)
    });
    if let Err(session) = result {
        // Do not turn an exceptional enqueue failure into the same frame stall.
        // Keep ownership until the final exit hook, with DH disabled meanwhile.
        RETIREMENT_ERROR.set(true);
        SESSION.set(Some(session));
        log::error!("Distant Horizons: could not retire cache; holding it safely until client exit");
    }
}

/// End workers while Windows threads are still alive. A thread-local Session
/// left to process teardown can otherwise join workers after ExitProcess has
/// terminated them, producing std's unexpected-thread-termination panic.
pub(super) fn shutdown(game: &mut Game<'_>) {
    let session = SESSION.take();
    if let Some(session) = &session {
        clear_session(game, session);
    }
    // Game::shut_down runs after the event loop, not during an off/on frame.
    // Complete every accepted save before resources and OS threads disappear.
    if let Some(mut worker) = RETIREMENT.take()
        && !worker.finish()
    {
        log::error!("Distant Horizons: cache retirement did not complete cleanly");
    }
    drop(session);
}

/// Once a frame (`native_frame`).
pub(super) fn frame(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    if RETIREMENT_ERROR.get() {
        return;
    }
    let settings = settings(game);
    let in_world = game.renderer.is_some() && game.player.is_some() && game.title.is_none();
    // Horizon's own far terrain (Video Settings > LOD Distance) draws in the
    // same place: Distant Horizons waits while it is on.
    let builtin = game.settings.horizon.lod_distance > 0;
    if builtin && in_world && settings.enabled && !NOTICE.get() {
        NOTICE.set(true);
        game.native_chat("Distant Horizons is paused while Video Settings > LOD Distance is on.");
    }
    let active = game.native_on(dh::ID) && settings.enabled && in_world && !builtin;
    let key = WorldKey {
        root: game.save.root.clone(),
        dimension: game.dim.id(),
        remote: game.remote.is_some() && game.integrated.is_none(),
    };
    let mut session = SESSION.take();
    if let Some(s) = session.take_if(|s| !active || s.key != key) {
        stop(game, s);
    }
    if !active {
        return;
    }
    if !retirement_ready() {
        // A previous world's immutable owned session may still be saving. Do
        // not start another writer (or discard an exceptional retained session).
        if let Some(session) = session {
            SESSION.set(Some(session));
        }
        return;
    }
    let mut session = session.unwrap_or_else(|| start(game, key, &settings));
    session.engine.set_settings(work_settings(game, &settings));
    RADIUS.set(game.server_far_terrain().map_or(settings.radius_chunks(), |o| {
        settings.radius_chunks().min(o.distance as i32)
    }));
    let host = session.engine.host();
    if settings.generation
        && host.source.lock().unwrap().is_none()
        && session.last_generator.elapsed() > Duration::from_secs(2)
    {
        session.last_generator = Instant::now();
        *host.source.lock().unwrap() = generator(game);
    }
    scan_chunks(game, &mut session);
    let Some(player) = game.native_player() else {
        SESSION.set(Some(session));
        return;
    };
    let renderer = game.renderer.as_ref().expect("active DH renderer");
    let revision = (game.world.chunks.membership_revision(), renderer.lod_detail_revision());
    if session.near_revision != Some(revision) {
        session.near_revision = Some(revision);
        // Data can arrive well before its GPU mesh. Keep far coverage until
        // every detailed section is uploaded, using the renderer's same API
        // as the built-in LOD handoff (not CPU chunk membership alone).
        session.loaded = renderer
            .lod_detail_columns()
            .filter(|key| game.world.chunks.contains_key(key))
            .collect();
    }
    let near = Near {
        center: game.streaming.center,
        radius: game.streaming.radius,
        loaded: session.loaded.clone(),
    };
    let actual_near = game.remote.as_ref().map_or(game.streaming.radius, |remote| {
        game.streaming.radius.min(remote.sent_radius())
    });
    let plan = DetailPlan::new(settings.quality, RADIUS.get()).with_near_radius(actual_near);
    let (meshes, removed) = session.engine.update([player.x, player.z], plan, near);
    let cutout = session.engine.take_cutout().map(|columns| columns.into_iter().collect());
    if let Some(renderer) = &mut game.renderer {
        let mut tiles = Vec::with_capacity(meshes.len());
        let mut gone: Vec<TileKey> = removed.into_iter().map(tile).collect();
        for m in meshes {
            match m.mesh {
                Some(mesh) => tiles.push((tile(m.section), mesh)),
                None => gone.push(tile(m.section)),
            }
        }
        // Commit holes only with their replacement meshes: departing detailed
        // chunks must survive a slow far remesh after a view-distance change.
        renderer.queue_lods(tiles, gone, cutout);
        renderer.upload_lods(UPLOAD_BUDGET);
    }
    SESSION.set(Some(session));
}

/// New and changed loaded chunks into the LODs, a few a frame.
fn scan_chunks(game: &mut Game<'_>, session: &mut Session) {
    let now = Instant::now();
    if session.pending.is_empty() && session.last_scan.is_none_or(|at| now - at >= SCAN_EVERY) {
        session.last_scan = Some(now);
        let chunks = &game.world.chunks;
        session.seen.retain(|k, _| chunks.contains_key(k));
        for (&key, chunk) in chunks.iter() {
            // Light must be in: the LODs keep each run's light.
            if !chunk.lit {
                continue;
            }
            let changed = match session.seen.get(&key) {
                None => true,
                // Read again when its blocks changed (the game remeshes
                // them), at most every few seconds. Its data's address also
                // moves with light updates alone: not a reason.
                Some(&at) => {
                    now - at >= REREAD_EVERY
                        && (0..chunk.sections.len() as i32)
                            .any(|sy| game.world.dirty_sections.contains(&(key.0, sy, key.1)))
                }
            };
            if changed {
                session.seen.insert(key, now);
                session.pending.push(key);
            }
        }
        // Nearest first.
        let (cx, cz) = game.streaming.center;
        session
            .pending
            .sort_unstable_by_key(|&(x, z)| std::cmp::Reverse((x - cx).pow(2) + (z - cz).pow(2)));
    }
    let air = game.res.facts().air.0;
    for _ in 0..CHUNKS_PER_FRAME {
        let Some(key) = session.pending.pop() else { break };
        if let Some(chunk) = game.world.chunks.get(&key) {
            session.engine.add_chunk(Box::new(LoadedChunk {
                chunk: chunk.clone(),
                air,
            }));
        }
    }
}

/// `terrain_distance` as the renderer uses it: far enough for the LODs
/// while a session runs.
pub(super) fn terrain_distance(value: i32) -> i32 {
    value.max(RADIUS.get())
}

pub(super) fn render_fog(game: &Game<'_>, kind: horizon_client::fog::FogType, fog: &mut horizon_client::fog::FogData) {
    if RADIUS.get() > 0
        && game.dim.id() == "minecraft:overworld"
        && kind == horizon_client::fog::FogType::Atmospheric
        && game.potions.effects.get("blindness").is_none()
        && game.potions.effects.get("darkness").is_none()
    {
        (fog.environmental_start, fog.environmental_end) =
            dh::distant_atmosphere(RADIUS.get(), fog.environmental_start, fog.environmental_end);
    }
}

/// `tree` with `/dh` and its words (`distant_horizons::COMMANDS`), for the
/// chat's completion. Called when Horizon builds the connection's chat tree,
/// not every frame. The existing native_command hook executes the command.
/// Keep a server-owned command of the same name unchanged.
pub(super) fn with_commands(tree: &horizon_client::command::Dispatcher) -> Option<horizon_client::command::Dispatcher> {
    use horizon_client::command::MOD_COMMAND;
    if tree.find("dh").is_some() && tree.find("distanthorizons").is_some() {
        return None;
    }
    let mut d = tree.clone();
    for alias in ["dh", "distanthorizons"] {
        if d.find(alias).is_some() {
            continue;
        }
        let root = d.literal(0, alias);
        d.executes(root, MOD_COMMAND);
        for &(word, values) in dh::COMMANDS {
            let node = d.literal(root, word);
            if values.is_empty() {
                d.executes(node, MOD_COMMAND);
            }
            for value in values {
                let leaf = d.literal(node, value);
                d.executes(leaf, MOD_COMMAND);
            }
        }
    }
    Some(d)
}

/// `/dh`: the status, or a setting changed.
pub(super) fn command(game: &mut Game<'_>, command: &str) -> bool {
    let mut words = command.split_whitespace();
    if !matches!(words.next(), Some("dh" | "distanthorizons")) {
        return false;
    }
    let args: Vec<&str> = words.collect();
    let mut settings = settings(game);
    let reply = match args.as_slice() {
        [] | ["status"] => {
            let session = SESSION.with_borrow(|s| {
                s.as_ref().map(|s| {
                    let generator = s.engine.host().source.lock().unwrap().is_some();
                    (s.engine.stats(), generator, s.dir.clone())
                })
            });
            match session {
                Some((s, generator, dir)) => format!(
                    "Distant Horizons {}: {} of {} sections drawn ({} faces), {} jobs queued, {} in memory, {} generated, {} chunks read, {} refined sections; generator {}; near {} chunks around {:?}; LODs {}",
                    dh::VERSION,
                    s.shown,
                    s.wanted,
                    s.quads,
                    s.queued,
                    s.loaded,
                    s.generated,
                    s.chunks,
                    s.refined,
                    if generator { "on" } else { "none" },
                    game.streaming.radius,
                    game.streaming.center,
                    if dir.is_some() {
                        "saved in the world"
                    } else {
                        "in memory"
                    },
                ),
                None => format!("Distant Horizons {}: not running here", dh::VERSION),
            }
        }
        ["save"] => {
            SESSION.with_borrow(|s| s.as_ref().map(|s| s.engine.save()));
            "Distant Horizons: saved".to_owned()
        }
        ["clear"] => {
            SESSION.with_borrow_mut(|s| s.as_mut().map(|s| s.engine.wipe()));
            "Distant Horizons: LODs cleared; they are made again".to_owned()
        }
        _ => match settings.command(&args) {
            Some(reply) => {
                set_settings(game, settings);
                reply
            }
            None => format!(
                "/dh [{}]",
                dh::COMMANDS
                    .iter()
                    .map(|(w, v)| if v.is_empty() {
                        (*w).to_owned()
                    } else {
                        format!("{w} {}", v.join("|"))
                    })
                    .collect::<Vec<_>>()
                    .join(" | ")
            ),
        },
    };
    game.native_chat(&reply);
    true
}

/// The debug box: shown with the debug detail colours.
pub(super) fn hud(
    game: &mut Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    use horizon_client::extras::Corner;
    if !game.native_on(dh::ID) || !settings(game).debug || !game.native_corner_shown(Corner::BottomLeft) {
        return;
    }
    let Some(s) = SESSION.with_borrow(|s| s.as_ref().map(|s| s.engine.stats())) else {
        return;
    };
    let lines = [
        format!("DH {}: {}/{} sections", dh::VERSION, s.shown, s.wanted),
        format!("{} queued, {} generated, {} chunks", s.queued, s.generated, s.chunks),
    ];
    let w = lines.iter().map(|l| game.native_text_width(l)).fold(0.0, f32::max) + 4.0;
    let h = 10.0 * lines.len() as f32 + 2.0;
    let (x, y) = corners.place(Corner::BottomLeft, w, h);
    game.native_fill(batch, [x, y, w, h], 0x7800_0000);
    for (i, line) in lines.iter().enumerate() {
        game.native_text(batch, line, [x + 2.0, y + 2.0 + 10.0 * i as f32], 0xE0E0E0, true);
    }
}
