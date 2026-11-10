// Distant Horizons for Horizon Client: the level-of-detail core of the
// Distant Horizons mod (https://gitlab.com/distant-horizons-team/distant-horizons,
// LGPL-3.0, Copyright (C) 2020 James Seibel and contributors), ported to
// Rust. This file is both the mod's Cargo library (its tests run with
// `cargo test`) and the module the bundle adds to Horizon's game as
// `game::distant_horizons`. It knows nothing of Horizon: chunks, the world
// generator and meshing reach it through the `ChunkView`, `ColumnSampler`
// and `Host` traits, which `bridge.rs` implements on Horizon's types.
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU Lesser General Public License as published by
// the Free Software Foundation, version 3. It is distributed WITHOUT ANY
// WARRANTY; see LICENSE for the full terms.
//
// The Java class each part ports is named on it. Where the port differs from
// Distant Horizons, the comment there says how and why.

/// The mod's id (its settings folder), name and version.
pub const ID: &str = "distant_horizons";
pub const NAME: &str = "Distant Horizons";
pub const VERSION: &str = "0.1.0+dormyu.17";
pub const DESCRIPTION: &str = "Distant Horizons' level-of-detail terrain, ported to Rust: the world past your render distance, built from the chunks you visit and the world generator, kept on disk and drawn at lower detail the farther it is.";

/// Horizon's default clear atmosphere ends at 1024 blocks unless its built-in
/// LOD is active. DH owns that view instead; extend only that default range.
pub fn distant_atmosphere(radius_chunks: i32, start: f32, end: f32) -> (f32, f32) {
    if (1023.9..=1024.1).contains(&end) {
        let scale = (radius_chunks as f32 * 16.0 / 1024.0).max(1.0);
        (start * scale, end * scale)
    } else {
        (start, end)
    }
}

#[cfg(test)]
mod atmosphere_tests {
    #[test]
    fn distant_mountains_remain_visible_and_special_fog_keeps_its_range() {
        assert_eq!(super::distant_atmosphere(256, 256.0, 1024.0), (1024.0, 4096.0));
        assert_eq!(super::distant_atmosphere(64, 256.0, 1024.0), (256.0, 1024.0));
        assert_eq!(super::distant_atmosphere(256, 0.0, 96.0), (0.0, 96.0));
        assert_eq!(super::distant_atmosphere(256, 0.0, 512.0), (0.0, 512.0));
        assert_eq!(super::distant_atmosphere(256, 1e9, 2e9), (1e9, 2e9));
    }
}

/// `DhSectionPos`: a quadtree node, packed in a long as Distant Horizons
/// does (detail level in bits 0-7, x in 8-35, z in 36-63). A section at
/// detail level `d` is `2^d` blocks wide and holds 64 x 64 data columns of
/// `2^(d-6)` blocks each.
#[allow(dead_code)]
pub mod pos {
    pub const SECTION_MINIMUM_DETAIL_LEVEL: u8 = 6;
    /// `SECTION_MINIMUM_DETAIL_LEVEL + LodUtil.BLOCK_DETAIL_LEVEL`.
    pub const SECTION_BLOCK_DETAIL_LEVEL: u8 = 6;
    /// `FullDataSourceProviderV2.ROOT_SECTION_DETAIL_LEVEL`: 16384 blocks.
    pub const ROOT_DETAIL_LEVEL: u8 = SECTION_MINIMUM_DETAIL_LEVEL + 8;
    /// `FullDataSourceV2.WIDTH`: data columns per side.
    pub const WIDTH: i32 = 64;

    const DETAIL_LEVEL_MASK: i64 = i8::MAX as i64;
    const POS_X_OFFSET: u32 = 8;
    const POS_Z_OFFSET: u32 = 8 + 28;
    const POS_MASK: i32 = (1 << 28) - 1;
    const POS_MISSING_WIDTH: u32 = 32 - 28;

    pub fn encode(detail_level: u8, x: i32, z: i32) -> i64 {
        let mut data = 0i64;
        data |= i64::from(detail_level) & DETAIL_LEVEL_MASK;
        data |= i64::from(x & POS_MASK) << POS_X_OFFSET;
        data |= i64::from(z & POS_MASK) << POS_Z_OFFSET;
        data
    }
    pub fn detail_level(pos: i64) -> u8 {
        (pos & DETAIL_LEVEL_MASK) as u8
    }
    pub fn x(pos: i64) -> i32 {
        let x = ((pos >> POS_X_OFFSET) as i32) & POS_MASK;
        (x << POS_MISSING_WIDTH) >> POS_MISSING_WIDTH
    }
    pub fn z(pos: i64) -> i32 {
        let z = ((pos >> POS_Z_OFFSET) as i32) & POS_MASK;
        (z << POS_MISSING_WIDTH) >> POS_MISSING_WIDTH
    }
    /// The section at `detail_level` holding block column `(x, z)`
    /// (`encodeContaining`).
    pub fn containing_block(detail_level: u8, x: i32, z: i32) -> i64 {
        let block = encode(SECTION_BLOCK_DETAIL_LEVEL, x.div_euclid(WIDTH), z.div_euclid(WIDTH));
        convert_to_detail_level(block, detail_level)
    }
    pub fn convert_to_detail_level(pos: i64, new_detail_level: u8) -> i64 {
        let detail = detail_level(pos);
        let (mut x, mut z) = (x(pos), z(pos));
        if new_detail_level >= detail {
            let scale = 1 << (new_detail_level - detail);
            x = x.div_euclid(scale);
            z = z.div_euclid(scale);
        } else {
            let scale = 1 << (detail - new_detail_level);
            x *= scale;
            z *= scale;
        }
        encode(new_detail_level, x, z)
    }
    pub fn block_width(pos: i64) -> i32 {
        1 << detail_level(pos)
    }
    /// Blocks per data column.
    pub fn data_width(pos: i64) -> i32 {
        1 << (detail_level(pos) - SECTION_MINIMUM_DETAIL_LEVEL)
    }
    fn center_block(pos: i64, coordinate: i32) -> i32 {
        let detail = detail_level(pos);
        if detail == 0 {
            return coordinate;
        }
        let offset = if detail != 1 { 1 << (detail - 1) } else { 0 };
        coordinate * (1 << detail) + offset
    }
    pub fn center_block_x(pos: i64) -> i32 {
        center_block(pos, x(pos))
    }
    pub fn center_block_z(pos: i64) -> i32 {
        center_block(pos, z(pos))
    }
    pub fn min_corner_block_x(pos: i64) -> i32 {
        let half = if detail_level(pos) != 1 {
            block_width(pos) / 2
        } else {
            0
        };
        center_block_x(pos) - half
    }
    pub fn min_corner_block_z(pos: i64) -> i32 {
        let half = if detail_level(pos) != 1 {
            block_width(pos) / 2
        } else {
            0
        };
        center_block_z(pos) - half
    }
    pub fn parent(pos: i64) -> i64 {
        encode(detail_level(pos) + 1, x(pos) >> 1, z(pos) >> 1)
    }
    /// Child 0 to 3: bit 0 is +x, bit 1 is +z (`getChildByIndex`).
    pub fn child(pos: i64, index: usize) -> i64 {
        let index = index as i32;
        encode(
            detail_level(pos) - 1,
            x(pos) * 2 + (index & 1),
            z(pos) * 2 + ((index & 2) >> 1),
        )
    }
    pub fn adjacent(pos: i64, dx: i32, dz: i32) -> i64 {
        encode(detail_level(pos), x(pos) + dx, z(pos) + dz)
    }
    /// Whether `a` covers `b`'s minimum corner (`contains`).
    pub fn contains(a: i64, b: i64) -> bool {
        let (ax, az) = (min_corner_block_x(a), min_corner_block_z(a));
        let (bx, bz) = (min_corner_block_x(b), min_corner_block_z(b));
        let width = block_width(a) - 1;
        ax <= bx && bx <= ax + width && az <= bz && bz <= az + width
    }
    /// `getChebyshevSignedBlockDistance`: negative inside the section.
    pub fn chebyshev_signed_block_distance(pos: i64, x: i32, z: i32) -> i32 {
        (center_block_x(pos) - x).abs().max((center_block_z(pos) - z).abs()) - block_width(pos) / 2
    }
    pub fn to_string(pos: i64) -> String {
        format!("{}*{},{}", detail_level(pos), x(pos), z(pos))
    }
}

/// `FullDataPointUtil`: one vertical run of a full data column in a long.
/// Bits 0-31 the id (in the source's `IdMap`), 32-43 the height, 44-55 the
/// bottom relative to the world's minimum y, 56-59 sky light, 60-63 block
/// light.
#[allow(dead_code)]
pub mod full_point {
    pub const EMPTY_DATA_POINT: u64 = 0;
    const ID_MASK: u64 = i32::MAX as u64;
    const HEIGHT_OFFSET: u32 = 32;
    const MIN_Y_OFFSET: u32 = 44;
    const SKY_LIGHT_OFFSET: u32 = 56;
    const BLOCK_LIGHT_OFFSET: u32 = 60;
    const HEIGHT_MASK: u64 = (1 << 12) - 1;
    const MIN_Y_MASK: u64 = (1 << 12) - 1;
    const LIGHT_MASK: u64 = 15;
    /// `RenderDataPointUtil.MAX_WORLD_Y_SIZE`.
    pub const MAX_WORLD_Y_SIZE: i32 = 2031 + 4064;

    /// `encode` with `validateData`: Distant Horizons throws
    /// `DataCorruptedException`, this returns `None`.
    pub fn encode(id: u32, height: i32, rel_min_y: i32, block_light: u8, sky_light: u8) -> Option<u64> {
        let valid = id <= i32::MAX as u32
            && (0..MAX_WORLD_Y_SIZE).contains(&rel_min_y)
            && height > 0
            && height < MAX_WORLD_Y_SIZE
            && rel_min_y + height <= MAX_WORLD_Y_SIZE
            && block_light <= 15
            && sky_light <= 15;
        valid.then(|| {
            (u64::from(id) & ID_MASK)
                | ((height as u64 & HEIGHT_MASK) << HEIGHT_OFFSET)
                | ((rel_min_y as u64 & MIN_Y_MASK) << MIN_Y_OFFSET)
                | (u64::from(block_light) << BLOCK_LIGHT_OFFSET)
                | (u64::from(sky_light) << SKY_LIGHT_OFFSET)
        })
    }
    pub fn id(data: u64) -> u32 {
        (data & ID_MASK) as u32
    }
    pub fn height(data: u64) -> i32 {
        ((data >> HEIGHT_OFFSET) & HEIGHT_MASK) as i32
    }
    pub fn bottom_y(data: u64) -> i32 {
        ((data >> MIN_Y_OFFSET) & MIN_Y_MASK) as i32
    }
    pub fn top_y(data: u64) -> i32 {
        bottom_y(data) + height(data)
    }
    pub fn block_light(data: u64) -> u8 {
        ((data >> BLOCK_LIGHT_OFFSET) & LIGHT_MASK) as u8
    }
    pub fn sky_light(data: u64) -> u8 {
        ((data >> SKY_LIGHT_OFFSET) & LIGHT_MASK) as u8
    }
    pub fn set_id(data: u64, id: u32) -> u64 {
        (data & !ID_MASK) | u64::from(id)
    }
    pub fn set_height(data: u64, height: i32) -> u64 {
        (data & !(HEIGHT_MASK << HEIGHT_OFFSET)) | ((height as u64 & HEIGHT_MASK) << HEIGHT_OFFSET)
    }
    pub fn set_bottom_y(data: u64, bottom: i32) -> u64 {
        (data & !(MIN_Y_MASK << MIN_Y_OFFSET)) | ((bottom as u64 & MIN_Y_MASK) << MIN_Y_OFFSET)
    }
}

/// `EDhApiWorldGenerationStep`: how complete a column's data is. Newer data
/// replaces older only when it is at least as complete.
#[allow(dead_code)]
pub mod gen_step {
    pub const DOWN_SAMPLED: i8 = -1;
    pub const EMPTY: i8 = 0;
    pub const SURFACE: i8 = 5;
    pub const FEATURES: i8 = 8;
    pub const LIGHT: i8 = 9;
}

/// `EDhApiBlockMaterial`, the render points' 4-bit material.
#[allow(dead_code)]
pub mod material {
    pub const UNKNOWN: u8 = 0;
    pub const LEAVES: u8 = 1;
    pub const STONE: u8 = 2;
    pub const LAVA: u8 = 6;
    pub const SNOW: u8 = 8;
    pub const SAND: u8 = 9;
    pub const WATER: u8 = 12;
    pub const GRASS: u8 = 13;
    pub const AIR: u8 = 14;
    pub const ILLUMINATED: u8 = 15;
}

/// What the LOD code asks of a block state (`IBlockStateWrapper`), read
/// from the game once per state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlockInfo {
    pub air: bool,
    /// 0 to 15, 16 for fully opaque (`LodUtil.BLOCK_FULLY_OPAQUE`).
    pub opacity: u8,
    /// Has a collision shape.
    pub solid: bool,
    pub liquid: bool,
    pub material: u8,
    /// 255, or less for a see-through block (water, ice, glass).
    pub alpha: u8,
    /// A snow layer: drawn as the colour of the block under it.
    pub snow_layer: bool,
    /// Frozen water: water directly under it is drawn as ice.
    pub ice: bool,
    /// A native log/wood column whose one-block footprint must survive reduction.
    pub stem: bool,
}

/// Every block state's `BlockInfo`, by state id. Unknown ids are air.
#[derive(Clone, Debug, Default)]
pub struct BlockTable {
    pub states: Vec<BlockInfo>,
}

impl BlockTable {
    pub fn get(&self, state: u32) -> BlockInfo {
        self.states.get(state as usize).copied().unwrap_or(BlockInfo {
            air: true,
            material: material::AIR,
            ..BlockInfo::default()
        })
    }
}

/// `FullDataPointIdMap`: the (biome, block state) pairs a data source uses;
/// data points name them by index.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IdMap {
    entries: Vec<(u16, u32)>,
    index: std::collections::HashMap<(u16, u32), u32>,
}

#[allow(dead_code)]
impl IdMap {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn add_if_not_present(&mut self, biome: u16, state: u32) -> u32 {
        if let Some(&id) = self.index.get(&(biome, state)) {
            return id;
        }
        let id = self.entries.len() as u32;
        self.entries.push((biome, state));
        self.index.insert((biome, state), id);
        id
    }
    /// The (biome, block state) of `id`; air in biome 0 for an id out of
    /// range (Distant Horizons logs it once and skips the point).
    pub fn get(&self, id: u32) -> (u16, u32) {
        self.entries.get(id as usize).copied().unwrap_or((0, 0))
    }
    pub fn state(&self, id: u32) -> u32 {
        self.get(id).1
    }
    /// `mergeAndReturnRemappedEntityIds`: adds `other`'s entries, and gives
    /// the id each of its ids has here.
    pub fn merge_and_remap(&mut self, other: &IdMap) -> Vec<u32> {
        other
            .entries
            .iter()
            .map(|&(biome, state)| self.add_if_not_present(biome, state))
            .collect()
    }
    pub fn entries(&self) -> &[(u16, u32)] {
        &self.entries
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
    }
}

/// `FullDataSourceV2`: a section's 64 x 64 full data columns. Each column
/// lists its runs from the top down (index 0 is the highest), as Distant
/// Horizons keeps them.
#[derive(Clone, Debug, PartialEq)]
pub struct FullDataSource {
    pub pos: i64,
    pub mapping: IdMap,
    /// By `index(x, z)`; an empty column has no data.
    pub columns: Vec<Vec<u64>>,
    /// Each column's `gen_step`.
    pub gen_steps: Vec<i8>,
    /// Sparse exact footprints, independent of the terrain cell's width.
    pub stems: Vec<Stem>,
    pub is_empty: bool,
}

/// Renderer-only one-block stem, with world X/Z and build-relative Y.
/// Four adjacent records retain a real 2x2 trunk; reduction never widens them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Stem {
    pub x: i32,
    pub z: i32,
    pub min_y: u16,
    pub max_y: u16,
    pub id: u32,
    pub sky: u8,
    pub block: u8,
    /// Zero for exact native logs. Predicted roots require this surviving crown.
    pub leaves: u32,
    pub crown_max: u16,
}

const STEM_MAX_WIDTH: i32 = 16;
const STEMS_PER_COLUMN: usize = 4;

/// `FullDataSourceV2.relativePosToIndex`.
pub fn index(x: i32, z: i32) -> usize {
    debug_assert!((0..pos::WIDTH).contains(&x) && (0..pos::WIDTH).contains(&z));
    (x * pos::WIDTH + z) as usize
}

const COLUMNS: usize = (pos::WIDTH * pos::WIDTH) as usize;

#[allow(dead_code)]
impl FullDataSource {
    pub fn empty(pos: i64) -> Self {
        Self {
            pos,
            mapping: IdMap::default(),
            columns: vec![Vec::new(); COLUMNS],
            gen_steps: vec![gen_step::EMPTY; COLUMNS],
            stems: Vec::new(),
            is_empty: true,
        }
    }
    pub fn column(&self, x: i32, z: i32) -> &[u64] {
        &self.columns[index(x, z)]
    }
    fn try_column(&self, x: i32, z: i32) -> Option<&Vec<u64>> {
        ((0..pos::WIDTH).contains(&x) && (0..pos::WIDTH).contains(&z)).then(|| &self.columns[index(x, z)])
    }
    pub fn set_column(&mut self, x: i32, z: i32, column: Vec<u64>, step: i8) {
        let i = index(x, z);
        let step = if column.is_empty() { gen_step::EMPTY } else { step };
        self.columns[i] = column;
        self.gen_steps[i] = step;
        self.is_empty &= step == gen_step::EMPTY;
    }
    fn column_is_missing(&self, i: usize) -> bool {
        self.gen_steps[i] == gen_step::EMPTY || self.columns[i].is_empty()
    }
    /// An empty vector is unknown, even if an older cache marked it complete.
    /// A known air column has an explicit encoded air run.
    pub fn missing_columns(&self) -> usize {
        (0..COLUMNS).filter(|&i| self.column_is_missing(i)).count()
    }

    /// `updateFromDataSource`: merges `input`, of this section, one of its
    /// children or its parent. Returns whether this source changed.
    pub fn update_from(&mut self, input: &FullDataSource, blocks: &BlockTable) -> bool {
        if input.mapping.is_empty() {
            return false;
        }
        let this_detail = pos::detail_level(self.pos);
        let input_detail = pos::detail_level(input.pos);
        let remapped = self.mapping.merge_and_remap(&input.mapping);
        let mut accepted = Vec::new();
        let changed: Vec<usize> = if input_detail == this_detail {
            self.update_from_same_detail_level(input, &remapped, &mut accepted)
        } else if input_detail + 1 == this_detail {
            self.update_from_one_below_detail_level(input, &remapped, blocks, &mut accepted)
        } else if input_detail == this_detail + 1 {
            self.downsample_from_one_above_detail_level(input, &remapped, &mut accepted)
        } else {
            panic!(
                "unsupported data source update: {} into {}",
                pos::to_string(input.pos),
                pos::to_string(self.pos)
            );
        };
        let stems_changed = self.update_stems(input, &remapped, blocks, &accepted);
        if changed.is_empty() && !stems_changed {
            return false;
        }
        self.remove_unused_ids_and_remap();
        // `EDhApiWorldCompressionMode.VISUALLY_EQUAL`, the default: hidden
        // runs are merged into the run above them. Distant Horizons culls
        // every column; only the changed ones and their neighbours can change.
        let mut culled = vec![false; COLUMNS];
        for i in changed {
            let (x, z) = ((i as i32) / pos::WIDTH, (i as i32) % pos::WIDTH);
            for (dx, dz) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, nz) = (x + dx, z + dz);
                if !(0..pos::WIDTH).contains(&nx) || !(0..pos::WIDTH).contains(&nz) || culled[index(nx, nz)] {
                    continue;
                }
                culled[index(nx, nz)] = true;
                if self.column(nx, nz).len() > 1 {
                    cull_hidden_data_points(self, nx, nz, blocks);
                }
            }
        }
        true
    }

    fn update_from_same_detail_level(&mut self, input: &FullDataSource, remapped: &[u32], accepted: &mut Vec<usize>) -> Vec<usize> {
        let mut changed = Vec::new();
        for i in 0..COLUMNS {
            let this_step = if self.columns[i].is_empty() { gen_step::EMPTY } else { self.gen_steps[i] };
            let input_step = input.gen_steps[i];
            if input.columns[i].is_empty() {
                continue;
            }
            // Down-sampled input only replaces empty or down-sampled data;
            // anything else replaces data that is no more complete.
            let allowed = if input_step == gen_step::DOWN_SAMPLED {
                this_step == gen_step::EMPTY || this_step == gen_step::DOWN_SAMPLED
            } else {
                input_step != gen_step::EMPTY && this_step <= input_step
            };
            if !allowed {
                continue;
            }
            let column: Vec<u64> = input.columns[i]
                .iter()
                .map(|&p| full_point::set_id(p, remapped[full_point::id(p) as usize]))
                .collect();
            accepted.push(i);
            if self.columns[i] != column || self.gen_steps[i] != input_step {
                changed.push(i);
            }
            self.columns[i] = column;
            self.gen_steps[i] = input_step;
            self.is_empty = false;
        }
        changed
    }

    /// `updateFromOneBelowDetailLevel`: each 2 x 2 of the child's columns
    /// becomes one column here.
    ///
    /// Unlike Distant Horizons, a 2 x 2 with any empty column leaves the
    /// column here as it is: this port generates far sections at their own
    /// detail level, so a parent often has data its children lack, and a
    /// single visited chunk must not blank the generated terrain around it.
    fn update_from_one_below_detail_level(
        &mut self,
        input: &FullDataSource,
        remapped: &[u32],
        blocks: &BlockTable,
        accepted: &mut Vec<usize>,
    ) -> Vec<usize> {
        let half = pos::WIDTH / 2;
        let first = pos::child(self.pos, 0);
        let offset_x = if pos::x(input.pos) == pos::x(first) { 0 } else { half };
        let offset_z = if pos::z(input.pos) == pos::z(first) { 0 } else { half };
        let mut changed = Vec::new();
        for x in (0..pos::WIDTH).step_by(2) {
            for z in (0..pos::WIDTH).step_by(2) {
                let step = (0..4)
                    .map(|i| {
                        let i=index(x + (i & 1), z + (i >> 1));
                        if input.columns[i].is_empty() { gen_step::EMPTY } else { input.gen_steps[i] }
                    })
                    .min()
                    .unwrap_or(gen_step::EMPTY);
                if step == gen_step::EMPTY {
                    continue;
                }
                let recipient = index(x / 2 + offset_x, z / 2 + offset_z);
                // A finer preview is still speculative. Never replace visited
                // or feature-complete parent data (including removed trees)
                // with a late SURFACE child merely because its cells are smaller.
                let current_step = if self.columns[recipient].is_empty() {
                    gen_step::EMPTY
                } else { self.gen_steps[recipient] };
                if step < current_step { continue; }
                accepted.push(recipient);
                let merged: Vec<u64> = merge_two_by_two_column(input, x, z, blocks)
                    .into_iter()
                    .map(|p| full_point::set_id(p, remapped[full_point::id(p) as usize]))
                    .collect();
                if self.columns[recipient] != merged || self.gen_steps[recipient] != step {
                    changed.push(recipient);
                }
                self.columns[recipient] = merged;
                self.gen_steps[recipient] = step;
                self.is_empty = false;
            }
        }
        changed
    }

    /// `downsampleFromOneAboveDetailLevel`: fills only the columns that have
    /// no data, so higher-detail data is never covered.
    fn downsample_from_one_above_detail_level(&mut self, input: &FullDataSource, remapped: &[u32], accepted: &mut Vec<usize>) -> Vec<usize> {
        let half = pos::WIDTH / 2;
        let first = pos::child(input.pos, 0);
        let offset_x = if pos::x(self.pos) == pos::x(first) { 0 } else { half };
        let offset_z = if pos::z(self.pos) == pos::z(first) { 0 } else { half };
        let mut changed = Vec::new();
        for x in 0..pos::WIDTH {
            for z in 0..pos::WIDTH {
                let recipient = index(x, z);
                let has_data = self.columns[recipient]
                    .iter()
                    .any(|&p| p != full_point::EMPTY_DATA_POINT);
                if has_data {
                    continue;
                }
                let source = &input.columns[index(x / 2 + offset_x, z / 2 + offset_z)];
                if source.is_empty() {
                    continue;
                }
                self.columns[recipient] = source
                    .iter()
                    .map(|&p| full_point::set_id(p, remapped[full_point::id(p) as usize]))
                    .collect();
                self.gen_steps[recipient] = gen_step::DOWN_SAMPLED;
                accepted.push(recipient);
                self.is_empty = false;
                changed.push(recipient);
            }
        }
        changed
    }

    /// `removeUnusedIdsAndRemap`: keeps the map to the ids in use.
    fn remove_unused_ids_and_remap(&mut self) {
        let mut fresh = IdMap::default();
        let mut new_by_old = std::collections::HashMap::new();
        for column in &self.columns {
            for &p in column {
                let old = full_point::id(p);
                new_by_old.entry(old).or_insert_with(|| {
                    let (biome, state) = self.mapping.get(old);
                    fresh.add_if_not_present(biome, state)
                });
            }
        }
        for stem in &self.stems {
            new_by_old.entry(stem.id).or_insert_with(|| {
                let (biome, state) = self.mapping.get(stem.id);
                fresh.add_if_not_present(biome, state)
            });
        }
        for column in &mut self.columns {
            for p in column.iter_mut() {
                *p = full_point::set_id(*p, new_by_old[&full_point::id(*p)]);
            }
        }
        self.mapping = fresh;
        for stem in &mut self.stems {
            stem.id = new_by_old[&stem.id];
        }
    }

    fn stem_column(&self, stem: &Stem) -> Option<usize> {
        let width = pos::data_width(self.pos);
        let x = (stem.x - pos::min_corner_block_x(self.pos)).div_euclid(width);
        let z = (stem.z - pos::min_corner_block_z(self.pos)).div_euclid(width);
        ((0..pos::WIDTH).contains(&x) && (0..pos::WIDTH).contains(&z)).then(|| index(x, z))
    }

    /// Bound physical footprints, not light-split vertical runs. A native2x2
    /// trunk occupies four footprints even when each has many lighting bands.
    pub fn normalize_stems(&mut self) {
        if pos::data_width(self.pos) > STEM_MAX_WIDTH {
            self.stems.clear();
            return;
        }
        let mut roots = std::collections::BTreeMap::<(usize, i32, i32), Vec<Stem>>::new();
        for &stem in &self.stems {
            if stem.min_y < stem.max_y && stem.sky <= 15 && stem.block <= 15
                && let Some(i) = self.stem_column(&stem) {
                roots.entry((i, stem.x, stem.z)).or_default().push(stem);
            }
        }
        let mut groups = Vec::new();
        for ((i, x, z), mut spans) in roots {
            if spans.iter().any(|s| s.leaves == 0) {
                spans.retain(|s| s.leaves == 0);
                spans.sort_unstable_by_key(|s| (s.min_y, s.max_y, s.id, s.sky, s.block));
                let mut merged: Vec<Stem> = Vec::new();
                for s in spans {
                    if let Some(last) = merged.last_mut()
                        && last.id == s.id && s.min_y <= last.max_y {
                        last.max_y = last.max_y.max(s.max_y);
                        last.crown_max = last.max_y;
                        last.sky = last.sky.max(s.sky);
                        last.block = last.block.max(s.block);
                    } else { merged.push(s); }
                }
                spans = merged;
            } else {
                // Same root observed from several nearby input samples is one
                // preview, not overlapping wood at identical X/Z.
                spans.sort_unstable_by_key(|s| (std::cmp::Reverse(s.crown_max),
                    std::cmp::Reverse(s.max_y - s.min_y), *s));
                spans.truncate(1);
            }
            let lo = spans.iter().map(|s| s.min_y).min().unwrap();
            let hi = spans.iter().map(|s| s.max_y).max().unwrap();
            groups.push((i, std::cmp::Reverse(hi - lo), x, z, spans));
        }
        groups.sort_unstable_by_key(|(i, height, x, z, _)| (*i, *height, *x, *z));
        self.stems.clear();
        let (mut previous, mut count) = (usize::MAX, 0);
        for (i, _, _, _, spans) in groups {
            if i != previous { previous = i; count = 0; }
            count += 1;
            if count <= STEMS_PER_COLUMN { self.stems.extend(spans); }
        }
    }

    fn exact_stems(&self, blocks: &BlockTable) -> Vec<Stem> {
        let mut stems = self.stems.clone();
        if pos::data_width(self.pos) != 1 { return stems; }
        let (bx, bz) = (pos::min_corner_block_x(self.pos), pos::min_corner_block_z(self.pos));
        for (i, column) in self.columns.iter().enumerate() {
            if self.gen_steps[i] < gen_step::FEATURES { continue; }
            for &p in column {
                if blocks.get(self.mapping.state(full_point::id(p))).stem {
                    stems.push(Stem {
                        x: bx + i as i32 / pos::WIDTH, z: bz + i as i32 % pos::WIDTH,
                        min_y: full_point::bottom_y(p) as u16, max_y: full_point::top_y(p) as u16,
                        id: full_point::id(p), sky: full_point::sky_light(p), block: full_point::block_light(p),
                        leaves: 0, crown_max: full_point::top_y(p) as u16,
                    });
                }
            }
        }
        stems
    }

    fn update_stems(&mut self, input: &Self, remapped: &[u32], blocks: &BlockTable, accepted: &[usize]) -> bool {
        if accepted.is_empty() { return false; }
        let mut mask = [false; COLUMNS];
        for &i in accepted { mask[i] = true; }
        let before = self.stems.clone();
        self.stems = self.stems.iter().copied().filter(|s|
            self.stem_column(s).is_some_and(|i| !mask[i])).collect();
        for mut stem in input.exact_stems(blocks) {
            if self.stem_column(&stem).is_some_and(|i| mask[i]) {
                stem.id = remapped[stem.id as usize];
                self.stems.push(stem);
            }
        }
        self.normalize_stems();
        self.stems != before
    }
}

/// `FullDataSourceV2.mergeInputTwoByTwoDataColumn`: the 2 x 2 columns at
/// `(x, z)`, sliced at every height where one of them changes; each slice
/// takes the most common non-air block, and the light of the columns that
/// have that block. The local adaptation requires at least two occupied
/// children, matching Horizon's direct-preview occupancy threshold.
fn merge_two_by_two_column(input: &FullDataSource, x: i32, z: i32, blocks: &BlockTable) -> Vec<u64> {
    let columns: [&[u64]; 4] = [
        input.column(x, z),
        input.column(x, z + 1),
        input.column(x + 1, z),
        input.column(x + 1, z + 1),
    ];
    let mut transitions: Vec<i32> = Vec::new();
    for column in columns {
        for &p in column {
            transitions.push(full_point::bottom_y(p));
            transitions.push(full_point::top_y(p));
        }
    }
    if transitions.is_empty() {
        return Vec::new();
    }
    transitions.sort_unstable();
    transitions.dedup();
    // From each column's bottom-most run up.
    let mut current: [isize; 4] = columns.map(|c| c.len() as isize - 1);
    let mut out = Vec::new();
    let (mut last_id, mut last_block, mut last_sky) = (0u32, 0u8, 0u8);
    let mut current_min_y = transitions[0];
    let mut accumulated = 0;
    for pair in transitions.windows(2) {
        let (slice_min, slice_max) = (pair[0], pair[1]);
        let slice_height = slice_max - slice_min;
        let sample_y = slice_min + slice_height / 2;
        let mut ids = [0u32; 4];
        let mut block_lights = [0u8; 4];
        let mut sky_lights = [0u8; 4];
        for i in 0..4 {
            while current[i] >= 0 {
                let p = columns[i][current[i] as usize];
                if sample_y >= full_point::top_y(p) {
                    current[i] -= 1;
                } else {
                    if sample_y >= full_point::bottom_y(p) {
                        ids[i] = full_point::id(p);
                        block_lights[i] = full_point::block_light(p);
                        sky_lights[i] = full_point::sky_light(p);
                    }
                    break;
                }
            }
        }
        let id = most_common_in_slice(ids, &input.mapping, blocks);
        let block_light = average_in_slice_with_id(block_lights, ids, id);
        let sky_light = average_in_slice_with_id(sky_lights, ids, id);
        if accumulated == 0 {
            (last_id, last_block, last_sky) = (id, block_light, sky_light);
            current_min_y = slice_min;
            accumulated = slice_height;
        } else if id != last_id || block_light != last_block || sky_light != last_sky {
            out.extend(full_point::encode(
                last_id,
                accumulated,
                current_min_y,
                last_block,
                last_sky,
            ));
            (last_id, last_block, last_sky) = (id, block_light, sky_light);
            current_min_y = slice_min;
            accumulated = slice_height;
        } else {
            accumulated += slice_height;
        }
    }
    if accumulated > 0 {
        out.extend(full_point::encode(
            last_id,
            accumulated,
            current_min_y,
            last_block,
            last_sky,
        ));
    }
    // Built from the bottom up; columns are kept from the top down.
    out.reverse();
    out
}

/// `determineMostCommonValueInColumnSlice`, with Horizon's half-footprint
/// occupancy rule: one shore/leaf column must not expand into all four.
/// Known air wins below that threshold; occupied material ties still go to
/// the first column. Missing children are rejected before this reduction.
fn most_common_in_slice(ids: [u32; 4], mapping: &IdMap, blocks: &BlockTable) -> u32 {
    let air = ids.map(|id| blocks.get(mapping.state(id)).air);
    if air.iter().filter(|&&is_air| !is_air).count() < 2 {
        return ids[air.iter().position(|&is_air| is_air).expect("mostly known air")];
    }
    let mut counts = [0; 4];
    for &id in &ids {
        if blocks.get(mapping.state(id)).air {
            continue;
        }
        let slot = if id == ids[0] {
            0
        } else if id == ids[1] {
            1
        } else if id == ids[2] {
            2
        } else {
            3
        };
        counts[slot] += 1;
    }
    let max = *counts.iter().max().unwrap_or(&0);
    let winner = counts.iter().position(|&c| c == max).unwrap_or(0);
    ids[winner]
}

/// `determineAverageValueInColumnSliceWithId`: the light of the columns with
/// the winning block, so surface water doesn't light the sand below it.
fn average_in_slice_with_id(values: [u8; 4], ids: [u32; 4], winner: u32) -> u8 {
    let (mut sum, mut count) = (0u32, 0u32);
    for i in 0..4 {
        if ids[i] == winner {
            sum += u32::from(values[i]);
            count += 1;
        }
    }
    // With no column of the winner's, all four (smoother light).
    sum.checked_div(count)
        .unwrap_or_else(|| values.iter().map(|&v| u32::from(v)).sum::<u32>() / 4) as u8
}

/// `FullDataOcclusionCuller.cullHiddenDatapointsInColumn`: a run with opaque
/// runs above, below and on all four sides over its whole height is merged
/// into the run above it.
fn cull_hidden_data_points(source: &mut FullDataSource, x: i32, z: i32, blocks: &BlockTable) {
    let translucent =
        |source: &FullDataSource, p: u64| blocks.get(source.mapping.state(full_point::id(p))).opacity < 16;
    let neighbors: [Vec<u64>; 4] = match [
        source.try_column(x + 1, z),
        source.try_column(x - 1, z),
        source.try_column(x, z + 1),
        source.try_column(x, z - 1),
    ] {
        [Some(a), Some(b), Some(c), Some(d)] if !a.is_empty() && !b.is_empty() && !c.is_empty() && !d.is_empty() => {
            [a.clone(), b.clone(), c.clone(), d.clone()]
        }
        _ => return,
    };
    let mut center = std::mem::take(&mut source.columns[index(x, z)]);
    let mut adjacent: [isize; 4] = neighbors.each_ref().map(|c| c.len() as isize - 1);
    let mut i = center.len() as isize - 1;
    'points: while i >= 0 {
        let at = i as usize;
        let current = center[at];
        let skip = translucent(source, current)
            || at == 0
            || translucent(source, center[at - 1])
            || (at + 1 < center.len() && translucent(source, center[at + 1]))
            || at + 1 == center.len();
        if skip {
            i -= 1;
            continue;
        }
        for side in 0..4 {
            let (covered, next) = check_occlusion(source, current, &neighbors[side], adjacent[side], &translucent);
            adjacent[side] = next;
            if !covered {
                i -= 1;
                continue 'points;
            }
        }
        center.remove(at);
        let above = center[at - 1];
        let above = full_point::set_bottom_y(above, full_point::bottom_y(current));
        let above = full_point::set_height(above, full_point::height(current) + full_point::height(above));
        center[at - 1] = above;
        i -= 1;
    }
    source.columns[index(x, z)] = center;
}

/// `FullDataOcclusionCuller.checkOcclusion`: whether opaque runs of
/// `adjacent` cover `center`'s whole height, and where to continue from.
fn check_occlusion(
    source: &FullDataSource,
    center: u64,
    adjacent: &[u64],
    mut at: isize,
    translucent: &impl Fn(&FullDataSource, u64) -> bool,
) -> (bool, isize) {
    if adjacent.is_empty() || (adjacent.len() == 1 && adjacent[0] == full_point::EMPTY_DATA_POINT) {
        return (false, at);
    }
    let bottom = full_point::bottom_y(center);
    let top = full_point::top_y(center);
    while at >= 0 {
        let p = adjacent[at as usize];
        let adjacent_top = full_point::top_y(p);
        if adjacent_top <= bottom {
            at -= 1;
            continue;
        } else if translucent(source, p) {
            return (false, at);
        } else if adjacent_top >= top {
            return (true, at);
        }
        at -= 1;
    }
    (false, at.max(0))
}

/// A chunk as `LodDataBuilder` reads it (`IChunkWrapper`).
pub trait ChunkView: Send {
    fn chunk_x(&self) -> i32;
    fn chunk_z(&self) -> i32;
    /// `getInclusiveMinBuildHeight`.
    fn min_build_height(&self) -> i32;
    /// `getExclusiveMaxBuildHeight`.
    fn max_build_height(&self) -> i32;
    /// `getMinNonEmptyHeight`: the bottom of the lowest section with blocks.
    fn min_non_empty_height(&self) -> i32;
    /// The highest of the heightmaps at `(x, z)` (one above the top block).
    fn top(&self, x: i32, z: i32) -> i32;
    fn state(&self, x: i32, y: i32, z: i32) -> u32;
    fn biome(&self, x: i32, y: i32, z: i32) -> u16;
    /// `(sky, block)` light, 0 to 15 each.
    fn light(&self, x: i32, y: i32, z: i32) -> (u8, u8);
}

/// `LodDataBuilder.createFromChunk`: a chunk's 16 x 16 columns in its leaf
/// section (detail 6). Runs are read from the top down; a block that colours
/// the top of what is under it (snow, flowers) is kept as its own run.
pub fn create_from_chunk(chunk: &dyn ChunkView, blocks: &BlockTable) -> FullDataSource {
    let (cx, cz) = (chunk.chunk_x(), chunk.chunk_z());
    let section = pos::encode(pos::SECTION_BLOCK_DETAIL_LEVEL, cx.div_euclid(4), cz.div_euclid(4));
    let mut source = FullDataSource::empty(section);
    let offset_x = (cx & 3) * 16;
    let offset_z = (cz & 3) * 16;
    let min_build = chunk.min_build_height();
    let max_build = chunk.max_build_height();
    let min_non_empty = chunk.min_non_empty_height();
    let air_state = 0;
    for rx in 0..16 {
        for rz in 0..16 {
            let mut column = Vec::with_capacity(16);
            let mut last_y = max_build;
            let mut current_biome = chunk.biome(rx, last_y, rz);
            let mut current_state = air_state;
            let mut mapped = source.mapping.add_if_not_present(current_biome, current_state);
            let (mut block_light, mut sky_light) = (0u8, 15u8);
            let mut y = chunk.top(rx, rz);
            while !blocks.get(chunk.state(rx, y, rz)).air && y < max_build {
                y += 1;
            }
            let mut force_single_block = false;
            while y >= min_non_empty {
                let biome = chunk.biome(rx, y, rz);
                let state = chunk.state(rx, y, rz);
                let (sky, block) = chunk.light(rx, y + 1, rz);
                if biome != current_biome || state != current_state || force_single_block {
                    let info = blocks.get(current_state);
                    force_single_block = !info.air && !info.solid && !info.liquid && info.opacity != 16;
                    column.extend(full_point::encode(
                        mapped,
                        last_y - y,
                        y + 1 - min_build,
                        block_light,
                        sky_light,
                    ));
                    current_biome = biome;
                    current_state = state;
                    mapped = source.mapping.add_if_not_present(current_biome, current_state);
                    block_light = block;
                    sky_light = sky;
                    last_y = y;
                }
                y -= 1;
            }
            column.extend(full_point::encode(
                mapped,
                last_y - y,
                y + 1 - min_build,
                block_light,
                sky_light,
            ));
            source.set_column(offset_x + rx, offset_z + rz, column, gen_step::LIGHT);
        }
    }
    source
}

/// A run of one sampled column, from the world generator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampledRun {
    /// Absolute block heights, `min..max`.
    pub min: i32,
    pub max: i32,
    pub state: u32,
    pub biome: u16,
    pub sky: u8,
    pub block: u8,
}

/// The world generator, as far terrain samples it: one column at a time,
/// without generating chunks. Runs come from the bottom up; gaps are air.
pub trait ColumnSampler: Send {
    /// Queue exact features for cached preview sections as well as new samples.
    fn request_section(&mut self, _section: i64) {}
    /// The current near area, for bounded background feature generation.
    fn view(&mut self, _center: [i32; 2], _near_radius: i32) {}
    /// Completed, feature-accurate columns replace previews even while idle.
    fn drain_refined(&mut self) -> Vec<FullDataSource> {
        Vec::new()
    }
    /// Begin a bounded sampling batch at this section's data spacing.
    fn begin_section(&mut self, _stride: i32) {}
    /// Attach only metadata collected by this exact sampling batch.
    fn finish_section(&mut self, _source: &mut FullDataSource) {}
    /// False means unavailable, not air (e.g. a refused server tile).
    fn try_column(&mut self, x: i32, z: i32, out: &mut Vec<SampledRun>) -> bool {
        self.column(x, z, out);
        true
    }
    fn column(&mut self, x: i32, z: i32, out: &mut Vec<SampledRun>);
}

/// Distant generation (`WorldGenerationQueue` with a generator's
/// `generateLod`): fills the columns of `source` that have no data, one
/// sample per data column at its centre, at the section's own detail
/// level. Returns how many columns it filled.
pub fn generate_missing(
    source: &mut FullDataSource,
    sampler: &mut dyn ColumnSampler,
    min_build: i32,
    max_build: i32,
) -> usize {
    let missing: Vec<_>=(0..COLUMNS).map(|i|source.column_is_missing(i)).collect();
    generate_missing_where(source,sampler,min_build,max_build,|i|missing[i])
}

/// Sample a sparse update without marking skipped columns as incoming data.
fn generate_missing_where(
    source: &mut FullDataSource,
    sampler: &mut dyn ColumnSampler,
    min_build: i32,
    max_build: i32,
    missing: impl Fn(usize) -> bool,
) -> usize {
    let width = pos::data_width(source.pos);
    let (base_x, base_z) = (pos::min_corner_block_x(source.pos), pos::min_corner_block_z(source.pos));
    sampler.begin_section(width);
    let mut runs = Vec::new();
    let mut filled = 0;
    for x in 0..pos::WIDTH {
        for z in 0..pos::WIDTH {
            if !missing(index(x, z)) {
                continue;
            }
            runs.clear();
            if !sampler.try_column(
                base_x + x * width + width / 2,
                base_z + z * width + width / 2,
                &mut runs,
            ) {
                continue;
            }
            let column = column_from_runs(&mut source.mapping, &runs, min_build, max_build);
            source.set_column(x, z, column, gen_step::SURFACE);
            filled += 1;
        }
    }
    sampler.finish_section(source);
    source.normalize_stems();
    filled
}

/// A full data column (top down) from sampled runs (bottom up), with air
/// over and between them.
pub fn column_from_runs(mapping: &mut IdMap, runs: &[SampledRun], min_build: i32, max_build: i32) -> Vec<u64> {
    let mut column = Vec::with_capacity(runs.len() * 2 + 1);
    let mut top = max_build;
    let mut above_light = (15u8, 0u8);
    for run in runs.iter().rev() {
        let (min, max) = (run.min.max(min_build), run.max.min(max_build));
        if min >= max {
            continue;
        }
        if max < top {
            let air = mapping.add_if_not_present(run.biome, 0);
            column.extend(full_point::encode(
                air,
                top - max,
                max - min_build,
                above_light.1,
                above_light.0,
            ));
        }
        let id = mapping.add_if_not_present(run.biome, run.state);
        column.extend(full_point::encode(id, max - min, min - min_build, run.block, run.sky));
        top = min;
        above_light = (run.sky, run.block);
    }
    if column.is_empty() {
        let air = mapping.add_if_not_present(0, 0);
        column.extend(full_point::encode(air, max_build - min_build, 0, 0, 15));
    }
    column
}

/// `EDhApiHorizontalQuality`: how far apart the drops in detail are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorizontalQuality {
    Lowest,
    Low,
    Medium,
    High,
    Extreme,
}

#[allow(dead_code)]
impl HorizontalQuality {
    pub const ALL: [Self; 5] = [Self::Lowest, Self::Low, Self::Medium, Self::High, Self::Extreme];
    /// `(quadraticBase, distanceUnitInBlocks)`.
    pub fn values(self) -> (f64, i32) {
        match self {
            Self::Lowest => (2.0, 4),
            Self::Low => (2.0, 8),
            Self::Medium => (2.0, 12),
            Self::High => (2.2, 16),
            Self::Extreme => (2.2, 20),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Lowest => "lowest",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Extreme => "extreme",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Lowest => "Lowest",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Extreme => "Extreme",
        }
    }
}

/// `LodQuadTree`'s detail-level logic (`updateDetailLevelVariables`,
/// `calcExpectedDetailLevel`) and its walk from the root nodes down
/// (`updateNode`): which sections to draw, at which detail, around a player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DetailPlan {
    drop_off_unit: f64,
    drop_off_log_base: f64,
    /// `maxHorizontalResolution`: 0 draws single blocks nearest.
    max_leaf_detail: u8,
    min_root_detail: u8,
    /// The LOD render distance, in blocks.
    pub radius: i32,
    boundary_radius: i32,
    boundary_allowed: bool,
}

#[allow(dead_code)]
impl DetailPlan {
    /// Preserve block-scale terrain around the actual near/distant handoff.
    pub fn with_near_radius(mut self, radius_chunks: i32) -> Self {
        let radius = radius_chunks.saturating_mul(16);
        if self.boundary_allowed && radius > 0 && radius < self.radius {
            self.boundary_radius = radius;
        }
        self
    }
    pub fn new(quality: HorizontalQuality, radius_chunks: i32) -> Self {
        let (base, unit) = quality.values();
        let mut plan = Self {
            drop_off_unit: f64::from(unit * 16),
            drop_off_log_base: base.ln(),
            max_leaf_detail: 0,
            min_root_detail: 0,
            radius: radius_chunks * 16,
            boundary_radius: 0,
            // Older-PC Low/Lowest profiles retain their original cost.
            boundary_allowed: matches!(
                quality,
                HorizontalQuality::Medium | HorizontalQuality::High | HorizontalQuality::Extreme
            ),
        };
        let tree_root = pos::ROOT_DETAIL_LEVEL - pos::SECTION_BLOCK_DETAIL_LEVEL;
        let min = plan.detail_from_distance(f64::from(plan.radius * 2)).saturating_sub(1);
        plan.min_root_detail = min.min(tree_root).max(plan.max_leaf_detail);
        plan
    }
    /// `calcDetailLevelFromDistance` (Java's int cast truncates toward 0).
    fn detail_from_distance(&self, distance: f64) -> u8 {
        let level = ((distance / self.drop_off_unit).ln() / self.drop_off_log_base) as i64;
        level.clamp(i64::from(self.max_leaf_detail), i64::from(pos::ROOT_DETAIL_LEVEL)) as u8
    }
    /// The section detail level a section should be drawn at.
    pub fn expected_detail_level(&self, player: [f64; 2], section: i64) -> u8 {
        let dx = f64::from(pos::center_block_x(section)) - player[0];
        let dz = f64::from(pos::center_block_z(section)) - player[1];
        if self.boundary_radius > 0 {
            // The vanilla near circle has one chunk of slack per axis. A
            // small full-block ring covers it and the next couple of chunks;
            // distant sections keep their ordinary quality/drop-off curve.
            let half = f64::from(pos::block_width(section)) * 0.5;
            let closest = (dx.abs() - half).max(0.).hypot((dz.abs() - half).max(0.));
            let farthest = (dx.abs() + half).hypot(dz.abs() + half);
            let radius = f64::from(self.boundary_radius);
            if closest < radius + 64. && farthest > (radius - 32.).max(0.) {
                return pos::SECTION_MINIMUM_DETAIL_LEVEL;
            }
        }
        let detail = self.detail_from_distance((dx * dx + dz * dz).sqrt());
        detail.min(self.min_root_detail) + pos::SECTION_MINIMUM_DETAIL_LEVEL
    }
    /// The sections to draw: the root nodes over the render distance's
    /// square, split while finer than their expected detail. `skip` leaves
    /// out sections the game draws itself (in its render distance).
    pub fn sections(&self, player: [f64; 2], skip: impl Fn(i64) -> bool) -> Vec<i64> {
        let (px, pz) = (player[0].floor() as i32, player[1].floor() as i32);
        let root = pos::ROOT_DETAIL_LEVEL;
        let lo = pos::containing_block(root, px - self.radius, pz - self.radius);
        let hi = pos::containing_block(root, px + self.radius, pz + self.radius);
        let mut out = Vec::new();
        for x in pos::x(lo)..=pos::x(hi) {
            for z in pos::z(lo)..=pos::z(hi) {
                self.walk(pos::encode(root, x, z), player, &skip, &mut out);
            }
        }
        out
    }
    fn walk(&self, node: i64, player: [f64; 2], skip: &impl Fn(i64) -> bool, out: &mut Vec<i64>) {
        let (px, pz) = (player[0].floor() as i32, player[1].floor() as i32);
        if pos::chebyshev_signed_block_distance(node, px, pz) > self.radius || skip(node) {
            return;
        }
        let detail = pos::detail_level(node);
        if detail > self.expected_detail_level(player, node) && detail > pos::SECTION_MINIMUM_DETAIL_LEVEL {
            for i in 0..4 {
                self.walk(pos::child(node, i), player, skip, out);
            }
        } else {
            out.push(node);
        }
    }
}

/// `RenderDataPointUtil`: one drawn box of a render column, in a long.
/// Bits 0-3 sky light, 4-7 block light, 8-19 bottom, 20-31 top (relative to
/// the world's minimum y), 32-55 colour, 56-59 alpha / 16, 60-63 material.
///
/// The colour bits differ from Distant Horizons: it stores the block's
/// averaged texture colour there, this port the block's id in its source's
/// `IdMap`, which Horizon draws with the block's own texture. Merging,
/// transparency and face culling compare it as Distant Horizons compares
/// colours.
#[allow(dead_code)]
pub mod render_point {
    pub const EMPTY_DATA: u64 = 0;
    const ALPHA_DOWNSIZE_SHIFT: u32 = 4;
    const MATERIAL_SHIFT: u32 = 60;
    const COLOR_SHIFT: u32 = 32;
    const ALPHA_SHIFT: u32 = 56;
    const MAX_Y_SHIFT: u32 = 20;
    const MIN_Y_SHIFT: u32 = 8;
    const BLOCK_LIGHT_SHIFT: u32 = 4;
    const MASK_12: u64 = 0xFFF;
    const COLOR_MASK: u64 = 0xFF_FFFF;

    /// `color` is `alpha << 24 | id`.
    pub fn create(max_y: i32, min_y: i32, color: u32, sky: u8, block: u8, material: u8) -> u64 {
        let alpha = u64::from(color >> 24);
        ((alpha >> ALPHA_DOWNSIZE_SHIFT) << ALPHA_SHIFT)
            | ((u64::from(color) & COLOR_MASK) << COLOR_SHIFT)
            | ((max_y as u64 & MASK_12) << MAX_Y_SHIFT)
            | ((min_y as u64 & MASK_12) << MIN_Y_SHIFT)
            | ((u64::from(block) & 15) << BLOCK_LIGHT_SHIFT)
            | (u64::from(sky) & 15)
            | ((u64::from(material) & 15) << MATERIAL_SHIFT)
    }
    pub fn y_max(p: u64) -> i32 {
        ((p >> MAX_Y_SHIFT) & MASK_12) as i32
    }
    pub fn y_min(p: u64) -> i32 {
        ((p >> MIN_Y_SHIFT) & MASK_12) as i32
    }
    pub fn set_y_min(p: u64, min_y: i32) -> u64 {
        (p & !(MASK_12 << MIN_Y_SHIFT)) | ((min_y as u64 & MASK_12) << MIN_Y_SHIFT)
    }
    pub fn alpha(p: u64) -> u8 {
        ((((p >> ALPHA_SHIFT) & 15) << ALPHA_DOWNSIZE_SHIFT) | 0b1111) as u8
    }
    pub fn id(p: u64) -> u32 {
        ((p >> COLOR_SHIFT) & COLOR_MASK) as u32
    }
    pub fn color(p: u64) -> u32 {
        (u32::from(alpha(p)) << 24) | id(p)
    }
    pub fn sky(p: u64) -> u8 {
        (p & 15) as u8
    }
    pub fn block(p: u64) -> u8 {
        ((p >> BLOCK_LIGHT_SHIFT) & 15) as u8
    }
    pub fn material(p: u64) -> u8 {
        ((p >> MATERIAL_SHIFT) & 15) as u8
    }
    pub fn exists(p: u64) -> bool {
        p != EMPTY_DATA
    }
    pub fn has_zero_height(p: u64) -> bool {
        ((p >> MIN_Y_SHIFT) & 0xFF_FFFF) == 0
    }
    pub fn is_fluid(p: u64) -> bool {
        let m = material(p);
        m == super::material::WATER || m == super::material::LAVA
    }
}

/// A render column: its boxes from the top down, and the void under them
/// (`ColumnRenderView` with `getVoid`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderColumn {
    pub points: Vec<u64>,
    pub void: u64,
}

/// What the render transform takes from the player's settings and the
/// dimension.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderSettings {
    /// `enableCaveCulling` (off in dimensions with a ceiling and the End).
    pub cave_culling: bool,
    /// `caveCullingHeight`, absolute.
    pub cave_culling_height: i32,
    /// The world's minimum y.
    pub min_y: i32,
}

/// `FullDataToRenderDataTransformer.setRenderColumnView`: the boxes to draw
/// for one full data column. Air and non-colliding blocks (grass, flowers)
/// are left out (`EDhApiBlocksToAvoid.NON_COLLIDING`); a snow layer colours
/// the block under it; dark runs under the cave culling height that only
/// connect to other dark runs are folded into the box above.
pub fn render_column(full: &[u64], mapping: &IdMap, blocks: &BlockTable, settings: &RenderSettings) -> RenderColumn {
    let mut out = RenderColumn::default();
    let cave_max_y = settings.cave_culling_height - settings.min_y;
    let mut last_color = 0u32;
    let mut last_bottom = -10_000;
    let mut last_block: Option<BlockInfo> = None;
    let mut air_after_last = false;
    let mut apply_to_next: Option<(u32, BlockInfo)> = None;
    let mut sky_to_next: Option<u8> = None;
    let mut block_to_next: Option<u8> = None;
    for (i, &p) in full.iter().enumerate() {
        let bottom = full_point::bottom_y(p);
        let mut height = full_point::height(p);
        let top = bottom + height;
        let id = full_point::id(p);
        let mut sky = full_point::sky_light(p);
        let mut light = full_point::block_light(p);
        let mut info = blocks.get(mapping.state(id));
        let mut color_id = id;
        if info.air {
            apply_to_next = None;
            sky_to_next = None;
            block_to_next = None;
            air_after_last = true;
        }
        if info.material == material::WATER
            && let Some(last) = last_block
            && last.ice
            && !air_after_last
        {
            info = last;
        }
        if !info.solid || info.liquid || info.opacity < 16 {
            if settings.cave_culling
                && sky == 0
                && top < cave_max_y
                && !out.points.is_empty()
                && i != 0
                && i + 1 < full.len()
            {
                let next_sky = full_point::sky_light(full[i + 1]);
                if next_sky == 0 && (last_color >> 24) == 255 {
                    let last = out.points.len() - 1;
                    out.points[last] = render_point::set_y_min(out.points[last], bottom);
                }
                continue;
            }
            if info.air {
                if i == full.len() - 1 {
                    set_void_air(&mut out, bottom, height, sky, light);
                }
                continue;
            }
        }
        let mut ignore = !info.solid && !info.liquid && info.opacity != 16;
        if info.snow_layer {
            height -= 1;
            if height == 0 {
                ignore = true;
                apply_to_next = Some((id, info));
            }
        }
        if ignore {
            // Kelp/seagrass occupy water, not an air gap. Once a column is
            // underwater, skipped non-colliding vegetation extends that water
            // until a real air run or a retained material ends it. Otherwise
            // every plant adds translucent top/bottom faces inside the ocean.
            if !info.air
                && !air_after_last
                && last_block.is_some_and(|last| last.material == material::WATER)
                && last_bottom == top
                && let Some(last) = out.points.last_mut()
            {
                *last = render_point::set_y_min(*last, bottom);
                last_bottom = bottom;
                continue;
            }
            // Flowers and grass only pass on their light: with block textures
            // their colour on the ground below would draw them as a carpet.
            if info.alpha != 0 {
                sky_to_next = Some(sky);
                block_to_next = Some(light);
            }
            if i == full.len() - 1 {
                set_void_air(&mut out, bottom, height, sky, light);
            }
            continue;
        }
        if let Some((carried, carried_info)) = apply_to_next.take() {
            color_id = carried;
            info = BlockInfo {
                alpha: 255,
                ..carried_info
            };
        }
        if let Some(s) = sky_to_next.take() {
            sky = s;
        }
        if let Some(b) = block_to_next.take() {
            light = b;
        }
        let color = (u32::from(info.alpha) << 24) | (color_id & 0xFF_FFFF);
        if color == last_color && bottom + height == last_bottom && !out.points.is_empty() {
            let last = out.points.len() - 1;
            out.points[last] = render_point::set_y_min(out.points[last], bottom);
        } else {
            out.points.push(render_point::create(
                bottom + height,
                bottom,
                color,
                sky,
                light,
                info.material,
            ));
        }
        last_bottom = bottom;
        last_color = color;
        last_block = Some(info);
        air_after_last = false;
    }
    out
}

/// `setVoidAir`: the void under a column that ends in air or ignored blocks,
/// for the light of the faces next to it.
fn set_void_air(out: &mut RenderColumn, bottom: i32, height: i32, sky: u8, light: u8) {
    let max_y = match out.points.last() {
        Some(&p) => render_point::y_min(p),
        None => bottom + height,
    };
    out.void = render_point::create(max_y, 0, 0, sky, light, material::AIR);
}

/// A section's render columns (`ColumnRenderSource`).
#[derive(Clone, Debug, PartialEq)]
pub struct RenderSource {
    pub pos: i64,
    pub columns: Vec<RenderColumn>,
}

impl RenderSource {
    /// `transformFullDataToRenderSource`.
    pub fn from_full(full: &FullDataSource, blocks: &BlockTable, settings: &RenderSettings) -> Self {
        let columns = full
            .columns
            .iter()
            .map(|c| render_column(c, &full.mapping, blocks, settings))
            .collect();
        Self { pos: full.pos, columns }
    }
    pub fn column(&self, x: i32, z: i32) -> &RenderColumn {
        &self.columns[index(x, z)]
    }
}

/// A face direction, in Distant Horizons' `EDhDirection` sense.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Dir {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

/// A face to draw, in blocks from its section's minimum corner (y is
/// absolute): `u`/`v` span x/z for up and down, x/y for north and south,
/// z/y for west and east; `plane` is the other coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Quad {
    pub dir: Dir,
    pub plane: i32,
    pub transparent: bool,
    /// The block's id in the section's `IdMap`.
    pub id: u32,
    pub sky: u8,
    pub block: u8,
    pub u0: i32,
    pub v0: i32,
    pub u1: i32,
    pub v1: i32,
    /// An exposed fluid top, or the upper edge of a side that meets it.
    /// Covered water below a roof remains a full block high.
    pub lower_top: bool,
}

impl Quad {
    /// World X/Z at this face's center, including vertical-face axis mapping.
    pub fn horizontal_center(&self, section: i64) -> [f64; 2] {
        let u = f64::from(self.u0 + self.u1) * 0.5;
        let v = f64::from(self.v0 + self.v1) * 0.5;
        let plane = f64::from(self.plane);
        let [x, z] = match self.dir {
            Dir::Up | Dir::Down => [u, v],
            Dir::North | Dir::South => [u, plane],
            Dir::West | Dir::East => [plane, u],
        };
        [
            x + f64::from(pos::min_corner_block_x(section)),
            z + f64::from(pos::min_corner_block_z(section)),
        ]
    }
    /// The face in block units consumed by the host's vertex writer.
    pub fn geometry(&self) -> (f32, [f32; 4]) {
        let mut plane = self.plane as f32;
        let mut rect = [self.u0, self.v0, self.u1, self.v1].map(|v| v as f32);
        if self.lower_top {
            // The detailed mesher's still-fluid interior is 8/9 high,
            // minus its 0.001 epsilon. Keep integer full data on disk;
            // project only the exposed render surface to the same height.
            const LOWER: f32 = 1.0 / 9.0 + 0.001;
            if self.dir == Dir::Up {
                plane -= LOWER;
            } else if self.dir != Dir::Down {
                rect[3] -= LOWER;
            }
        }
        (plane, rect)
    }
}

/// A vertical range of a side face and the sky light it gets; `covered`
/// when an adjacent box hides it (`YSegmentUtil`).
#[derive(Clone, Copy, Debug)]
struct Segment {
    start: i32,
    end: i32,
    /// -1: covered by an adjacent box (`SKYLIGHT_COVERED`).
    sky: i8,
    covered: bool,
}

const SKYLIGHT_COVERED: i8 = -1;

/// `ColumnRenderBufferBuilder` with `ColumnBox.addBoxQuadsToBuilder`: the
/// faces of a section's boxes. Side faces are cut where the adjacent
/// column's boxes hide them. `neighbors` holds the sections north, south,
/// west and east of it at the same detail level, when loaded; columns at a
/// missing neighbor have no proven opaque cliff to draw. Columns `cutout` names are
/// left out (the game draws that terrain itself).
pub fn build_quads(
    source: &RenderSource,
    neighbors: [Option<&RenderSource>; 4],
    settings: &RenderSettings,
    cutout: impl Fn(i32, i32) -> bool,
) -> Vec<Quad> {
    let width = pos::data_width(source.pos);
    let mut quads = Vec::new();
    let adjacent = |x: i32, z: i32| -> Option<&RenderColumn> {
        let (nx, nz, side) = match (x, z) {
            (_, -1) => (x, pos::WIDTH - 1, 0),
            (_, z) if z == pos::WIDTH => (x, 0, 1),
            (-1, _) => (pos::WIDTH - 1, z, 2),
            (x, _) if x == pos::WIDTH => (0, z, 3),
            // A cutout is drawn by the near renderer, not an air-filled
            // hole. Its retained data must still occlude the far bank.
            _ => return known_column(source.column(x, z)),
        };
        neighbors[side].and_then(|s| known_column(s.column(nx, nz)))
    };
    for x in 0..pos::WIDTH {
        for z in 0..pos::WIDTH {
            if cutout(x, z) {
                continue;
            }
            let column = source.column(x, z);
            let adj = [
                adjacent(x, z - 1),
                adjacent(x, z + 1),
                adjacent(x - 1, z),
                adjacent(x + 1, z),
            ];
            let mut below_water = false;
            for (i, &p) in column.points.iter().enumerate() {
                if !render_point::exists(p) || render_point::has_zero_height(p) {
                    break;
                }
                let top = if i > 0 {
                    column.points[i - 1]
                } else {
                    render_point::EMPTY_DATA
                };
                let bottom = column.points.get(i + 1).copied().unwrap_or(column.void);
                add_box(
                    &mut quads,
                    BoxAt {
                        width,
                        min_x: x * width,
                        min_z: z * width,
                        min_y: render_point::y_min(p),
                        max_y: render_point::y_max(p),
                        min_world_y: settings.min_y,
                        below_water,
                    },
                    p,
                    top,
                    bottom,
                    adj,
                );
                below_water |= render_point::material(p) == material::WATER;
            }
        }
    }
    merge_quads(&mut quads);
    quads
}

/// No data is different from a known air column (whose void has a range).
fn known_column(column: &RenderColumn) -> Option<&RenderColumn> {
    (!column.points.is_empty() || column.void != render_point::EMPTY_DATA).then_some(column)
}

/// A predicted trunk is anchored only beneath its own surviving crown. LOD
/// rounding may move either end by at most one cell; a lost crown is not a
/// licence to create a freestanding pole or connect an unrelated neighbouring tree.
pub fn stem_bounds(source: &FullDataSource, stem: &Stem, blocks: &BlockTable) -> Option<(i32, i32)> {
    let i = source.stem_column(stem)?;
    if source.column_is_missing(i) { return None; }
    let (min, max) = (i32::from(stem.min_y), i32::from(stem.max_y));
    if stem.leaves == 0 { return (min < max).then_some((min, max)); }
    let column = &source.columns[i];
    let crown = column.iter().filter(|&&p|
        source.mapping.state(full_point::id(p)) == stem.leaves
        && full_point::bottom_y(p) < i32::from(stem.crown_max)
        && full_point::top_y(p) > max)
        .map(|&p| full_point::bottom_y(p)).min()?;
    let ground = column.iter().filter(|&&p| {
        let block = blocks.get(source.mapping.state(full_point::id(p)));
        !block.air && !block.liquid && !block.stem && block.material != material::LEAVES
            && block.opacity == 16 && full_point::top_y(p) <= crown
    }).map(|&p| full_point::top_y(p)).max()?;
    let tolerance = pos::data_width(source.pos);
    if (ground - min).abs() > tolerance || (crown - max).abs() > tolerance
        || !(1..=64).contains(&(crown - ground)) { return None; }
    if column.iter().any(|&p| {
        blocks.get(source.mapping.state(full_point::id(p))).liquid
            && full_point::top_y(p) > ground && full_point::bottom_y(p) < crown
    }) { return None; }
    Some((ground, crown))
}

/// At most six faces per visible exact one-block segment. These faces use the
/// existing terrain upload, world-generation identity and near cutout lifecycle.
pub fn append_stem_quads(
    source: &FullDataSource, blocks: &BlockTable, min_y: i32,
    cutout: impl Fn(i32, i32) -> bool, quads: &mut Vec<Quad>,
) {
    let width = pos::data_width(source.pos);
    if !(2..=STEM_MAX_WIDTH).contains(&width) { return; }
    let (bx, bz) = (pos::min_corner_block_x(source.pos), pos::min_corner_block_z(source.pos));
    for stem in &source.stems {
        let Some(i) = source.stem_column(stem) else { continue };
        if cutout(i as i32 / pos::WIDTH, i as i32 % pos::WIDTH) { continue; }
        let Some((bottom, top)) = stem_bounds(source, stem, blocks) else { continue };
        let column = &source.columns[i];
        let opaque = |p: u64| {
            let info = blocks.get(source.mapping.state(full_point::id(p)));
            !info.air && info.opacity == 16
        };
        // Avoid duplicate geometry where the reduced terrain already fills this
        // interval, including a dense real2x2 stem retained by the majority vote.
        let mut gaps = vec![(bottom, top)];
        for &p in column.iter().filter(|&&p| opaque(p)) {
            let (lo, hi) = (full_point::bottom_y(p), full_point::top_y(p));
            gaps = gaps.into_iter().flat_map(|(a, b)| {
                [(a < lo).then_some((a, b.min(lo))), (b > hi).then_some((a.max(hi), b))]
                    .into_iter().flatten().filter(|(a, b)| a < b)
            }).collect();
            if gaps.is_empty() { break; }
        }
        let (x, z) = (stem.x - bx, stem.z - bz);
        let covered = |y: i32| column.iter().any(|&p|
            opaque(p) && full_point::bottom_y(p) <= y && y < full_point::top_y(p));
        for (lo, hi) in gaps {
            let mut face = |dir, plane, u0, v0, u1, v1| quads.push(Quad {
                dir, plane, transparent: false, id: stem.id, sky: stem.sky, block: stem.block,
                u0, v0, u1, v1, lower_top: false,
            });
            face(Dir::North, z, x, lo + min_y, x + 1, hi + min_y);
            face(Dir::South, z + 1, x, lo + min_y, x + 1, hi + min_y);
            face(Dir::West, x, z, lo + min_y, z + 1, hi + min_y);
            face(Dir::East, x + 1, z, lo + min_y, z + 1, hi + min_y);
            if !covered(hi) { face(Dir::Up, hi + min_y, x, z, x + 1, z + 1); }
            if !covered(lo - 1) { face(Dir::Down, lo + min_y, x, z, x + 1, z + 1); }
        }
    }
}

#[derive(Clone, Copy)]
struct BoxAt {
    width: i32,
    min_x: i32,
    min_z: i32,
    /// Relative to the world's minimum y.
    min_y: i32,
    max_y: i32,
    min_world_y: i32,
    below_water: bool,
}

fn is_same_transparent_block(p: u64, color: u32, material: u8) -> bool {
    // State/biome mapping ids may change within one continuous fluid. Its
    // exposed faces retain their own tint, but the shared interior has no wall.
    render_point::material(p) == material
        && (matches!(material, material::WATER | material::LAVA) || render_point::color(p) == color)
}

fn add_box(quads: &mut Vec<Quad>, at: BoxAt, p: u64, top: u64, bottom: u64, adj: [Option<&RenderColumn>; 4]) {
    let material = render_point::material(p);
    if material == material::AIR {
        return;
    }
    let color = render_point::color(p);
    let id = render_point::id(p);
    let sky = render_point::sky(p) as i8;
    let block = render_point::block(p);
    let over_void = !render_point::exists(bottom);
    let transparent = render_point::alpha(p) < 255 && !over_void;
    let top_transparent = render_point::alpha(top) < 255;
    let bottom_transparent = render_point::alpha(bottom) < 255;
    let (x0, z0, w) = (at.min_x, at.min_z, at.width);
    let y0 = at.min_y + at.min_world_y;
    let y1 = at.max_y + at.min_world_y;
    let skip_top = render_point::exists(top)
        && render_point::y_min(top) == at.max_y
        && (!top_transparent || (transparent && is_same_transparent_block(top, color, material)));
    let exposed_fluid = render_point::is_fluid(p) && !skip_top;
    let quad = |dir, plane, u0, v0, u1, v1, sky: i8| Quad {
        dir,
        plane,
        transparent,
        id,
        sky: sky.max(0) as u8,
        block,
        u0,
        v0,
        u1,
        v1,
        lower_top: exposed_fluid && dir == Dir::Up,
    };
    if !skip_top {
        quads.push(quad(Dir::Up, y1, x0, z0, x0 + w, z0 + w, sky));
    }
    let skip_bottom = render_point::exists(bottom)
        && render_point::y_max(bottom) == at.min_y
        && (!bottom_transparent || (transparent && is_same_transparent_block(bottom, color, material)))
        && !(render_point::is_fluid(bottom) && render_point::material(bottom) != material);
    if !skip_bottom {
        let bottom_sky = if render_point::exists(bottom) {
            render_point::sky(bottom)
        } else {
            0
        };
        quads.push(quad(Dir::Down, y0, x0, z0, x0 + w, z0 + w, bottom_sky as i8));
    }
    // North and south faces span x; west and east span z.
    let sides = [
        (Dir::North, z0, x0),
        (Dir::South, z0 + w, x0),
        (Dir::West, x0, z0),
        (Dir::East, x0 + w, z0),
    ];
    for (side, (dir, plane, u)) in sides.into_iter().enumerate() {
        let segments = match adj[side] {
            None => {
                // A neighbour may still be preparing or use another detail
                // level. Unknown data is not air: inventing an opaque side
                // reveals compressed dirt/stone down to the world's bottom.
                // Known lower beds and genuine air still use side_segments.
                if !transparent || at.below_water || !over_void {
                    continue;
                }
                vec![Segment {
                    start: at.min_y,
                    end: at.max_y,
                    sky: 15,
                    covered: false,
                }]
            }
            Some(column) => side_segments(at, transparent, color, material, column),
        };
        for s in segments {
            if s.sky < 0 || (transparent && s.covered) || s.end <= s.start {
                continue;
            }
            let (v0, v1) = (s.start + at.min_world_y, s.end + at.min_world_y);
            quads.push(Quad {
                dir,
                plane,
                transparent,
                id,
                sky: s.sky as u8,
                block,
                u0: u,
                v0,
                u1: u + w,
                v1,
                lower_top: exposed_fluid && s.end == at.max_y,
            });
        }
    }
}

/// `ColumnBox.makeAdjVerticalQuad`: splits a side face by the boxes of the
/// adjacent column, each range getting the light it faces.
fn side_segments(
    at: BoxAt,
    transparent: bool,
    color: u32,
    material: u8,
    column: &RenderColumn,
) -> Vec<Segment> {
    let (y_min, y_max) = (at.min_y, at.max_y);
    if column.points.is_empty() || render_point::has_zero_height(column.points[0]) {
        return vec![Segment {
            start: y_min,
            end: y_max,
            sky: 15,
            covered: false,
        }];
    }
    let mut segments = vec![Segment {
        start: y_min,
        end: y_max,
        sky: 15,
        covered: false,
    }];
    let count = column.points.len() as isize;
    for adj_index in -1..count {
        let adj = if adj_index >= 0 {
            column.points[adj_index as usize]
        } else {
            column.void
        };
        let adj_min = render_point::y_min(adj);
        let mut adj_max = render_point::y_max(adj);
        if !render_point::exists(adj) || render_point::has_zero_height(adj) || y_max <= adj_min {
            continue;
        }
        let above = if adj_index > 0 {
            column.points[adj_index as usize - 1]
        } else {
            render_point::EMPTY_DATA
        };
        let mut below = if adj_index >= 0 {
            column
                .points
                .get(adj_index as usize + 1)
                .copied()
                .unwrap_or(column.void)
        } else {
            column.void
        };
        if below == render_point::EMPTY_DATA {
            below = column.void;
        }
        let adj_over_void = !render_point::exists(below);
        let adj_transparent = !adj_over_void && render_point::alpha(adj) < 255;
        let adj_sky = render_point::sky(adj) as i8;
        let same_fluid = render_point::is_fluid(adj) && render_point::material(adj) == material;
        let surface_lowered = render_point::is_fluid(adj)
            && !(render_point::exists(above)
                && render_point::y_min(above) == render_point::y_max(adj)
                && render_point::material(above) == render_point::material(adj));
        if !adj_transparent && !same_fluid && surface_lowered {
            adj_max -= 1;
        }
        let (light, covers) = if !adj_transparent {
            // Occupied opaque ground hides this range even at a section
            // boundary. Compressed columns may carry surface skylight all
            // the way down; that light is not evidence of exposed terrain.
            // Uncovered ranges still retain their ordinary cliff faces.
            (SKYLIGHT_COVERED, true)
        } else if transparent && is_same_transparent_block(adj, color, material) {
            (SKYLIGHT_COVERED, true)
        } else {
            (render_point::sky(below) as i8, false)
        };
        segments = apply_light_to_range(&segments, adj_min, adj_max, light, covers);
        let above_min = render_point::y_min(above);
        if adj_max < above_min {
            segments = apply_light_to_range(&segments, adj_max, above_min, adj_sky, false);
        }
    }
    segments
}

/// `applyLightToRangeAndPopulateNewSgements`: the segments with
/// `start..end` taking the lower of its light and `light`, split as needed.
fn apply_light_to_range(segments: &[Segment], start: i32, end: i32, light: i8, covers: bool) -> Vec<Segment> {
    let mut out = Vec::with_capacity(segments.len() + 2);
    for &s in segments {
        if s.end <= start || s.start >= end {
            out.push(s);
            continue;
        }
        if s.start < start {
            out.push(Segment { end: start, ..s });
        }
        out.push(Segment {
            start: s.start.max(start),
            end: s.end.min(end),
            sky: light.min(s.sky),
            covered: s.covered || covers,
        });
        if s.end > end {
            out.push(Segment { start: end, ..s });
        }
    }
    out
}

/// `LodQuadBuilder`'s merging: faces in one plane with the same block and
/// light that share an edge become one, first along `u`, then along `v`.
pub fn merge_quads(quads: &mut Vec<Quad>) {
    for pass in 0..2 {
        if pass == 0 {
            quads.sort_unstable_by_key(|q| {
                (
                    q.dir,
                    q.plane,
                    q.transparent,
                    q.id,
                    q.sky,
                    q.block,
                    q.lower_top,
                    q.v0,
                    q.v1,
                    q.u0,
                )
            });
        } else {
            quads.sort_unstable_by_key(|q| {
                (
                    q.dir,
                    q.plane,
                    q.transparent,
                    q.id,
                    q.sky,
                    q.block,
                    q.lower_top,
                    q.u0,
                    q.u1,
                    q.v0,
                )
            });
        }
        let mut merged: Vec<Quad> = Vec::with_capacity(quads.len());
        for &q in quads.iter() {
            if let Some(last) = merged.last_mut() {
                let same = last.dir == q.dir
                    && last.plane == q.plane
                    && last.transparent == q.transparent
                    && last.id == q.id
                    && last.sky == q.sky
                    && last.block == q.block
                    && last.lower_top == q.lower_top;
                if same && pass == 0 && last.v0 == q.v0 && last.v1 == q.v1 && last.u1 == q.u0 {
                    last.u1 = q.u1;
                    continue;
                }
                // Lowered side tops may not merge vertically: that would
                // erase the exposed edge at the end of the lower rectangle.
                let vertical_side = !matches!(q.dir, Dir::Up | Dir::Down);
                if same
                    && pass == 1
                    && !(vertical_side && q.lower_top)
                    && last.u0 == q.u0
                    && last.u1 == q.u1
                    && last.v1 == q.v0
                {
                    last.v1 = q.v1;
                    continue;
                }
            }
            merged.push(q);
        }
        *quads = merged;
    }
}

/// The section files: Distant Horizons keeps its full data in an SQLite
/// database (`FullDataSourceV2Repo`); this port keeps one file per section,
/// `<detail>/<x>_<z>.dhs`, zlib-compressed, in the same layout (the id map,
/// each column's generation step and data points).
#[allow(dead_code)]
pub mod storage {
    use super::{FullDataSource, IdMap, gen_step, pos};
    use std::io::{Read, Write};
    use std::path::{Path, PathBuf};

    const MAGIC: &[u8; 4] = b"DHH2";

    pub fn path(dir: &Path, section: i64) -> PathBuf {
        dir.join(pos::detail_level(section).to_string())
            .join(format!("{}_{}.dhs", pos::x(section), pos::z(section)))
    }

    /// `fingerprint` names the game's block and biome registries: a file
    /// written with others is not read back.
    pub fn encode(source: &FullDataSource, fingerprint: u64) -> Vec<u8> {
        let mut raw = Vec::new();
        raw.extend_from_slice(&source.pos.to_le_bytes());
        raw.extend_from_slice(&(source.mapping.len() as u32).to_le_bytes());
        for &(biome, state) in source.mapping.entries() {
            raw.extend_from_slice(&biome.to_le_bytes());
            raw.extend_from_slice(&state.to_le_bytes());
        }
        raw.extend(source.gen_steps.iter().map(|&s| s as u8));
        for column in &source.columns {
            raw.extend_from_slice(&(column.len() as u16).to_le_bytes());
            for p in column {
                raw.extend_from_slice(&p.to_le_bytes());
            }
        }
        raw.extend_from_slice(&(source.stems.len() as u32).to_le_bytes());
        for s in &source.stems {
            raw.extend_from_slice(&s.x.to_le_bytes());
            raw.extend_from_slice(&s.z.to_le_bytes());
            raw.extend_from_slice(&s.min_y.to_le_bytes());
            raw.extend_from_slice(&s.max_y.to_le_bytes());
            raw.extend_from_slice(&s.id.to_le_bytes());
            raw.extend_from_slice(&[s.sky, s.block]);
            raw.extend_from_slice(&s.leaves.to_le_bytes());
            raw.extend_from_slice(&s.crown_max.to_le_bytes());
        }
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&fingerprint.to_le_bytes());
        let mut z = flate2::write::ZlibEncoder::new(out, flate2::Compression::fast());
        let _ = z.write_all(&raw);
        z.finish().unwrap_or_default()
    }

    pub fn decode(bytes: &[u8], fingerprint: u64) -> Option<FullDataSource> {
        if bytes.len() < 12 || &bytes[..4] != MAGIC || bytes[4..12] != fingerprint.to_le_bytes() {
            return None;
        }
        let mut raw = Vec::new();
        flate2::read::ZlibDecoder::new(&bytes[12..])
            .read_to_end(&mut raw)
            .ok()?;
        let mut at = 0usize;
        let mut take = |n: usize| -> Option<&[u8]> {
            let s = raw.get(at..at + n)?;
            at += n;
            Some(s)
        };
        let section = i64::from_le_bytes(take(8)?.try_into().ok()?);
        let mut source = FullDataSource::empty(section);
        let entries = u32::from_le_bytes(take(4)?.try_into().ok()?);
        let mut mapping = IdMap::default();
        for _ in 0..entries {
            let biome = u16::from_le_bytes(take(2)?.try_into().ok()?);
            let state = u32::from_le_bytes(take(4)?.try_into().ok()?);
            mapping.add_if_not_present(biome, state);
        }
        if mapping.len() != entries as usize {
            return None;
        }
        source.mapping = mapping;
        let steps = take(source.gen_steps.len())?.to_vec();
        for (s, b) in source.gen_steps.iter_mut().zip(steps) {
            *s = b as i8;
        }
        for column in &mut source.columns {
            let n = u16::from_le_bytes(take(2)?.try_into().ok()?);
            for _ in 0..n {
                let p = u64::from_le_bytes(take(8)?.try_into().ok()?);
                if super::full_point::id(p) as usize >= entries as usize {
                    return None;
                }
                column.push(p);
            }
        }
        let count = u32::from_le_bytes(take(4)?.try_into().ok()?) as usize;
        if count > super::COLUMNS * super::STEMS_PER_COLUMN * super::full_point::MAX_WORLD_Y_SIZE as usize { return None; }
        for _ in 0..count {
            let stem = super::Stem {
                x: i32::from_le_bytes(take(4)?.try_into().ok()?),
                z: i32::from_le_bytes(take(4)?.try_into().ok()?),
                min_y: u16::from_le_bytes(take(2)?.try_into().ok()?),
                max_y: u16::from_le_bytes(take(2)?.try_into().ok()?),
                id: u32::from_le_bytes(take(4)?.try_into().ok()?),
                sky: take(1)?[0], block: take(1)?[0],
                leaves: u32::from_le_bytes(take(4)?.try_into().ok()?),
                crown_max: u16::from_le_bytes(take(2)?.try_into().ok()?),
            };
            if stem.id as usize >= entries as usize || stem.min_y >= stem.max_y
                || stem.sky > 15 || stem.block > 15 || source.stem_column(&stem).is_none()
                || i32::from(stem.max_y) >= super::full_point::MAX_WORLD_Y_SIZE
                || (stem.leaves != 0 && stem.max_y >= stem.crown_max) { return None; }
            source.stems.push(stem);
        }
        if at != raw.len() { return None; }
        // Old prepare updates could carry LIGHT metadata with empty vectors.
        // Such columns are unavailable, not a proof of a completely air world.
        for (step,column) in source.gen_steps.iter_mut().zip(&source.columns) {
            if column.is_empty() {
                *step=gen_step::EMPTY;
            }
        }
        source.is_empty = source.gen_steps.iter().all(|&s| s == gen_step::EMPTY);
        Some(source)
    }

    pub fn load(dir: &Path, section: i64, fingerprint: u64) -> Option<FullDataSource> {
        let bytes = std::fs::read(path(dir, section)).ok()?;
        decode(&bytes, fingerprint).filter(|s| s.pos == section)
    }

    pub fn save(dir: &Path, source: &FullDataSource, fingerprint: u64) -> std::io::Result<()> {
        let path = path(dir, source.pos);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Written beside, then renamed: a crash never leaves half a file.
        let partial = path.with_extension("dhs.partial");
        std::fs::write(&partial, encode(source, fingerprint))?;
        std::fs::rename(partial, path)
    }
}

/// The mod's settings, one file per key in its data folder, as Horizon's
/// native mods keep them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub enabled: bool,
    /// An index into `DISTANCES`.
    pub distance: usize,
    pub quality: HorizontalQuality,
    /// Distant generation: sample the world generator for terrain not
    /// visited yet (single player and LAN hosts only).
    pub generation: bool,
    pub cave_culling: bool,
    /// Tint each section by its detail level (`LodUtil.DEBUG_DETAIL_LEVEL_COLORS`).
    pub debug: bool,
}

/// LOD render distances, in chunks (`lodChunkRenderDistanceRadius`).
pub const DISTANCES: [i32; 5] = [64, 128, 256, 512, 1024];

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            distance: 2,
            quality: HorizontalQuality::Medium,
            generation: true,
            cave_culling: true,
            debug: false,
        }
    }
}

#[allow(dead_code)]
impl Settings {
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let d = Self::default();
        let flag = |key: &str, default: bool| match get(key).as_deref().map(str::trim) {
            Some("true") => true,
            Some("false") => false,
            _ => default,
        };
        let distance = get("distance")
            .and_then(|v| v.trim().parse::<i32>().ok())
            .and_then(|v| DISTANCES.iter().position(|&d| d == v))
            .unwrap_or(d.distance);
        let quality = get("quality")
            .and_then(|v| HorizontalQuality::ALL.into_iter().find(|q| q.name() == v.trim()))
            .unwrap_or(d.quality);
        Self {
            enabled: flag("enabled", d.enabled),
            distance,
            quality,
            generation: flag("generation", d.generation),
            cave_culling: flag("cave_culling", d.cave_culling),
            debug: flag("debug", d.debug),
        }
    }
    pub fn stored(&self) -> Vec<(&'static str, String)> {
        vec![
            ("enabled", self.enabled.to_string()),
            ("distance", self.radius_chunks().to_string()),
            ("quality", self.quality.name().to_owned()),
            ("generation", self.generation.to_string()),
            ("cave_culling", self.cave_culling.to_string()),
            ("debug", self.debug.to_string()),
        ]
    }
    pub fn radius_chunks(&self) -> i32 {
        DISTANCES[self.distance]
    }
    pub fn labels(&self) -> Vec<String> {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        vec![
            format!("Distant Horizons: {}", on(self.enabled)),
            format!("LOD render distance: {} chunks", self.radius_chunks()),
            format!("Horizontal quality: {}", self.quality.label()),
            format!("Distant generation: {}", on(self.generation)),
            format!("Cave culling: {}", on(self.cave_culling)),
            format!("Debug detail colours: {}", on(self.debug)),
        ]
    }
    pub fn click(&mut self, button: usize) {
        match button {
            0 => self.enabled = !self.enabled,
            1 => self.distance = (self.distance + 1) % DISTANCES.len(),
            2 => {
                let i = HorizontalQuality::ALL
                    .iter()
                    .position(|&q| q == self.quality)
                    .unwrap_or(2);
                self.quality = HorizontalQuality::ALL[(i + 1) % HorizontalQuality::ALL.len()];
            }
            3 => self.generation = !self.generation,
            4 => self.cave_culling = !self.cave_culling,
            5 => self.debug = !self.debug,
            _ => {}
        }
    }
    /// `/dh <args>`: changes a setting; the reply line, or `None` for an
    /// unknown command.
    pub fn command(&mut self, args: &[&str]) -> Option<String> {
        match args {
            ["on"] | ["off"] => {
                self.enabled = args[0] == "on";
                Some(format!("Distant Horizons {}", if self.enabled { "on" } else { "off" }))
            }
            ["distance", n] => {
                let n: i32 = n.parse().ok()?;
                let i = DISTANCES.iter().position(|&d| d >= n).unwrap_or(DISTANCES.len() - 1);
                self.distance = i;
                Some(format!("LOD render distance: {} chunks", self.radius_chunks()))
            }
            ["quality", q] => {
                self.quality = HorizontalQuality::ALL.into_iter().find(|h| h.name() == *q)?;
                Some(format!("Horizontal quality: {}", self.quality.label()))
            }
            ["generation", v] | ["caves", v] | ["debug", v] => {
                let on = match *v {
                    "on" => true,
                    "off" => false,
                    _ => return None,
                };
                match args[0] {
                    "generation" => self.generation = on,
                    "caves" => self.cave_culling = on,
                    _ => self.debug = on,
                }
                Some(format!("{} {}", args[0], v))
            }
            _ => None,
        }
    }
}

/// `/dh`'s words, for chat completion: each word after `dh` and the values
/// it takes (none: the word alone). `Settings::command` and the bridge's
/// `status`, `save` and `clear` accept every one of them.
pub const COMMANDS: &[(&str, &[&str])] = &[
    ("status", &[]),
    ("save", &[]),
    ("clear", &[]),
    ("on", &[]),
    ("off", &[]),
    ("distance", &["64", "128", "256", "512", "1024"]),
    ("quality", &["lowest", "low", "medium", "high", "extreme"]),
    ("generation", &["on", "off"]),
    ("caves", &["on", "off"]),
    ("debug", &["on", "off"]),
];

/// `LodUtil.DEBUG_DETAIL_LEVEL_COLORS`, by data detail level.
pub const DEBUG_DETAIL_LEVEL_COLORS: [u32; 13] = [
    0xFF0000, 0xFF7F00, 0xFFFF00, 0x7FFF00, 0x00FF00, 0x00FF7F, 0x00FFFF, 0x007FFF, 0x0000FF, 0x7F00FF, 0xFF00FF,
    0xFF7FFF, 0xFFFFFF,
];

/// What the engine needs of the game. Implemented on Horizon in bridge.rs,
/// and on a small fake in the tests.
pub struct TintNeighborhood<'a> {
    pub source: &'a FullDataSource,
    /// N, S, W, E, NW, NE, SW, SE at source's detail level.
    pub neighbors: [Option<&'a FullDataSource>; 8],
}

pub struct MeshNeighborhood<'a> {
    pub neighbors: [Option<&'a FullDataSource>; 8],
    /// Selected, potentially touching coarser tiles, ordered coarsest first.
    pub coarser: &'a [TintNeighborhood<'a>],
}

pub trait Host: Send + Sync + 'static {
    type Mesh: Send + 'static;
    fn blocks(&self) -> &BlockTable;
    /// The world's minimum y and its exclusive maximum.
    fn build_height(&self) -> (i32, i32);
    /// A sampler of the world generator, for distant generation.
    fn sampler(&self) -> Option<Box<dyn ColumnSampler>>;
    /// The faces of a section, made into the game's mesh.
    fn mesh(&self, section: i64, source: &FullDataSource, quads: &[Quad], debug: bool) -> Self::Mesh;
    /// Existing halo data for tint interpolation; no generation is requested.
    /// Order: N, S, W, E, NW, NE, SW, SE. Non-tint hosts keep the old API.
    fn mesh_with_neighbors(
        &self,
        section: i64,
        source: &FullDataSource,
        quads: &[Quad],
        debug: bool,
        _neighborhood: MeshNeighborhood<'_>,
    ) -> Self::Mesh {
        self.mesh(section, source, quads, debug)
    }
    /// A host may subdivide colour transitions or emit material overlays.
    /// Report those actual faces, while simpler hosts keep the input count.
    fn mesh_quad_count(&self, _mesh: &Self::Mesh, input_quads: usize) -> usize {
        input_quads
    }
}

/// The LOD world (`DhClientLevel` with its `FullDataSourceProviderV2`,
/// `LodQuadTree` and `WorldGenerationQueue`): full data kept in memory and
/// on disk, updated from visited chunks and the generator by worker
/// threads, which also mesh the sections the player should see.
pub mod engine {
    use super::*;
    use std::collections::{BinaryHeap, HashMap, HashSet};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Condvar, Mutex};
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    /// One owned session may finish joining its workers and saving derived data
    /// away from the frame thread. A new session must wait for `ready`: even a
    /// rapid off/on cannot create two writers for the same world cache.
    pub struct Retirement<T: Send + 'static> {
        sender: Option<std::sync::mpsc::SyncSender<T>>,
        worker: Option<JoinHandle<()>>,
        busy: Arc<AtomicBool>,
        failed: Arc<AtomicBool>,
    }

    impl<T: Send + 'static> Retirement<T> {
        pub fn new() -> std::io::Result<Self> {
            let (sender, receiver) = std::sync::mpsc::sync_channel::<T>(1);
            let busy = Arc::new(AtomicBool::new(false));
            let failed = Arc::new(AtomicBool::new(false));
            let (working, failure) = (busy.clone(), failed.clone());
            let worker = std::thread::Builder::new()
                .name("Distant Horizons retirement".into())
                .spawn(move || {
                    while let Ok(value) = receiver.recv() {
                        // Reset busy even when a destructor panics. Keep a
                        // separate failure latch: do not reopen an uncertain
                        // cache or silently treat an incomplete save as ready.
                        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(value))).is_err() {
                            failure.store(true, Ordering::Release);
                        }
                        working.store(false, Ordering::Release);
                    }
                })?;
            Ok(Self { sender: Some(sender), worker: Some(worker), busy, failed })
        }

        pub fn ready(&self) -> bool {
            !self.busy()
                && !self.failed()
                && self.worker.as_ref().is_some_and(|worker| !worker.is_finished())
        }

        pub fn busy(&self) -> bool {
            self.busy.load(Ordering::Acquire)
        }

        pub fn failed(&self) -> bool {
            self.failed.load(Ordering::Acquire)
        }

        /// Never waits for the worker or for channel capacity. A rejected value
        /// stays owned by the caller; it must not be dropped on a live frame.
        pub fn try_retire(&mut self, value: T) -> Result<(), T> {
            if !self.ready()
                || self.busy.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err()
            {
                return Err(value);
            }
            match self.sender.as_ref().expect("live retirement sender").try_send(value) {
                Ok(()) => Ok(()),
                Err(std::sync::mpsc::TrySendError::Full(value) | std::sync::mpsc::TrySendError::Disconnected(value)) => {
                    self.failed.store(true, Ordering::Release);
                    self.busy.store(false, Ordering::Release);
                    Err(value)
                }
            }
        }

        /// Only the explicit application-exit hook may wait. Closing the sender
        /// drains the accepted session before Windows tears its threads down.
        pub fn finish(&mut self) -> bool {
            self.sender.take();
            if let Some(worker) = self.worker.take()
                && worker.join().is_err()
            {
                self.failed.store(true, Ordering::Release);
            }
            self.busy.store(false, Ordering::Release);
            !self.failed()
        }
    }

    impl<T: Send + 'static> Drop for Retirement<T> {
        fn drop(&mut self) {
            self.finish();
        }
    }

    /// A finished section mesh, or the news that a section has nothing to
    /// draw.
    pub struct Meshed<M> {
        pub section: i64,
        pub mesh: Option<M>,
        /// The exact holes used by this worker, checked before publishing:
        /// near meshes may arrive while its far mesh is still being built.
        cutout: Option<Vec<u64>>,
        /// Source, neighbor, settings and coverage requests share this stamp.
        /// Two workers can finish a section out of order.
        revision: u64,
        quad_count: usize,
    }

    /// The near area the game draws itself: GPU-complete chunk columns
    /// within its render distance. CPU-loaded chunks alone are insufficient.
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct Near {
        pub center: (i32, i32),
        pub radius: i32,
        pub loaded: HashSet<(i32, i32)>,
    }

    impl Near {
        /// Whether the game draws chunk `(x, z)`: vanilla's cylindrical
        /// render-distance test (`ChunkTrackingView.isWithinDistance`).
        pub fn covers(&self, x: i32, z: i32) -> bool {
            let dx = (x - self.center.0).abs().max(1) - 1;
            let dz = (z - self.center.1).abs().max(1) - 1;
            let r = self.radius;
            dx * dx + dz * dz < r * r && self.loaded.contains(&(x, z))
        }

        fn section_mask(&self, section: i64) -> Option<Vec<u64>> {
            let (x0, z0) = (
                pos::min_corner_block_x(section) >> 4,
                pos::min_corner_block_z(section) >> 4,
            );
            let n = (pos::block_width(section) >> 4).max(1);
            // Wider data columns never cut holes around individual chunks.
            if n > 64 {
                return None;
            }
            let mut mask = vec![0u64; (n as usize * n as usize).div_ceil(64)];
            for dx in 0..n {
                for dz in 0..n {
                    if self.covers(x0 + dx, z0 + dz) {
                        let index = (dx * n + dz) as usize;
                        mask[index / 64] |= 1 << (index % 64);
                    }
                }
            }
            Some(mask)
        }
    }

    /// The count of sections each kind of work is waiting on, and done.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Stats {
        pub wanted: usize,
        pub shown: usize,
        pub queued: usize,
        pub loaded: usize,
        pub generated: usize,
        pub chunks: usize,
        pub refined: usize,
        /// Faces in the wanted sections' meshes.
        pub quads: usize,
    }

    #[derive(PartialEq, Eq)]
    enum Job {
        /// A visited chunk's columns.
        Chunk(u64),
        /// Load or generate a section, then mesh it.
        Section(i64),
        /// Mesh a section again (its data or the near area changed).
        Mesh(i64),
        /// Merge a section's changed children into it
        /// (`FullDataUpdatePropagatorV2`, batched as it batches them).
        Propagate(i64),
        /// Evict persisted cache data off the render thread.
        Trim,
        Save,
    }

    #[derive(PartialEq, Eq)]
    struct Queued {
        priority: i64,
        sequence: u64,
        job: Job,
    }
    impl Ord for Queued {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            // The lowest priority first, then the oldest.
            other
                .priority
                .cmp(&self.priority)
                .then(other.sequence.cmp(&self.sequence))
        }
    }
    impl PartialOrd for Queued {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    struct Queue {
        heap: BinaryHeap<Queued>,
        sequence: u64,
        /// Sections to load or generate, queued or being worked on.
        sections: HashSet<i64>,
        /// One follow-up mesh if coverage/data changes during a Section job.
        remesh_after_section: HashSet<i64>,
        /// Sections queued to mesh again.
        meshes: HashSet<i64>,
        /// Latest requested appearance, including requests coalesced by dedup.
        /// Globally increasing stamps prevent a removed/re-added section from
        /// accidentally accepting an old result with a reused local counter.
        mesh_revisions: HashMap<i64, u64>,
        /// Section keys read for a mesh's tint halo, including missing data.
        /// Registered under the store lock before a changed source can publish.
        mesh_dependencies: HashMap<i64, Vec<i64>>,
        next_mesh_revision: u64,
        /// Sections queued to take their children's changes: which children
        /// (bit `child index`) changed.
        propagate: HashMap<i64, u8>,
        chunks: HashMap<u64, Box<dyn ChunkView>>,
        next_chunk: u64,
        trim_queued: bool,
    }

    impl Queue {
        fn invalidate_mesh(&mut self, section: i64) {
            self.next_mesh_revision = self.next_mesh_revision.wrapping_add(1);
            self.mesh_revisions.insert(section, self.next_mesh_revision);
        }
        /// A delayed old job may reach its snapshot after a newer mesh. It must
        /// not replace that mesh's dependency set merely because its own final
        /// mesh will be rejected later. Caller also holds the store lock.
        fn register_mesh_dependencies(&mut self, section: i64, revision: u64, dependencies: Vec<i64>) -> bool {
            if self.mesh_revisions.get(&section) != Some(&revision) {
                return false;
            }
            self.mesh_dependencies.insert(section, dependencies);
            true
        }
    }

    struct Store {
        sections: HashMap<i64, Arc<FullDataSource>>,
        dirty: HashSet<i64>,
    }

    /// The settings the workers read, set from the game thread.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct WorkSettings {
        pub render: RenderSettings,
        pub generation: bool,
        pub debug: bool,
    }

    struct Shared<H: Host> {
        host: H,
        dir: Option<PathBuf>,
        fingerprint: u64,
        queue: Mutex<Queue>,
        ready: Condvar,
        store: Mutex<Store>,
        wanted: Mutex<HashSet<i64>>,
        near: Mutex<Arc<Near>>,
        settings: Mutex<WorkSettings>,
        done: Mutex<Vec<Meshed<H::Mesh>>>,
        /// Missing server data is retried with backoff, even while standing still.
        retry: Mutex<HashMap<i64, Instant>>,
        stop: AtomicBool,
        generated: AtomicUsize,
        /// Each meshed section's face count.
        quads: Mutex<HashMap<i64, usize>>,
        chunks: AtomicUsize,
        refined: AtomicUsize,
        /// Jobs being worked on.
        active: AtomicUsize,
    }

    pub struct Engine<H: Host> {
        shared: Arc<Shared<H>>,
        threads: Vec<JoinHandle<()>>,
        plan: Option<DetailPlan>,
        planned_at: Option<(Instant, [f64; 2])>,
        wanted: HashSet<i64>,
        /// Sections with a mesh in the game, and those it still shows that
        /// are no longer wanted (removed once what replaces them is in).
        shown: HashSet<i64>,
        retiring: HashSet<i64>,
        /// Sections meshed with nothing to draw.
        blank: HashSet<i64>,
        // Exact column positions, not their count: equal-sized near areas
        // moving through one section must still move the holes in its mesh.
        near_sections: HashMap<i64, Vec<u64>>,
        /// Holes in meshes already submitted to the renderer, including tiles
        /// still standing in while a changed quality/distance plan is prepared.
        published_cutouts: HashMap<i64, Option<Vec<u64>>>,
        renderer_cutout: HashSet<(i32, i32)>,
        cutout_dirty: bool,
        last_save: Instant,
        /// Where the player was last, for the order of work.
        player: [f64; 2],
    }

    /// Worker threads, as Horizon's far terrain picks them for its own
    /// sampling on 12 or more threads (up to 6), else a third, 2 to 4.
    pub fn worker_threads(cores: usize) -> usize {
        if cores >= 12 {
            (cores / 2).min(6)
        } else {
            (cores / 3).clamp(2, 4)
        }
    }

    impl<H: Host> Engine<H> {
        /// `dir`: where the sections are kept (none: in memory only).
        pub fn new(host: H, dir: Option<PathBuf>, fingerprint: u64, settings: WorkSettings, threads: usize) -> Self {
            let shared = Arc::new(Shared {
                host,
                dir,
                fingerprint,
                queue: Mutex::new(Queue {
                    heap: BinaryHeap::new(),
                    sequence: 0,
                    sections: HashSet::new(),
                    remesh_after_section: HashSet::new(),
                    meshes: HashSet::new(),
                    mesh_revisions: HashMap::new(),
                    mesh_dependencies: HashMap::new(),
                    next_mesh_revision: 0,
                    propagate: HashMap::new(),
                    chunks: HashMap::new(),
                    next_chunk: 0,
                    trim_queued: false,
                }),
                ready: Condvar::new(),
                store: Mutex::new(Store {
                    sections: HashMap::new(),
                    dirty: HashSet::new(),
                }),
                wanted: Mutex::new(HashSet::new()),
                near: Mutex::new(Arc::new(Near::default())),
                settings: Mutex::new(settings),
                done: Mutex::new(Vec::new()),
                retry: Mutex::new(HashMap::new()),
                stop: AtomicBool::new(false),
                generated: AtomicUsize::new(0),
                chunks: AtomicUsize::new(0),
                refined: AtomicUsize::new(0),
                active: AtomicUsize::new(0),
                quads: Mutex::new(HashMap::new()),
            });
            let threads = (0..threads.max(1))
                .map(|i| {
                    let shared = shared.clone();
                    std::thread::Builder::new()
                        .name(format!("Distant Horizons worker {i}"))
                        .spawn(move || work(&shared))
                        .expect("spawn a Distant Horizons worker")
                })
                .collect();
            Self {
                shared,
                threads,
                plan: None,
                planned_at: None,
                wanted: HashSet::new(),
                shown: HashSet::new(),
                retiring: HashSet::new(),
                blank: HashSet::new(),
                near_sections: HashMap::new(),
                published_cutouts: HashMap::new(),
                renderer_cutout: HashSet::new(),
                cutout_dirty: false,
                last_save: Instant::now(),
                player: [0.0; 2],
            }
        }

        /// New settings: everything is meshed again when they change how
        /// sections look.
        pub fn set_settings(&mut self, settings: WorkSettings) {
            let old = std::mem::replace(&mut *self.shared.settings.lock().unwrap(), settings);
            if old.render != settings.render || old.debug != settings.debug {
                for &s in self.wanted.iter() {
                    push(&self.shared, Job::Mesh(s), 0);
                }
            }
            if !old.generation && settings.generation {
                for &s in self.wanted.iter() {
                    push(&self.shared, Job::Section(s), 0);
                }
            }
        }

        /// A visited chunk, to put in the LODs.
        pub fn add_chunk(&mut self, chunk: Box<dyn ChunkView>) {
            // Nearest first, among the sections' work: neither starves the
            // other.
            let dx = f64::from(chunk.chunk_x() * 16 + 8) - self.player[0];
            let dz = f64::from(chunk.chunk_z() * 16 + 8) - self.player[1];
            let priority = dx.hypot(dz) as i64;
            let mut q = self.shared.queue.lock().unwrap();
            let id = q.next_chunk;
            q.next_chunk += 1;
            q.chunks.insert(id, chunk);
            let sequence = q.sequence;
            q.sequence += 1;
            q.heap.push(Queued {
                priority,
                sequence,
                job: Job::Chunk(id),
            });
            drop(q);
            self.shared.ready.notify_one();
        }

        /// Once a frame: plans the sections to draw around `player` (block
        /// x, z) when it moved or `plan` changed, queues their work, and
        /// returns the meshes finished since and the sections to take away.
        pub fn update(&mut self, player: [f64; 2], plan: DetailPlan, near: Near) -> (Vec<Meshed<H::Mesh>>, Vec<i64>) {
            let now = Instant::now();
            self.player = player;
            let near_changed = **self.shared.near.lock().unwrap() != near;
            self.cutout_dirty |= near_changed;
            let moved = self.planned_at.is_none_or(|(at, p)| {
                let d = (p[0] - player[0]).hypot(p[1] - player[1]);
                d >= 16.0 || (d > 0.0 && now - at > Duration::from_secs(2))
            });
            if self.plan != Some(plan) || moved || near_changed {
                if near_changed {
                    *self.shared.near.lock().unwrap() = Arc::new(near.clone());
                }
                self.replan(player, plan, &near);
                self.planned_at = Some((now, player));
            }
            if now - self.last_save > Duration::from_secs(10) {
                self.last_save = now;
                push(&self.shared, Job::Save, i64::MAX);
            }
            if self.shared.settings.lock().unwrap().generation {
                let mut retry = self.shared.retry.lock().unwrap();
                retry.retain(|s, _| self.wanted.contains(s));
                let mut due: Vec<_> = retry
                    .iter()
                    .filter(|(_, at)| **at <= now)
                    .map(|(&s, &at)| (s, at))
                    .collect();
                due.sort_unstable_by_key(|&(s, at)| (at, s));
                // Bound retries per frame; permanent refusals cannot monopolize
                // the queue because retry times move forward after each attempt.
                for (s, _) in due.into_iter().take(4) {
                    retry.remove(&s);
                    push(&self.shared, Job::Section(s), 0);
                }
            }
            let done = std::mem::take(&mut *self.shared.done.lock().unwrap());
            let mut meshes = Vec::with_capacity(done.len());
            for m in done {
                if !self.wanted.contains(&m.section) {
                    continue;
                }
                if self.shared.queue.lock().unwrap().mesh_revisions.get(&m.section) != Some(&m.revision) {
                    // A newer data/neighbor/settings request already has a
                    // queued, active or completed replacement. Keep the last
                    // published mesh; requeueing here would invalidate that
                    // replacement again and could starve publication.
                    continue;
                }
                let same_cutout = match self.near_sections.get(&m.section) {
                    Some(expected) => m.cutout.as_ref() == Some(expected),
                    None => m.cutout.as_ref().is_none_or(|mask| mask.iter().all(|&bits| bits == 0)),
                };
                if !same_cutout {
                    // The worker finished an older coverage snapshot. Keep
                    // the visible mesh and rebuild from the latest one;
                    // never publish a hole where its near chunk has gone.
                    push(&self.shared, Job::Mesh(m.section), 0);
                    continue;
                }
                if m.mesh.is_some() {
                    self.shown.insert(m.section);
                    self.blank.remove(&m.section);
                } else {
                    self.shown.remove(&m.section);
                    self.blank.insert(m.section);
                }
                self.published_cutouts.insert(m.section, m.cutout.clone());
                self.shared.quads.lock().unwrap().insert(m.section, m.quad_count);
                self.cutout_dirty = true;
                meshes.push(m);
            }
            // A retiring section goes once every wanted section over it has
            // its mesh in (or has nothing to draw): no holes meanwhile.
            let retry = self.shared.retry.lock().unwrap();
            let ready = |s: &i64| !retry.contains_key(s) && (self.shown.contains(s) || self.blank.contains(s));
            let removed: Vec<i64> = self
                .retiring
                .iter()
                .copied()
                .filter(|&old| {
                    self.wanted
                        .iter()
                        .filter(|&&w| pos::contains(old, w) || pos::contains(w, old))
                        .all(ready)
                })
                .collect();
            for s in &removed {
                self.retiring.remove(s);
                self.shown.remove(s);
                self.published_cutouts.remove(s);
                self.cutout_dirty = true;
            }
            (meshes, removed)
        }

        fn replan(&mut self, player: [f64; 2], plan: DetailPlan, near: &Near) {
            let fully_near = |s: i64| {
                let (x0, z0) = (pos::min_corner_block_x(s) >> 4, pos::min_corner_block_z(s) >> 4);
                let n = pos::block_width(s) >> 4;
                n <= 64 && (0..n.max(1)).all(|dx| (0..n.max(1)).all(|dz| near.covers(x0 + dx, z0 + dz)))
            };
            let wanted: HashSet<i64> = plan.sections(player, fully_near).into_iter().collect();
            let distance = |s: i64| {
                let dx = f64::from(pos::center_block_x(s)) - player[0];
                let dz = f64::from(pos::center_block_z(s)) - player[1];
                (dx * dx + dz * dz).sqrt() as i64
            };
            // Sections across the near area's edge are meshed again when the
            // chunks it holds there change.
            let mut near_sections = HashMap::new();
            for &s in &wanted {
                let Some(mask) = near.section_mask(s) else { continue };
                if mask.iter().any(|&bits| bits != 0) || self.near_sections.contains_key(&s) {
                    near_sections.insert(s, mask);
                }
            }
            // Workers skip sections no longer wanted: they must see these
            // as wanted before their jobs are queued.
            *self.shared.wanted.lock().unwrap() = wanted.clone();
            for &s in &wanted {
                let shown = self.shown.contains(&s) || self.blank.contains(&s);
                if self.plan.is_none() || (!self.wanted.contains(&s) && !shown) {
                    push(&self.shared, Job::Section(s), distance(s));
                } else if coarser_tint_sources(s, &self.wanted) != coarser_tint_sources(s, &wanted)
                    || (shown && (!self.published_cutouts.contains_key(&s)
                        || !same_cutout(
                            self.published_cutouts.get(&s).and_then(Option::as_ref),
                            near_sections.get(&s),
                        )))
                {
                    push(&self.shared, Job::Mesh(s), distance(s));
                }
            }
            for &s in self.shown.iter() {
                if !wanted.contains(&s) {
                    self.retiring.insert(s);
                }
            }
            self.retiring.retain(|s| !wanted.contains(s));
            self.blank.retain(|s| wanted.contains(s));
            self.published_cutouts.retain(|s, _| self.shown.contains(s) || self.blank.contains(s));
            self.wanted = wanted;
            {
                let mut queue = self.shared.queue.lock().unwrap();
                let Queue { mesh_revisions, mesh_dependencies, sections, meshes, .. } = &mut *queue;
                mesh_revisions.retain(|s,_| self.wanted.contains(s) || sections.contains(s) || meshes.contains(s));
                mesh_dependencies.retain(|s,_| mesh_revisions.contains_key(s));
            }
            self.near_sections = near_sections;
            self.plan = Some(plan);
            // Dropping hundreds of Vec-heavy sections here stalled a game tick.
            push(&self.shared, Job::Trim, -1);
        }

        /// Commit with the same mesh/removal batch. Horizon retains departing
        /// detailed chunks until upload_lods has uploaded their replacements.
        pub fn take_cutout(&mut self) -> Option<HashSet<(i32, i32)>> {
            if !std::mem::take(&mut self.cutout_dirty) {
                return None;
            }
            let near = self.shared.near.lock().unwrap();
            let mut covered: HashSet<_> = near.loaded.iter().copied()
                .filter(|&(x,z)| near.covers(x,z)).collect();
            for (&section, mask) in &self.published_cutouts {
                let Some(mask) = mask else { continue };
                let n = (pos::block_width(section) >> 4).max(1);
                let (x0,z0) = (pos::min_corner_block_x(section) >> 4, pos::min_corner_block_z(section) >> 4);
                for (word_index, &word) in mask.iter().enumerate() {
                    let mut bits = word;
                    while bits != 0 {
                        let bit = bits.trailing_zeros() as usize;
                        bits &= bits - 1;
                        let index = (word_index * 64 + bit) as i32;
                        if index < n * n {
                            covered.insert((x0 + index / n, z0 + index % n));
                        }
                    }
                }
            }
            // A fully detailed tile was skipped by replan and has no published
            // mask at all. After the near radius shrinks, its old detailed
            // columns still stand in until their first far mesh is submitted.
            // Checking the containing node at each level is bounded by nine
            // lookups per old column, not columns times all wanted tiles.
            for &(x, z) in &self.renderer_cutout {
                if !covered.contains(&(x, z)) && self.awaiting_far_column(x, z) {
                    covered.insert((x, z));
                }
            }
            if covered == self.renderer_cutout {
                return None;
            }
            self.renderer_cutout = covered.clone();
            Some(covered)
        }

        fn awaiting_far_column(&self, x: i32, z: i32) -> bool {
            for level in pos::SECTION_MINIMUM_DETAIL_LEVEL..=pos::ROOT_DETAIL_LEVEL {
                let n = 1 << (level - 4);
                let section = pos::encode(level, x.div_euclid(n), z.div_euclid(n));
                if !self.wanted.contains(&section) {
                    continue;
                }
                return match self.published_cutouts.get(&section) {
                    None => true,
                    Some(None) => false,
                    Some(Some(mask)) => {
                        let index = (x.rem_euclid(n) * n + z.rem_euclid(n)) as usize;
                        mask.get(index / 64).is_some_and(|bits| bits & (1 << (index % 64)) != 0)
                    }
                };
            }
            // No wanted tile covers this column: it left DH's range, so it
            // must not retain near GPU geometry indefinitely after movement.
            false
        }

        pub fn host(&self) -> &H {
            &self.shared.host
        }

        /// Every section the game shows, to clear them.
        pub fn shown_sections(&self) -> impl Iterator<Item = i64> + '_ {
            self.shown.iter().copied()
        }

        pub fn stats(&self) -> Stats {
            let queued = self.shared.queue.lock().unwrap().heap.len() + self.shared.active.load(Ordering::Relaxed);
            Stats {
                wanted: self.wanted.len(),
                shown: self.shown.len(),
                queued,
                loaded: self.shared.store.lock().unwrap().sections.len(),
                generated: self.shared.generated.load(Ordering::Relaxed),
                chunks: self.shared.chunks.load(Ordering::Relaxed),
                refined: self.shared.refined.load(Ordering::Relaxed),
                quads: {
                    let quads = self.shared.quads.lock().unwrap();
                    self.wanted.iter().filter_map(|s| quads.get(s)).sum()
                },
            }
        }

        /// Writes every changed section now.
        pub fn save(&self) {
            save_dirty(&self.shared);
        }

        /// Forgets every section, in memory and on disk.
        pub fn wipe(&mut self) {
            {
                let mut queue = self.shared.queue.lock().unwrap();
                let sections: Vec<_> = queue.mesh_revisions.keys().copied().collect();
                for section in sections { queue.invalidate_mesh(section); }
                queue.mesh_dependencies.clear();
            }
            {
                let mut store = self.shared.store.lock().unwrap();
                store.sections.clear();
                store.dirty.clear();
            }
            if let Some(dir) = &self.shared.dir {
                let _ = std::fs::remove_dir_all(dir);
            }
            self.retiring.extend(self.shown.iter().copied());
            self.blank.clear();
            self.planned_at = None;
            self.plan = None;
            self.wanted.clear();
            self.cutout_dirty = true;
        }
    }

    fn same_cutout(a: Option<&Vec<u64>>, b: Option<&Vec<u64>>) -> bool {
        let nonempty = |mask: &&Vec<u64>| mask.iter().any(|&bits| bits != 0);
        a.filter(nonempty) == b.filter(nonempty)
    }

    impl<H: Host> Drop for Engine<H> {
        fn drop(&mut self) {
            self.shared.stop.store(true, Ordering::Relaxed);
            self.shared.ready.notify_all();
            for t in self.threads.drain(..) {
                let _ = t.join();
            }
            // Changes still on their way up the levels are carried to the top
            // first (Distant Horizons keeps them in its database for later):
            // finer levels first, as theirs add up in the coarser ones.
            loop {
                let next = {
                    let mut q = self.shared.queue.lock().unwrap();
                    let next = q.propagate.keys().copied().min_by_key(|&s| pos::detail_level(s));
                    next.map(|s| (s, q.propagate.remove(&s).unwrap_or(0)))
                };
                let Some((s, children)) = next else { break };
                propagate(&self.shared, s, children);
            }
            save_dirty(&self.shared);
        }
    }

    fn push<H: Host>(shared: &Shared<H>, job: Job, priority: i64) {
        let mut q = shared.queue.lock().unwrap();
        if let Job::Mesh(s) = job {
            // Even a coalesced request invalidates the snapshot of an active
            // worker. The already queued job reads the latest stamp on start.
            q.invalidate_mesh(s);
        }
        let duplicate = match job {
            Job::Section(s) => !q.sections.insert(s),
            Job::Mesh(s) => {
                if q.sections.contains(&s) {
                    q.remesh_after_section.insert(s);
                    return;
                }
                !q.meshes.insert(s)
            }
            Job::Trim => std::mem::replace(&mut q.trim_queued, true),
            _ => false,
        };
        if duplicate {
            return;
        }
        if let Job::Section(s) = job {
            q.invalidate_mesh(s);
        }
        let sequence = q.sequence;
        q.sequence += 1;
        q.heap.push(Queued {
            priority,
            sequence,
            job,
        });
        drop(q);
        shared.ready.notify_one();
    }

    fn work<H: Host>(shared: &Shared<H>) {
        let mut sampler: Option<Box<dyn ColumnSampler>> = None;
        loop {
            if let Some(sampler) = &mut sampler {
                let near = shared.near.lock().unwrap().clone();
                sampler.view([near.center.0 * 16, near.center.1 * 16], near.radius);
                if shared.settings.lock().unwrap().generation {
                    for refined in sampler.drain_refined() {
                        let target = refined.pos;
                        if apply(shared, target, &refined) {
                            shared.refined.fetch_add(1, Ordering::Relaxed);
                            changed(shared, target);
                        }
                    }
                }
            }
            let next = {
                let mut q = shared.queue.lock().unwrap();
                loop {
                    if shared.stop.load(Ordering::Relaxed) {
                        return;
                    }
                    if let Some(next) = q.heap.pop() {
                        // Counted before the queue is let go: never neither.
                        shared.active.fetch_add(1, Ordering::Relaxed);
                        let chunk = match next.job {
                            Job::Chunk(id) => q.chunks.remove(&id),
                            _ => None,
                        };
                        break Some((next.job, chunk));
                    }
                    let (waiting, timed) = shared.ready.wait_timeout(q, Duration::from_millis(250)).unwrap();
                    q = waiting;
                    if timed.timed_out() {
                        break None;
                    }
                }
            };
            let Some((job, chunk)) = next else { continue };
            match job {
                Job::Chunk(_) => {
                    if let Some(chunk) = chunk {
                        add_chunk(shared, &*chunk);
                    }
                }
                Job::Section(s) => {
                    let settings = *shared.settings.lock().unwrap();
                    // Asked again until there is one: the game's may come
                    // after the session starts (a world still loading).
                    if settings.generation && sampler.is_none() {
                        sampler = shared.host.sampler();
                    }
                    let generator = match &mut sampler {
                        Some(s) if settings.generation => Some(&mut **s as &mut dyn ColumnSampler),
                        _ => None,
                    };
                    if shared.wanted.lock().unwrap().contains(&s) {
                        prepare(shared, s, generator);
                        mesh(shared, s);
                        let missing = shared
                            .store
                            .lock()
                            .unwrap()
                            .sections
                            .get(&s)
                            .is_none_or(|data| data.missing_columns() > 0);
                        if settings.generation && missing {
                            shared
                                .retry
                                .lock()
                                .unwrap()
                                .insert(s, Instant::now() + Duration::from_secs(2));
                        } else {
                            shared.retry.lock().unwrap().remove(&s);
                        }
                    }
                    let remesh = {
                        let mut queue = shared.queue.lock().unwrap();
                        queue.sections.remove(&s);
                        queue.remesh_after_section.remove(&s)
                    };
                    if remesh {
                        push(shared, Job::Mesh(s), 0);
                    }
                }
                Job::Mesh(s) => {
                    shared.queue.lock().unwrap().meshes.remove(&s);
                    if shared.wanted.lock().unwrap().contains(&s) {
                        mesh(shared, s);
                    }
                }
                Job::Propagate(s) => {
                    let children = shared.queue.lock().unwrap().propagate.remove(&s).unwrap_or(0);
                    propagate(shared, s, children);
                }
                Job::Save => save_dirty(shared),
                Job::Trim => {
                    trim_store(shared);
                    shared.queue.lock().unwrap().trim_queued = false;
                }
            }
            shared.active.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// A section from memory, else from its file, else empty.
    fn section<H: Host>(shared: &Shared<H>, store: &mut Store, s: i64) -> Arc<FullDataSource> {
        if let Some(found) = store.sections.get(&s) {
            return found.clone();
        }
        let loaded = shared
            .dir
            .as_ref()
            .and_then(|dir| storage::load(dir, s, shared.fingerprint))
            .unwrap_or_else(|| FullDataSource::empty(s));
        let loaded = Arc::new(loaded);
        store.sections.insert(s, loaded.clone());
        loaded
    }

    /// Borders may belong to a cached tile that is not currently selected by
    /// the detail plan. Read existing data without generating extra tiles or
    /// inserting empty neighbours into the store.
    fn cached_neighbor<H: Host>(shared: &Shared<H>, store: &mut Store, s: i64) -> Option<Arc<FullDataSource>> {
        if let Some(found) = store.sections.get(&s) {
            return Some(found.clone());
        }
        let loaded = shared.dir.as_ref().and_then(|dir| storage::load(dir, s, shared.fingerprint))?;
        let loaded = Arc::new(loaded);
        store.sections.insert(s, loaded.clone());
        Some(loaded)
    }

    /// A completed neighbour changes side visibility even when this tile's
    /// own columns and the player's near cutout have not changed.
    fn remesh_neighbors<H: Host>(shared: &Shared<H>, s: i64) {
        let mut affected: HashSet<_> = shared.queue.lock().unwrap().mesh_dependencies.iter()
            .filter(|(target, dependencies)| **target != s && dependencies.binary_search(&s).is_ok())
            .map(|(&target, _)| target).collect();
        affected.extend(TINT_NEIGHBORS.map(|(dx, dz)| pos::adjacent(s, dx, dz)));
        let wanted = shared.wanted.lock().unwrap();
        for neighbor in affected {
            if wanted.contains(&neighbor) {
                push(shared, Job::Mesh(neighbor), 0);
            }
        }
    }

    /// The chunk into its leaf section; the coarser levels take the change
    /// later, batched (`Propagate` jobs).
    fn add_chunk<H: Host>(shared: &Shared<H>, chunk: &dyn ChunkView) {
        let input = create_from_chunk(chunk, shared.host.blocks());
        shared.chunks.fetch_add(1, Ordering::Relaxed);
        if apply(shared, input.pos, &input) {
            changed(shared, input.pos);
        }
    }

    /// Merges `input` into section `target` without holding the store while
    /// it works: on a copy, put back unless another worker changed the
    /// section meanwhile (then again, on the newer one). Returns whether it
    /// changed.
    fn apply<H: Host>(shared: &Shared<H>, target: i64, input: &FullDataSource) -> bool {
        let blocks = shared.host.blocks();
        loop {
            let current = section(shared, &mut shared.store.lock().unwrap(), target);
            let mut updated = (*current).clone();
            if !updated.update_from(input, blocks) {
                return false;
            }
            let mut store = shared.store.lock().unwrap();
            if store.sections.get(&target).is_some_and(|s| Arc::ptr_eq(s, &current)) {
                store.sections.insert(target, Arc::new(updated));
                store.dirty.insert(target);
                invalidate_data_appearance(shared, target);
                return true;
            }
        }
    }

    /// Called while holding the store lock: no worker can read the replacement
    /// data without its appearance stamp changing first. Requests alone are not
    /// sufficient when an earlier queued Mesh starts during Section::prepare.
    /// The caller still schedules work through changed/prepare as before.
    fn invalidate_data_appearance<H: Host>(shared: &Shared<H>, section: i64) {
        let mut queue = shared.queue.lock().unwrap();
        let mut affected: HashSet<_> = queue.mesh_dependencies.iter()
            .filter(|(_, dependencies)| dependencies.binary_search(&section).is_ok())
            .map(|(&target, _)| target).collect();
        affected.insert(section);
        affected.extend(TINT_NEIGHBORS.map(|(dx, dz)| pos::adjacent(section, dx, dz)));
        for affected in affected {
            // Never create bookkeeping for fully near/unrequested sections.
            if queue.mesh_revisions.contains_key(&affected) {
                queue.invalidate_mesh(affected);
            }
        }
    }

    /// Section `s` changed: its mesh and its neighbours' borders again, and
    /// its parent queued to take the change.
    fn changed<H: Host>(shared: &Shared<H>, s: i64) {
        if shared.wanted.lock().unwrap().contains(&s) {
            push(shared, Job::Mesh(s), 0);
        }
        remesh_neighbors(shared, s);
        if pos::detail_level(s) >= pos::ROOT_DETAIL_LEVEL {
            return;
        }
        let parent = pos::parent(s);
        let child = ((pos::x(s) & 1) | ((pos::z(s) & 1) << 1)) as u8;
        let queued = {
            let mut q = shared.queue.lock().unwrap();
            let queued = q.propagate.contains_key(&parent);
            *q.propagate.entry(parent).or_insert(0) |= 1 << child;
            queued
        };
        if !queued {
            // After the work in view, finer levels first: their changes add
            // up in the coarser ones.
            push(
                shared,
                Job::Propagate(parent),
                (1 << 20) | i64::from(pos::detail_level(parent)),
            );
        }
    }

    /// Section `s` takes the changes of its children named in `children`.
    fn propagate<H: Host>(shared: &Shared<H>, s: i64, children: u8) {
        let mut any = false;
        for i in 0..4 {
            if children & (1 << i) == 0 {
                continue;
            }
            let child = shared.store.lock().unwrap().sections.get(&pos::child(s, i)).cloned();
            if let Some(child) = child {
                any |= apply(shared, s, &child);
            }
        }
        if any {
            changed(shared, s);
        }
    }

    /// Loads section `s`, then fills what it lacks: from its parent, scaled
    /// up, and from the generator when there is one.
    fn prepare<H: Host>(shared: &Shared<H>, s: i64, mut generator: Option<&mut dyn ColumnSampler>) {
        let blocks = shared.host.blocks();
        let (current, parent) = {
            let mut store = shared.store.lock().unwrap();
            let current = section(shared, &mut store, s);
            let parent = (current.missing_columns() > 0 && pos::detail_level(s) < pos::ROOT_DETAIL_LEVEL)
                .then(|| store.sections.get(&pos::parent(s)).cloned())
                .flatten();
            (current, parent)
        };
        if (current.missing_columns()>0 || current.gen_steps.iter().any(|&step| step < gen_step::FEATURES))
            && let Some(generator) = generator.as_mut()
        {
            generator.request_section(s);
        }
        if current.missing_columns() == 0 {
            remesh_neighbors(shared, s);
            return;
        }
        let input = if let Some(generator) = generator {
            let (min_y, max_y) = shared.host.build_height();
            let mut generated = FullDataSource::empty(s);
            if generate_missing_where(&mut generated, generator, min_y, max_y, |i|current.column_is_missing(i)) > 0 {
                shared.generated.fetch_add(1, Ordering::Relaxed);
                Some(generated)
            } else {
                None
            }
        } else {
            parent.map(|p|(*p).clone())
        };
        let Some(input)=input else { return; };
        let mut updated = (*current).clone();
        if updated.update_from(&input, blocks) {
            let mut store = shared.store.lock().unwrap();
            if store.sections.get(&s).is_some_and(|n| Arc::ptr_eq(n, &current)) {
                store.sections.insert(s, Arc::new(updated));
                store.dirty.insert(s);
                invalidate_data_appearance(shared, s);
                drop(store);
            } else {
                // A chunk changed it meanwhile: merged into the newest.
                drop(store);
                apply(shared, s, &input);
            }
            remesh_neighbors(shared, s);
        }
    }

    const TINT_NEIGHBORS: [(i32, i32); 8] = [
        (0, -1), (0, 1), (-1, 0), (1, 0), (-1, -1), (1, -1), (-1, 1), (1, 1),
    ];

    /// Only selected coarse tiles can constrain a finer edge. The small margin
    /// covers successive coarser edge interpolation endpoints (their geometric
    /// series is less than twice the largest selected cell). No world sampling.
    fn coarser_tint_sources(s: i64, wanted: &HashSet<i64>) -> Vec<i64> {
        let margin = wanted.iter().map(|&p| pos::data_width(p)).max().unwrap_or(1) * 2;
        let (x, z) = (pos::min_corner_block_x(s), pos::min_corner_block_z(s));
        let width = pos::block_width(s);
        let mut sources: Vec<_> = wanted.iter().copied().filter(|&p| {
            if pos::detail_level(p) <= pos::detail_level(s) { return false; }
            let (px, pz) = (pos::min_corner_block_x(p), pos::min_corner_block_z(p));
            let pw = pos::block_width(p);
            px <= x + width + margin && px + pw >= x - margin
                && pz <= z + width + margin && pz + pw >= z - margin
        }).collect();
        sources.sort_unstable_by_key(|&p| (std::cmp::Reverse(pos::detail_level(p)), p));
        sources
    }

    fn tint_neighbors<H: Host>(shared: &Shared<H>, store: &mut Store, s: i64, dependencies: &mut Vec<i64>)
        -> [Option<Arc<FullDataSource>>; 8]
    {
        TINT_NEIGHBORS.map(|(dx, dz)| {
            let key = pos::adjacent(s, dx, dz);
            dependencies.push(key);
            cached_neighbor(shared, store, key)
        })
    }

    fn mesh<H: Host>(shared: &Shared<H>, s: i64) {
        let Some(revision) = shared.queue.lock().unwrap().mesh_revisions.get(&s).copied() else {
            // The section left the current plan before this job started.
            return;
        };
        let settings = *shared.settings.lock().unwrap();
        let blocks = shared.host.blocks();
        let coarse_keys = coarser_tint_sources(s, &shared.wanted.lock().unwrap());
        let (source, neighbors, coarse_data) = {
            let mut store = shared.store.lock().unwrap();
            let mut dependencies = vec![s];
            let source = section(shared, &mut store, s);
            let neighbors = tint_neighbors(shared, &mut store, s, &mut dependencies);
            let mut coarse_data = Vec::new();
            for key in coarse_keys {
                dependencies.push(key);
                if let Some(data) = cached_neighbor(shared, &mut store, key) {
                    let halo = tint_neighbors(shared, &mut store, key, &mut dependencies);
                    coarse_data.push((data, halo));
                }
            }
            dependencies.sort_unstable();
            dependencies.dedup();
            if !shared.queue.lock().unwrap().register_mesh_dependencies(s, revision, dependencies) {
                // The newer request/data change owns the replacement. Queue
                // bookkeeping is completed by work(); requeueing here would
                // invalidate that replacement again and could starve it.
                return;
            }
            (source, neighbors, coarse_data)
        };
        let near = shared.near.lock().unwrap().clone();
        let render = RenderSource::from_full(&source, blocks, &settings.render);
        let render_neighbors: [Option<RenderSource>; 4] = std::array::from_fn(|i| {
            neighbors[i].as_ref().map(|n| RenderSource::from_full(n, blocks, &settings.render))
        });
        let width = pos::data_width(s);
        let (bx, bz) = (pos::min_corner_block_x(s), pos::min_corner_block_z(s));
        let cutout = |x: i32, z: i32| {
            width <= 16 && near.covers((bx + x * width + width / 2) >> 4, (bz + z * width + width / 2) >> 4)
        };
        let mut quads = build_quads(
            &render,
            render_neighbors.each_ref().map(Option::as_ref),
            &settings.render,
            cutout,
        );
        append_stem_quads(&source, blocks, settings.render.min_y, cutout, &mut quads);
        let coarser: Vec<_> = coarse_data.iter().map(|(source, neighbors)| TintNeighborhood {
            source, neighbors: neighbors.each_ref().map(|n| n.as_deref()),
        }).collect();
        let mesh = (!quads.is_empty()).then(|| shared.host.mesh_with_neighbors(
            s, &source, &quads, settings.debug, MeshNeighborhood {
                neighbors: neighbors.each_ref().map(|n| n.as_deref()), coarser: &coarser,
            },
        ));
        let quad_count = mesh.as_ref().map_or(0, |mesh| shared.host.mesh_quad_count(mesh, quads.len()));
        shared.done.lock().unwrap().push(Meshed {
            section: s,
            mesh,
            cutout: near.section_mask(s),
            revision,
            quad_count,
        });
    }

    fn trim_store<H: Host>(shared: &Shared<H>) {
        let wanted = shared.wanted.lock().unwrap().clone();
        let garbage: Vec<Arc<FullDataSource>> = {
            let mut store = shared.store.lock().unwrap();
            if store.sections.len() <= wanted.len() * 3 + 256 {
                return;
            }
            let removed: Vec<_> = store
                .sections
                .keys()
                .copied()
                .filter(|s| !store.dirty.contains(s) && !wanted.contains(s))
                .collect();
            removed.into_iter().filter_map(|s| store.sections.remove(&s)).collect()
        };
        // No store/queue lock is held during destruction. The frame keeps rendering.
        drop(garbage);
    }

    fn save_dirty<H: Host>(shared: &Shared<H>) {
        let Some(dir) = &shared.dir else {
            shared.store.lock().unwrap().dirty.clear();
            return;
        };
        let pending: Vec<Arc<FullDataSource>> = {
            let mut store = shared.store.lock().unwrap();
            let dirty: Vec<i64> = store.dirty.drain().collect();
            dirty.iter().filter_map(|s| store.sections.get(s).cloned()).collect()
        };
        for source in pending {
            let _ = storage::save(dir, &source, shared.fingerprint);
        }
    }

    #[cfg(test)]
    mod water_tint_tests {
        use super::*;

        struct TintHost {
            blocks: BlockTable,
            held: Arc<AtomicBool>,
            release: Arc<AtomicBool>,
            target: i64,
        }
        impl Host for TintHost {
            type Mesh = u16;
            fn blocks(&self) -> &BlockTable { &self.blocks }
            fn build_height(&self) -> (i32, i32) { (-64, 320) }
            fn sampler(&self) -> Option<Box<dyn ColumnSampler>> { None }
            fn mesh(&self, _: i64, _: &FullDataSource, _: &[Quad], _: bool) -> u16 { 0 }
            fn mesh_with_neighbors(&self, s: i64, _: &FullDataSource, _: &[Quad], _: bool, neighborhood: MeshNeighborhood<'_>) -> u16 {
                if s != self.target { return 0; }
                let biome = neighborhood.coarser.first().and_then(|n| n.source.column(0, 0).first()
                    .map(|&p| n.source.mapping.get(full_point::id(p)).0)).unwrap_or(0);
                if biome == 13 && !self.release.load(Ordering::SeqCst) {
                    self.held.store(true, Ordering::SeqCst);
                    while !self.release.load(Ordering::SeqCst) { std::thread::sleep(Duration::from_millis(1)); }
                }
                biome
            }
        }
        fn host(target: i64) -> TintHost {
            TintHost {
                blocks: BlockTable { states: vec![
                    BlockInfo { air: true, material: material::AIR, ..BlockInfo::default() },
                    BlockInfo { solid: true, opacity: 16, alpha: 255, material: material::STONE, ..BlockInfo::default() },
                ] },
                held: Arc::new(AtomicBool::new(false)), release: Arc::new(AtomicBool::new(false)), target,
            }
        }
        fn settings() -> WorkSettings {
            WorkSettings { render: RenderSettings { cave_culling: false, cave_culling_height: 60, min_y: -64 },
                generation: false, debug: false }
        }
        fn terrain(s: i64, biome: u16) -> FullDataSource {
            let mut source = FullDataSource::empty(s);
            let column = column_from_runs(&mut source.mapping,
                &[SampledRun { min: -64, max: 63, state: 1, biome, sky: 15, block: 0 }], -64, 320);
            for x in 0..64 { for z in 0..64 { source.set_column(x, z, column.clone(), gen_step::LIGHT); } }
            source
        }

        #[test]
        fn changed_coarse_tint_invalidates_held_fine_mesh_and_publishes_replacement() {
            let fine = pos::encode(8, -6, -14);
            let coarse = pos::encode(9, -4, -7);
            let h = host(fine);
            let (held, release) = (h.held.clone(), h.release.clone());
            let mut engine = Engine::new(h, None, 1, settings(), 1);
            struct Release(Arc<AtomicBool>);
            impl Drop for Release { fn drop(&mut self) { self.0.store(true, Ordering::SeqCst); } }
            let _release_on_drop = Release(release.clone());
            engine.wanted = HashSet::from([fine, coarse]);
            *engine.shared.wanted.lock().unwrap() = engine.wanted.clone();
            {
                let mut store = engine.shared.store.lock().unwrap();
                store.sections.insert(fine, Arc::new(terrain(fine, 0)));
                store.sections.insert(coarse, Arc::new(terrain(coarse, 13)));
            }
            engine.shared.queue.lock().unwrap().invalidate_mesh(fine);
            let plan = DetailPlan::new(HorizontalQuality::Medium, 128);
            let player = [-1408., -3456.];
            engine.plan = Some(plan);
            engine.planned_at = Some((Instant::now(), player));
            let old_shared = engine.shared.clone();
            let old_mesh = std::thread::spawn(move || mesh(&old_shared, fine));
            let started = Instant::now();
            while !held.load(Ordering::SeqCst) {
                assert!(started.elapsed() < Duration::from_secs(5), "fine mesh did not read its coarse tint source");
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(apply(&engine.shared, coarse, &terrain(coarse, 60)));
            changed(&engine.shared, coarse);
            let mut replacement = false;
            let started = Instant::now();
            while !replacement || engine.stats().queued != 0 {
                for mesh in engine.update(player, plan, Near::default()).0 {
                    replacement |= mesh.section == fine && mesh.mesh == Some(60);
                    assert!(!(mesh.section == fine && mesh.mesh == Some(13)));
                }
                assert!(started.elapsed() < Duration::from_secs(10), "changed coarse data did not publish a fine replacement");
                std::thread::sleep(Duration::from_millis(1));
            }
            release.store(true, Ordering::SeqCst);
            old_mesh.join().unwrap();
            assert!(!engine.update(player, plan, Near::default()).0.iter()
                .any(|mesh| mesh.section == fine && mesh.mesh == Some(13)), "stale old tint overwrote the replacement");
            assert!(!engine.shared.queue.lock().unwrap().mesh_dependencies.is_empty());
            engine.wipe();
            assert!(engine.shared.queue.lock().unwrap().mesh_dependencies.is_empty(), "world wipe retained tint dependencies");
            let other_world = Engine::new(host(fine), None, 2, settings(), 1);
            assert!(other_world.shared.queue.lock().unwrap().mesh_dependencies.is_empty(), "another world inherited tint state");
        }

        #[test]
        fn moving_detail_plan_remeshes_only_surviving_tiles_whose_coarse_context_changed() {
            let plan = DetailPlan::new(HorizontalQuality::Medium, 128).with_near_radius(32);
            let before: HashSet<_> = plan.sections([0., 0.], |_| false).into_iter().collect();
            let after: HashSet<_> = plan.sections([128., 0.], |_| false).into_iter().collect();
            let changed: Vec<_> = before.intersection(&after).copied()
                .filter(|&s| coarser_tint_sources(s, &before) != coarser_tint_sources(s, &after)).collect();
            let unchanged: Vec<_> = before.intersection(&after).copied()
                .filter(|&s| coarser_tint_sources(s, &before) == coarser_tint_sources(s, &after)).collect();
            assert!(!changed.is_empty() && !unchanged.is_empty(), "fixture must exercise both outcomes");
            let mut engine = Engine::new(host(0), None, 1, settings(), 1);
            // Inspect actual replan queueing without racing a background mesh.
            engine.shared.stop.store(true, Ordering::Relaxed);
            engine.shared.ready.notify_all();
            for worker in std::mem::take(&mut engine.threads) { worker.join().unwrap(); }
            engine.wanted = before.clone();
            engine.shown = before.clone();
            engine.published_cutouts = before.iter().map(|&s| (s, None)).collect();
            engine.plan = Some(plan);
            *engine.shared.wanted.lock().unwrap() = before;
            engine.replan([128., 0.], plan, &Near::default());
            let queue = engine.shared.queue.lock().unwrap();
            assert!(changed.iter().all(|s| queue.meshes.contains(s)), "surviving mixed edges were not remeshed");
            assert!(unchanged.iter().all(|s| !queue.meshes.contains(s)), "unchanged ocean tiles were needlessly rebuilt");
        }

        #[test]
        fn delayed_old_snapshot_cannot_replace_current_tint_dependencies_or_lose_queued_job() {
            let fine = pos::encode(8, -8, -14);
            let old_coarse = pos::encode(9, -5, -7);
            let new_coarse = pos::encode(10, -3, -4);
            let old_plan = HashSet::from([fine, old_coarse]);
            let new_plan = HashSet::from([fine, new_coarse]);
            let mut engine = Engine::new(host(fine), None, 1, settings(), 1);
            engine.shared.stop.store(true, Ordering::Relaxed);
            engine.shared.ready.notify_all();
            for worker in std::mem::take(&mut engine.threads) { worker.join().unwrap(); }
            engine.wanted = new_plan.clone();
            *engine.shared.wanted.lock().unwrap() = new_plan.clone();
            let old_revision = {
                let mut queue = engine.shared.queue.lock().unwrap();
                queue.invalidate_mesh(fine);
                queue.mesh_revisions[&fine]
            };
            let mut delayed_dependencies = coarser_tint_sources(fine, &old_plan);
            assert_eq!(delayed_dependencies, [old_coarse]);
            delayed_dependencies.push(fine);
            delayed_dependencies.sort_unstable();
            // Old job captured revision/coarse keys and is delayed before its
            // store snapshot. Replan queues a newer job with another halo.
            push(&engine.shared, Job::Mesh(fine), 0);
            let mut current_dependencies = coarser_tint_sources(fine, &new_plan);
            assert_eq!(current_dependencies, [new_coarse]);
            current_dependencies.push(fine);
            current_dependencies.sort_unstable();
            {
                let _store = engine.shared.store.lock().unwrap();
                let mut queue = engine.shared.queue.lock().unwrap();
                let current_revision = queue.mesh_revisions[&fine];
                assert_ne!(current_revision, old_revision);
                assert!(queue.register_mesh_dependencies(fine, current_revision, current_dependencies.clone()));
                let queued = queue.heap.len();
                // Resume old registration after the current one, in the exact
                // adverse order; rejection must not mutate current work.
                assert!(!queue.register_mesh_dependencies(fine, old_revision, delayed_dependencies));
                assert_eq!(queue.mesh_dependencies[&fine], current_dependencies);
                assert_eq!(queue.mesh_revisions[&fine], current_revision);
                assert_eq!(queue.heap.len(), queued);
                assert!(queue.meshes.contains(&fine));
                drop(queue);
                invalidate_data_appearance(&engine.shared, new_coarse);
                assert_ne!(engine.shared.queue.lock().unwrap().mesh_revisions[&fine], current_revision,
                    "the retained current dependency must invalidate the fine mesh when it changes");
            }
            remesh_neighbors(&engine.shared, new_coarse);
            let queue = engine.shared.queue.lock().unwrap();
            assert!(queue.meshes.contains(&fine));
            assert_eq!(queue.heap.iter().filter(|job| matches!(&job.job, Job::Mesh(s) if *s == fine)).count(), 1,
                "old registration must neither strand nor duplicate the replacement");
        }
    }

    #[cfg(test)]
    mod publication_tests {
        use super::*;

        #[test]
        fn partial_preparation_keeps_visited_columns_while_filling_unknowns() {
            struct TestHost(BlockTable);
            impl Host for TestHost {
                type Mesh = ();
                fn blocks(&self) -> &BlockTable { &self.0 }
                fn build_height(&self) -> (i32,i32) { (-64,320) }
                fn sampler(&self) -> Option<Box<dyn ColumnSampler>> { None }
                fn mesh(&self, _:i64, _: &FullDataSource, _: &[Quad], _:bool) {}
            }
            struct Fill;
            impl ColumnSampler for Fill {
                fn column(&mut self, _:i32, _:i32, out:&mut Vec<SampledRun>) {
                    out.push(SampledRun { min:-64,max:80,state:1,biome:0,sky:15,block:0 });
                }
            }
            let blocks=BlockTable { states:vec![
                BlockInfo { air:true,material:material::AIR,..BlockInfo::default() },
                BlockInfo { solid:true,opacity:16,alpha:255,material:material::STONE,..BlockInfo::default() },
            ] };
            let settings=WorkSettings { render:RenderSettings { cave_culling:false,cave_culling_height:60,min_y:-64 },generation:true,debug:false };
            let e=Engine::new(TestHost(blocks),None,1,settings,1);
            let s=pos::encode(6,0,0);
            let mut initial=FullDataSource::empty(s);
            let visited=column_from_runs(&mut initial.mapping,&[SampledRun { min:-64,max:63,state:1,biome:0,sky:15,block:0 }],-64,320);
            initial.set_column(0,0,visited.clone(),gen_step::LIGHT);
            let known_air=column_from_runs(&mut initial.mapping,&[],-64,320);
            initial.set_column(0,1,known_air.clone(),gen_step::LIGHT);
            e.shared.store.lock().unwrap().sections.insert(s,Arc::new(initial));
            prepare(&e.shared,s,Some(&mut Fill));
            let prepared=e.shared.store.lock().unwrap().sections[&s].clone();
            assert_eq!(prepared.column(0,0),visited,"preparing unknown terrain erased a visited LIGHT column");
            assert_eq!(prepared.column(0,1),known_air,"known air must remain an explicit column");
            assert_eq!(prepared.gen_steps[index(0,0)],gen_step::LIGHT);
            assert_eq!(prepared.missing_columns(),0);
            assert!(prepared.columns.iter().all(|c|!c.is_empty()),"complete metadata concealed unknown empty vectors");
        }

        #[test]
        fn prepare_replacement_invalidates_a_mesh_started_under_the_same_request() {
            struct TestHost {
                blocks: BlockTable,
                held: Arc<AtomicBool>,
                release: Arc<AtomicBool>,
            }
            impl Host for TestHost {
                type Mesh = bool;
                fn blocks(&self) -> &BlockTable { &self.blocks }
                fn build_height(&self) -> (i32,i32) { (-64,320) }
                fn sampler(&self) -> Option<Box<dyn ColumnSampler>> { None }
                fn mesh(&self, _:i64, source:&FullDataSource, _: &[Quad], _:bool) -> bool {
                    let complete = source.missing_columns() == 0;
                    if !complete {
                        self.held.store(true,Ordering::SeqCst);
                        while !self.release.load(Ordering::SeqCst) {
                            std::thread::sleep(Duration::from_millis(1));
                        }
                    }
                    complete
                }
            }
            struct Fill;
            impl ColumnSampler for Fill {
                fn column(&mut self, _:i32, _:i32, out:&mut Vec<SampledRun>) {
                    out.push(SampledRun { min:-64,max:80,state:1,biome:0,sky:15,block:0 });
                }
            }
            struct ReleaseOnDrop(Arc<AtomicBool>);
            impl Drop for ReleaseOnDrop {
                fn drop(&mut self) { self.0.store(true,Ordering::SeqCst); }
            }
            let held=Arc::new(AtomicBool::new(false));
            let release=Arc::new(AtomicBool::new(false));
            let blocks=BlockTable { states:vec![
                BlockInfo { air:true,material:material::AIR,..BlockInfo::default() },
                BlockInfo { solid:true,opacity:16,alpha:255,material:material::STONE,..BlockInfo::default() },
            ] };
            let settings=WorkSettings { render:RenderSettings { cave_culling:false,cave_culling_height:60,min_y:-64 },generation:true,debug:false };
            let mut e=Engine::new(TestHost { blocks,held:held.clone(),release:release.clone() },None,1,settings,1);
            let _release_on_drop=ReleaseOnDrop(release.clone());
            let section=pos::encode(6,0,0);
            let mut initial=FullDataSource::empty(section);
            let column=column_from_runs(&mut initial.mapping,&[SampledRun { min:-64,max:63,state:1,biome:0,sky:15,block:0 }],-64,320);
            initial.set_column(0,0,column,gen_step::SURFACE);
            e.shared.store.lock().unwrap().sections.insert(section,Arc::new(initial));
            // An already queued Mesh can start after Section's request stamp
            // changes, but before that Section actually writes prepared data.
            e.shared.queue.lock().unwrap().invalidate_mesh(section);
            e.wanted.insert(section);
            e.shared.wanted.lock().unwrap().insert(section);
            let plan=DetailPlan::new(HorizontalQuality::Low,8);
            e.plan=Some(plan);
            e.planned_at=Some((Instant::now(),[8.,8.]));
            let old_shared=e.shared.clone();
            let old=std::thread::spawn(move || mesh(&old_shared,section));
            let started=Instant::now();
            while !held.load(Ordering::SeqCst) {
                assert!(started.elapsed().as_secs()<5,"old partial mesh did not reach the host");
                std::thread::sleep(Duration::from_millis(1));
            }
            prepare(&e.shared,section,Some(&mut Fill));
            mesh(&e.shared,section);
            let (new,_) = e.update([8.,8.],plan,Near::default());
            assert!(new.iter().any(|m|m.section==section && m.mesh==Some(true)),"prepared replacement did not publish");
            release.store(true,Ordering::SeqCst);
            old.join().unwrap();
            let (late,_) = e.update([8.,8.],plan,Near::default());
            assert!(!late.iter().any(|m|m.section==section && m.mesh==Some(false)),"pre-prepare mesh overwrote the prepared source under one request stamp");
            assert_eq!(e.stats().queued,0,"stale rejection invented an unneeded remesh");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AIR: u32 = 0;
    const STONE: u32 = 1;
    const GRASS: u32 = 2;
    const WATER: u32 = 3;
    const FLOWER: u32 = 4;
    const SNOW: u32 = 5;

    fn blocks() -> BlockTable {
        let solid = BlockInfo {
            opacity: 16,
            solid: true,
            material: material::STONE,
            alpha: 255,
            ..BlockInfo::default()
        };
        BlockTable {
            states: vec![
                BlockInfo {
                    air: true,
                    material: material::AIR,
                    ..BlockInfo::default()
                },
                solid,
                BlockInfo {
                    material: material::GRASS,
                    ..solid
                },
                BlockInfo {
                    opacity: 1,
                    liquid: true,
                    material: material::WATER,
                    alpha: 0xA0,
                    ..BlockInfo::default()
                },
                BlockInfo {
                    alpha: 255,
                    material: material::LEAVES,
                    ..BlockInfo::default()
                },
                BlockInfo {
                    alpha: 255,
                    material: material::SNOW,
                    snow_layer: true,
                    ..BlockInfo::default()
                },
            ],
        }
    }

    /// A chunk of stone up to `height(x, z)` (exclusive), grass on top.
    struct FlatChunk {
        x: i32,
        z: i32,
        height: fn(i32, i32) -> i32,
    }
    impl ChunkView for FlatChunk {
        fn chunk_x(&self) -> i32 {
            self.x
        }
        fn chunk_z(&self) -> i32 {
            self.z
        }
        fn min_build_height(&self) -> i32 {
            -64
        }
        fn max_build_height(&self) -> i32 {
            320
        }
        fn min_non_empty_height(&self) -> i32 {
            -64
        }
        fn top(&self, x: i32, z: i32) -> i32 {
            (self.height)(self.x * 16 + x, self.z * 16 + z)
        }
        fn state(&self, x: i32, y: i32, z: i32) -> u32 {
            let h = (self.height)(self.x * 16 + x, self.z * 16 + z);
            if y >= h {
                AIR
            } else if y == h - 1 {
                GRASS
            } else {
                STONE
            }
        }
        fn biome(&self, _: i32, _: i32, _: i32) -> u16 {
            1
        }
        fn light(&self, x: i32, y: i32, z: i32) -> (u8, u8) {
            if y >= self.top(x, z) { (15, 0) } else { (0, 0) }
        }
    }

    #[test]
    fn section_positions_pack_as_distant_horizons_does() {
        let p = pos::encode(10, -3, 7);
        assert_eq!((pos::detail_level(p), pos::x(p), pos::z(p)), (10, -3, 7));
        // DhSectionPos.encode(10, -3, 7), computed with the Java code.
        assert_eq!(p, ((7i64) << 36) | ((-3i64 & ((1 << 28) - 1)) << 8) | 10);
        assert_eq!(pos::min_corner_block_x(p), -3 * 1024);
        assert_eq!(pos::center_block_z(p), 7 * 1024 + 512);
        assert_eq!(pos::parent(pos::child(p, 3)), p);
        assert_eq!(pos::containing_block(6, -1, 64), pos::encode(6, -1, 1));
        assert_eq!(pos::containing_block(8, -1, 64), pos::encode(8, -1, 0));
        assert!(pos::contains(p, pos::child(pos::child(p, 2), 1)));
        assert_eq!(pos::to_string(p), "10*-3,7");
    }

    #[test]
    fn full_data_points_keep_their_fields() {
        let p = full_point::encode(1234, 70, 64, 9, 15).unwrap();
        assert_eq!(full_point::id(p), 1234);
        assert_eq!(full_point::height(p), 70);
        assert_eq!(full_point::bottom_y(p), 64);
        assert_eq!(full_point::block_light(p), 9);
        assert_eq!(full_point::sky_light(p), 15);
        assert_eq!(full_point::top_y(full_point::set_height(p, 3)), 67);
        assert!(full_point::encode(1, 0, 0, 0, 0).is_none());
        assert!(full_point::encode(1, 1, 0, 16, 0).is_none());
    }

    #[test]
    fn a_chunk_becomes_runs_from_the_top_down() {
        let chunk = FlatChunk {
            x: 5,
            z: -1,
            height: |_, _| 64,
        };
        let source = create_from_chunk(&chunk, &blocks());
        assert_eq!(source.pos, pos::encode(6, 1, -1));
        // Chunk 5 is the second of its section, chunk -1 the last.
        let column = source.column(16, 48);
        let runs: Vec<_> = column
            .iter()
            .map(|&p| {
                (
                    source.mapping.state(full_point::id(p)),
                    full_point::bottom_y(p),
                    full_point::height(p),
                )
            })
            .collect();
        // Air down to 64, grass at 63, stone to the bottom (relative to -64).
        // The air run starts at the exclusive build limit, one above the
        // world, as `createFromChunk`'s `lastY` does.
        assert_eq!(runs, vec![(AIR, 128, 257), (GRASS, 127, 1), (STONE, 0, 127)]);
        assert_eq!(full_point::sky_light(column[1]), 15);
        assert_eq!(source.gen_steps[index(16, 48)], gen_step::LIGHT);
        assert_eq!(source.gen_steps[index(0, 0)], gen_step::EMPTY);
    }

    #[test]
    fn two_by_two_columns_vote_for_their_block() {
        let b = blocks();
        let mut child = FullDataSource::empty(pos::encode(6, 0, 0));
        let id = |m: &mut IdMap, s| m.add_if_not_present(1, s);
        for (i, height) in [10, 10, 12, 10].into_iter().enumerate() {
            let (x, z) = ((i & 1) as i32, (i >> 1) as i32);
            let air = id(&mut child.mapping, AIR);
            let stone = id(&mut child.mapping, STONE);
            let column = vec![
                full_point::encode(air, 384 - height, height, 0, 15).unwrap(),
                full_point::encode(stone, height, 0, 0, 0).unwrap(),
            ];
            child.set_column(x, z, column, gen_step::LIGHT);
        }
        let mut parent = FullDataSource::empty(pos::encode(7, 0, 0));
        assert!(parent.update_from(&child, &b));
        let merged = parent.column(0, 0);
        let runs: Vec<_> = merged
            .iter()
            .map(|&p| {
                (
                    parent.mapping.state(full_point::id(p)),
                    full_point::bottom_y(p),
                    full_point::top_y(p),
                )
            })
            .collect();
        // One taller column must not raise the whole parent footprint.
        assert_eq!(runs, vec![(AIR, 10, 384), (STONE, 0, 10)]);
        assert_eq!(parent.gen_steps[index(0, 0)], gen_step::LIGHT);
        // The other columns of the child were empty: left as they were.
        assert_eq!(parent.gen_steps[index(1, 0)], gen_step::EMPTY);
    }

    #[test]
    fn coarse_shores_and_canopies_require_half_the_child_footprint() {
        let mut b = blocks();
        let leaves = b.states.len() as u32;
        b.states.push(BlockInfo {
            material: material::LEAVES,
            ..b.states[GRASS as usize]
        });
        for cap in [GRASS, leaves] {
            for occupied in 1..=4 {
                let mut child = FullDataSource::empty(pos::encode(6, 0, 0));
                for i in 0..4 {
                    // A narrow shore protrusion or canopy occupies only part
                    // of this 2x2 footprint. All columns retain real water
                    // beneath it; air above is known data, not a missing tile.
                    let biome = i as u16 + 1;
                    let air = child.mapping.add_if_not_present(biome, AIR);
                    let top = child.mapping.add_if_not_present(biome, cap);
                    let water = child.mapping.add_if_not_present(1, WATER);
                    let stone = child.mapping.add_if_not_present(1, STONE);
                    let high = if i < occupied { 132 } else { 127 };
                    let mut runs = vec![full_point::encode(air, 384 - high, high, 0, 15).unwrap()];
                    if i < occupied {
                        runs.push(full_point::encode(top, 5, 127, 0, 15).unwrap());
                    }
                    runs.extend([
                        full_point::encode(water, 3, 124, 0, 15).unwrap(),
                        full_point::encode(stone, 124, 0, 0, 0).unwrap(),
                    ]);
                    child.set_column(i & 1, i >> 1, runs, gen_step::LIGHT);
                }
                let mut parent = FullDataSource::empty(pos::encode(7, 0, 0));
                assert!(parent.update_from(&child, &b));
                let state_at = |y| {
                    let p = parent.column(0, 0).iter().find(|&&p| {
                        full_point::bottom_y(p) <= y && y < full_point::top_y(p)
                    }).expect("known air, water and solid ranges remain represented");
                    parent.mapping.state(full_point::id(*p))
                };
                assert_eq!(state_at(125), WATER, "continuous water is retained");
                assert_eq!(
                    state_at(130),
                    if occupied >= 2 { cap } else { AIR },
                    "{occupied}/4 occupied child columns must follow native preview occupancy"
                );
                assert_eq!(state_at(140), AIR);
                assert_eq!(parent.gen_steps[index(0, 0)], gen_step::LIGHT);
                assert_eq!(parent.gen_steps[index(1, 0)], gen_step::EMPTY);
            }
        }
    }

    #[test]
    fn less_complete_data_never_replaces_more_complete() {
        let b = blocks();
        let chunk = FlatChunk {
            x: 0,
            z: 0,
            height: |_, _| 64,
        };
        let mut section = create_from_chunk(&chunk, &b);
        let before = section.clone();
        let mut generated = FullDataSource::empty(section.pos);
        let stone = generated.mapping.add_if_not_present(1, STONE);
        for x in 0..pos::WIDTH {
            for z in 0..pos::WIDTH {
                generated.set_column(
                    x,
                    z,
                    vec![full_point::encode(stone, 10, 0, 0, 15).unwrap()],
                    gen_step::SURFACE,
                );
            }
        }
        assert!(section.update_from(&generated, &b));
        assert_eq!(section.column(3, 3).len(), before.column(3, 3).len());
        assert_eq!(section.gen_steps[index(3, 3)], gen_step::LIGHT);
        assert_eq!(section.gen_steps[index(40, 40)], gen_step::SURFACE);
        assert_eq!(section.missing_columns(), 0);
    }

    #[test]
    fn buried_runs_are_culled_into_the_run_above() {
        let b = blocks();
        let mut source = FullDataSource::empty(pos::encode(6, 0, 0));
        let mut input = FullDataSource::empty(source.pos);
        let air = input.mapping.add_if_not_present(1, AIR);
        let grass = input.mapping.add_if_not_present(1, GRASS);
        let stone = input.mapping.add_if_not_present(1, STONE);
        for x in 0..3 {
            for z in 0..3 {
                let column = vec![
                    full_point::encode(air, 300, 84, 0, 15).unwrap(),
                    full_point::encode(grass, 4, 80, 0, 15).unwrap(),
                    full_point::encode(stone, 80, 0, 0, 0).unwrap(),
                ];
                input.set_column(x, z, column, gen_step::LIGHT);
            }
        }
        source.update_from(&input, &b);
        // The middle column's grass is surrounded and covered above by air:
        // translucent, so it stays; the stone at the bottom is the last run.
        assert_eq!(source.column(1, 1).len(), 3);
        // Three opaque runs: the middle one is hidden on every side.
        let mut input = FullDataSource::empty(source.pos);
        let ids = [AIR, GRASS, STONE, GRASS].map(|s| input.mapping.add_if_not_present(1, s));
        for x in 0..3 {
            for z in 0..3 {
                let column = vec![
                    full_point::encode(ids[0], 300, 84, 0, 15).unwrap(),
                    full_point::encode(ids[1], 4, 80, 0, 15).unwrap(),
                    full_point::encode(ids[2], 40, 40, 0, 0).unwrap(),
                    full_point::encode(ids[3], 40, 0, 0, 0).unwrap(),
                ];
                input.set_column(x, z, column, gen_step::LIGHT);
            }
        }
        let mut source = FullDataSource::empty(input.pos);
        source.update_from(&input, &b);
        let middle = source.column(1, 1);
        assert_eq!(middle.len(), 3);
        assert_eq!(full_point::bottom_y(middle[1]), 40);
        assert_eq!(full_point::height(middle[1]), 44);
        assert_eq!(source.column(0, 0).len(), 4);
    }

    #[test]
    fn the_detail_plan_drops_detail_with_distance() {
        let plan = DetailPlan::new(HorizontalQuality::Medium, 256);
        let player = [8.0, 8.0];
        let near = pos::containing_block(6, 100, 0);
        let far = pos::containing_block(6, 3000, 0);
        // Medium: 192-block units, base 2.
        assert_eq!(plan.expected_detail_level(player, near), 6);
        assert_eq!(plan.expected_detail_level(player, far), 6 + 3);
        let sections = plan.sections(player, |_| false);
        assert!(!sections.is_empty());
        // The sections tile the area without overlapping.
        for (i, &a) in sections.iter().enumerate() {
            for &b in &sections[i + 1..] {
                assert!(
                    !pos::contains(a, b) && !pos::contains(b, a),
                    "{} {}",
                    pos::to_string(a),
                    pos::to_string(b)
                );
            }
        }
        let area: i64 = sections.iter().map(|&s| i64::from(pos::block_width(s)).pow(2)).sum();
        assert!(area >= i64::from(plan.radius * 2).pow(2), "{area}");
        // Nearest sections are the finest.
        let nearest = sections
            .iter()
            .min_by_key(|&&s| pos::chebyshev_signed_block_distance(s, 8, 8))
            .unwrap();
        assert_eq!(pos::detail_level(*nearest), 6);
        assert!(sections.iter().any(|&s| pos::detail_level(s) >= 9));
    }

    #[test]
    fn boundary_detail_stays_block_scale_without_widening_the_whole_view() {
        let player = [8., 8.];
        let base = DetailPlan::new(HorizontalQuality::Medium, 128);
        let improved = base.with_near_radius(32);
        let old = base.sections(player, |_| false);
        let new = improved.sections(player, |_| false);
        let at = |sections: &[i64], x, z| {
            let point = pos::containing_block(pos::SECTION_MINIMUM_DETAIL_LEVEL, x, z);
            *sections.iter().find(|&&s| pos::contains(s, point)).unwrap()
        };
        assert_eq!(
            pos::data_width(at(&new, 550, 8)),
            1,
            "near32 must meet single-block far terrain"
        );
        assert!(
            pos::data_width(at(&old, 550, 8)) > 1,
            "fixture must expose the old resolution jump"
        );
        for (x, z) in [(1200, 8), (-1400, 8), (8, 1600), (8, -1900)] {
            assert_eq!(
                at(&old, x, z),
                at(&new, x, z),
                "outside the boundary ring retain normal detail"
            );
        }
        assert!(
            new.len() <= old.len() * 2,
            "transition detail must have a bounded section budget"
        );
        for radius in [0, 2, 8] {
            assert_eq!(
                base.with_near_radius(radius).sections(player, |_| false),
                old,
                "do not spend extra geometry when Medium already draws this boundary at block scale"
            );
        }
    }

    #[test]
    fn boundary_detail_respects_cheap_profiles_and_negative_world_positions() {
        for quality in [HorizontalQuality::Lowest, HorizontalQuality::Low] {
            let plan = DetailPlan::new(quality, 128);
            assert_eq!(
                plan.with_near_radius(32),
                plan,
                "cheap profiles must not add a detail ring"
            );
        }
        let plan = DetailPlan::new(HorizontalQuality::Medium, 128).with_near_radius(32);
        for player in [[8., 8.], [-104., -216.], [4104., -8184.]] {
            let sections = plan.sections(player, |_| false);
            for (dx, dz) in [(544., 0.), (-544., 0.), (0., 544.), (0., -544.)] {
                let point = pos::containing_block(6, (player[0] + dx) as i32, (player[1] + dz) as i32);
                let section = *sections.iter().find(|&&s| pos::contains(s, point)).unwrap();
                assert_eq!(
                    pos::data_width(section),
                    1,
                    "ring must follow the camera in every quadrant"
                );
            }
            for (index, &section) in sections.iter().enumerate() {
                assert!(
                    sections[index + 1..]
                        .iter()
                        .all(|&other| !pos::contains(section, other) && !pos::contains(other, section)),
                    "ring may not overlap its coarser neighbors"
                );
            }
        }
    }

    #[test]
    fn material_sampling_uses_world_coordinates_on_every_face() {
        let section = pos::encode(6, -2, 3);
        for (dir, expected) in [
            (Dir::Up, [-120., 242.]),
            (Dir::Down, [-120., 242.]),
            (Dir::North, [-120., 194.]),
            (Dir::South, [-120., 194.]),
            (Dir::West, [-126., 200.]),
            (Dir::East, [-126., 200.]),
        ] {
            let q = Quad {
                dir,
                plane: 2,
                transparent: false,
                id: 0,
                sky: 15,
                block: 0,
                u0: 4,
                v0: 40,
                u1: 12,
                v1: 60,
                lower_top: false,
            };
            assert_eq!(q.horizontal_center(section), expected);
        }
    }

    #[test]
    fn water_volume_ignores_submerged_plant_gaps_but_keeps_real_air() {
        let b = blocks();
        let mut mapping = IdMap::default();
        let [air, water, plant, stone] = [AIR, WATER, FLOWER, STONE].map(|s| mapping.add_if_not_present(1, s));
        let settings = RenderSettings {
            cave_culling: false,
            cave_culling_height: 60,
            min_y: -64,
        };
        let mut full = vec![
            full_point::encode(air, 100, 132, 0, 15).unwrap(),
            full_point::encode(water, 4, 128, 0, 15).unwrap(),
            full_point::encode(plant, 1, 127, 0, 10).unwrap(),
            full_point::encode(water, 15, 112, 0, 10).unwrap(),
            full_point::encode(stone, 112, 0, 0, 0).unwrap(),
        ];
        let rendered = render_column(&full, &mapping, &b, &settings);
        let fluids: Vec<_> = rendered
            .points
            .iter()
            .copied()
            .filter(|&p| render_point::is_fluid(p))
            .collect();
        assert_eq!(
            fluids.len(),
            1,
            "Underwater plants must not add interior water surfaces"
        );
        assert_eq!(
            (render_point::y_min(fluids[0]), render_point::y_max(fluids[0])),
            (112, 132)
        );
        full[2] = full_point::encode(air, 1, 127, 0, 10).unwrap();
        let rendered = render_column(&full, &mapping, &b, &settings);
        assert_eq!(
            rendered.points.iter().filter(|&&p| render_point::is_fluid(p)).count(),
            2,
            "An actual air pocket must remain open"
        );
    }

    #[test]
    fn water_volume_has_no_internal_walls_at_biome_boundaries() {
        let b = blocks();
        let mut full = FullDataSource::empty(pos::encode(6, 0, 0));
        let air = full.mapping.add_if_not_present(1, AIR);
        let water_a = full.mapping.add_if_not_present(1, WATER);
        let water_b = full.mapping.add_if_not_present(2, WATER);
        let stone = full.mapping.add_if_not_present(1, STONE);
        for x in 0..pos::WIDTH {
            for z in 0..pos::WIDTH {
                full.set_column(
                    x,
                    z,
                    vec![
                        full_point::encode(air, 100, 132, 0, 15).unwrap(),
                        full_point::encode(if x < 32 { water_a } else { water_b }, 20, 112, 0, 15).unwrap(),
                        full_point::encode(stone, 112, 0, 0, 0).unwrap(),
                    ],
                    gen_step::LIGHT,
                );
            }
        }
        let settings = RenderSettings {
            cave_culling: false,
            cave_culling_height: 60,
            min_y: -64,
        };
        let render = RenderSource::from_full(&full, &b, &settings);
        let quads = build_quads(&render, [None; 4], &settings, |_, _| false);
        assert!(
            !quads.iter().any(|q| (q.dir == Dir::West || q.dir == Dir::East)
                && q.plane == 32
                && [water_a, water_b].contains(&q.id)),
            "Tint changes must not create a translucent wall inside continuous water"
        );
        assert!(quads.iter().any(|q| q.dir == Dir::Up && q.id == water_a));
        assert!(quads.iter().any(|q| q.dir == Dir::Up && q.id == water_b));
        // A real edge against air still needs its water side face.
        full.set_column(
            32,
            0,
            vec![
                full_point::encode(air, 120, 112, 0, 15).unwrap(),
                full_point::encode(stone, 112, 0, 0, 0).unwrap(),
            ],
            gen_step::LIGHT,
        );
        let render = RenderSource::from_full(&full, &b, &settings);
        let quads = build_quads(&render, [None; 4], &settings, |_, _| false);
        assert!(
            quads
                .iter()
                .any(|q| q.dir == Dir::East && q.plane == 32 && q.id == water_a)
        );
    }

    #[test]
    fn render_columns_merge_and_carry_snow() {
        let b = blocks();
        let mut mapping = IdMap::default();
        let ids = [AIR, SNOW, GRASS, STONE, FLOWER, WATER].map(|s| mapping.add_if_not_present(1, s));
        let settings = RenderSettings {
            cave_culling: true,
            cave_culling_height: 60,
            min_y: -64,
        };
        let column = vec![
            full_point::encode(ids[0], 100, 129, 0, 15).unwrap(),
            full_point::encode(ids[1], 1, 128, 0, 15).unwrap(),
            full_point::encode(ids[2], 1, 127, 0, 15).unwrap(),
            full_point::encode(ids[3], 127, 0, 0, 0).unwrap(),
        ];
        let r = render_column(&column, &mapping, &b, &settings);
        // Snow is not drawn itself: the grass under it takes its id.
        assert_eq!(r.points.len(), 2);
        assert_eq!(render_point::id(r.points[0]), ids[1]);
        assert_eq!(render_point::y_max(r.points[0]), 128);
        assert_eq!(render_point::id(r.points[1]), ids[3]);
        // A flower is left out; water keeps its alpha.
        let column = vec![
            full_point::encode(ids[0], 100, 130, 0, 15).unwrap(),
            full_point::encode(ids[4], 1, 129, 0, 15).unwrap(),
            full_point::encode(ids[5], 2, 127, 0, 15).unwrap(),
            full_point::encode(ids[3], 127, 0, 0, 0).unwrap(),
        ];
        let r = render_column(&column, &mapping, &b, &settings);
        assert_eq!(r.points.len(), 2);
        assert_eq!(render_point::id(r.points[0]), ids[5]);
        assert!(render_point::alpha(r.points[0]) < 255);
        assert!(render_point::is_fluid(r.points[0]));
    }

    #[test]
    fn a_flat_section_meshes_to_one_top_face() {
        let b = blocks();
        let mut source = FullDataSource::empty(pos::encode(6, 0, 0));
        let mut mapping = IdMap::default();
        let air = mapping.add_if_not_present(1, AIR);
        let grass = mapping.add_if_not_present(1, GRASS);
        source.mapping = mapping;
        for x in 0..pos::WIDTH {
            for z in 0..pos::WIDTH {
                let column = vec![
                    full_point::encode(air, 200, 128, 0, 15).unwrap(),
                    full_point::encode(grass, 128, 0, 0, 15).unwrap(),
                ];
                source.set_column(x, z, column, gen_step::LIGHT);
            }
        }
        let settings = RenderSettings {
            cave_culling: true,
            cave_culling_height: 60,
            min_y: -64,
        };
        let render = RenderSource::from_full(&source, &b, &settings);
        let quads = build_quads(&render, [None; 4], &settings, |_, _| false);
        let up: Vec<_> = quads.iter().filter(|q| q.dir == Dir::Up).collect();
        assert_eq!(up.len(), 1);
        assert_eq!(
            (up[0].plane, up[0].u0, up[0].v0, up[0].u1, up[0].v1),
            (64, 0, 0, 64, 64)
        );
        // The bottom over the void; unknown neighbors do not imply cliffs.
        assert_eq!(quads.len(), 2, "{quads:?}");
        let walls = quads.iter().filter(|q| q.dir == Dir::North).count();
        assert_eq!(walls, 0);
        // Near-owned columns hide adjacent sides even though their top/bottom
        // are omitted here. A renderer cutout is not a terrain excavation.
        // Merged by rows, then rows into bands: the rows above and below the
        // hole, and the two halves of its own row.
        let quads = build_quads(&render, [None; 4], &settings, |x, z| x == 10 && z == 10);
        assert_eq!(quads.iter().filter(|q| q.dir == Dir::Up).count(), 4);
        assert_eq!(quads.iter().filter(|q| q.dir == Dir::Down).count(), 4);
        assert_eq!(quads.len(), 4 + 4);
    }

    fn lake_source(section: i64, floor: i32) -> FullDataSource {
        let mut source = FullDataSource::empty(section);
        let column = column_from_runs(
            &mut source.mapping,
            &[
                SampledRun { min: -64, max: floor, state: STONE, biome: 1, sky: 15, block: 0 },
                SampledRun { min: floor, max: 63, state: WATER, biome: 1, sky: 15, block: 0 },
            ],
            -64,
            320,
        );
        for x in 0..pos::WIDTH {
            for z in 0..pos::WIDTH {
                source.set_column(x, z, column.clone(), gen_step::LIGHT);
            }
        }
        source
    }

    #[test]
    fn cold_water_boundaries_do_not_invent_walls_but_known_underwater_cliffs_remain() {
        let blocks = blocks();
        let settings = RenderSettings { cave_culling: false, cave_culling_height: 60, min_y: -64 };
        let source = lake_source(pos::encode(6, 0, 0), 60);
        let render = RenderSource::from_full(&source, &blocks, &settings);
        let empty = RenderSource::from_full(&FullDataSource::empty(pos::encode(6, 1, 0)), &blocks, &settings);
        for neighbors in [[None; 4], [None, None, None, Some(&empty)]] {
            let quads = build_quads(&render, neighbors, &settings, |_, _| false);
            assert!(quads.iter().all(|q| matches!(q.dir, Dir::Up | Dir::Down)),
                "unknown neighbour invented an underwater curtain: {quads:?}");
        }
        let lower = RenderSource::from_full(&lake_source(pos::encode(6, 1, 0), 50), &blocks, &settings);
        let quads = build_quads(&render, [None, None, None, Some(&lower)], &settings, |_, _| false);
        let edge: Vec<_> = quads.iter().filter(|q| q.dir == Dir::East && q.plane == 64).collect();
        assert_eq!(edge.len(), 1, "a known underwater cliff must remain");
        assert_eq!((edge[0].v0, edge[0].v1), (50, 60));
    }

    #[test]
    fn sunlit_submerged_ground_is_occluded_across_known_section_borders() {
        let blocks = blocks();
        let settings = RenderSettings { cave_culling: false, cave_culling_height: 60, min_y: -64 };
        let source = lake_source(pos::encode(6, 0, 0), 61);
        let render = RenderSource::from_full(&source, &blocks, &settings);
        // Fresh synthetic columns carry skylight through the water to the
        // ground. A real lit neighbor is still solid, not a reason to expose
        // its compressed bed down to -64 along every 64-block tile border.
        let neighbors = [(0, -1), (0, 1), (-1, 0), (1, 0)].map(|(x, z)| {
            RenderSource::from_full(&lake_source(pos::encode(6, x, z), 61), &blocks, &settings)
        });
        let quads = build_quads(&render, neighbors.each_ref().map(Some), &settings, |_, _| false);
        let buried: Vec<_> = quads.iter().filter(|q| !matches!(q.dir, Dir::Up | Dir::Down)).collect();
        assert!(buried.is_empty(), "known equal underwater beds exposed buried sides: {buried:?}");

        let lower = RenderSource::from_full(&lake_source(pos::encode(6, 1, 0), 50), &blocks, &settings);
        let quads = build_quads(&render, [None, None, None, Some(&lower)], &settings, |_, _| false);
        let edge: Vec<_> = quads.iter().filter(|q| q.dir == Dir::East && q.plane == 64).collect();
        assert_eq!(edge.len(), 1, "the exposed part of a real underwater cliff must remain");
        assert_eq!((edge[0].v0, edge[0].v1), (50, 61));
    }

    #[test]
    fn a_dry_shore_does_not_expose_buried_ground_against_a_lit_wet_neighbor() {
        let blocks = blocks();
        let settings = RenderSettings { cave_culling: false, cave_culling_height: 60, min_y: -64 };
        let mut source = FullDataSource::empty(pos::encode(6, 0, 0));
        let column = column_from_runs(&mut source.mapping,
            &[SampledRun { min: -64, max: 70, state: STONE, biome: 1, sky: 15, block: 0 }], -64, 320);
        for x in 0..pos::WIDTH {
            for z in 0..pos::WIDTH {
                source.set_column(x, z, column.clone(), gen_step::LIGHT);
            }
        }
        let render = RenderSource::from_full(&source, &blocks, &settings);
        let wet = RenderSource::from_full(&lake_source(pos::encode(6, 1, 0), 61), &blocks, &settings);
        let quads = build_quads(&render, [None, None, None, Some(&wet)], &settings, |_, _| false);
        let edge: Vec<_> = quads.iter().filter(|q| q.dir == Dir::East && q.plane == 64).collect();
        assert_eq!(edge.len(), 1, "the real exposed bank must remain: {edge:?}");
        assert_eq!((edge[0].v0, edge[0].v1), (61, 70), "the solid neighbor occludes the buried bank");
    }

    #[test]
    fn lit_dry_borders_cull_only_covered_ground_and_keep_real_cliffs() {
        let blocks = blocks();
        let settings = RenderSettings { cave_culling: false, cave_culling_height: 60, min_y: -64 };
        let source = |section, top, state| {
            let mut full = FullDataSource::empty(section);
            let column = column_from_runs(&mut full.mapping,
                &[SampledRun { min: -64, max: top, state, biome: 1, sky: 15, block: 0 }], -64, 320);
            for x in 0..pos::WIDTH {
                for z in 0..pos::WIDTH {
                    full.set_column(x, z, column.clone(), gen_step::LIGHT);
                }
            }
            RenderSource::from_full(&full, &blocks, &settings)
        };
        let dry = source(pos::encode(6, 0, 0), 70, STONE);
        let east = pos::encode(6, 1, 0);
        for (label, neighbor, expected) in [
            ("equal opaque columns", source(east, 70, STONE), Vec::new()),
            ("lower opaque ground", source(east, 65, STONE), vec![(65, 70)]),
            ("known air", source(east, 320, AIR), vec![(-64, 70)]),
        ] {
            let quads = build_quads(&dry, [None, None, None, Some(&neighbor)], &settings, |_, _| false);
            let edge: Vec<_> = quads.iter().filter(|q| q.dir == Dir::East && q.plane == 64)
                .map(|q| (q.v0, q.v1)).collect();
            assert_eq!(edge, expected, "{label}: skylight must not turn occupied ground into an exposed wall");
        }
    }

    #[test]
    fn unknown_dry_neighbor_does_not_invent_a_cliff_but_known_air_does() {
        let blocks=blocks();
        let settings=RenderSettings { cave_culling:false,cave_culling_height:60,min_y:-64 };
        let make=|s,state| {
            let mut full=FullDataSource::empty(s);
            let column=column_from_runs(&mut full.mapping,&[SampledRun { min:-64,max:70,state,biome:1,sky:15,block:0 }],-64,320);
            for x in 0..64 { for z in 0..64 { full.set_column(x,z,column.clone(),gen_step::LIGHT); } }
            RenderSource::from_full(&full,&blocks,&settings)
        };
        let source=make(pos::encode(6,0,0),STONE);
        let missing=RenderSource::from_full(&FullDataSource::empty(pos::encode(6,1,0)),&blocks,&settings);
        let air=make(pos::encode(6,1,0),AIR);
        for (neighbor,expected,label) in [(None,vec![],"missing section"),(Some(&missing),vec![],"unknown column"),(Some(&air),vec![(-64,70)],"known air")] {
            let quads=build_quads(&source,[None,None,None,neighbor],&settings,|_,_|false);
            let east:Vec<_>=quads.iter().filter(|q|q.dir==Dir::East&&q.plane==64).map(|q|(q.v0,q.v1)).collect();
            assert_eq!(east,expected,"{label}");
        }
    }

    #[test]
    fn an_empty_light_cache_column_is_unknown_and_can_be_regenerated() {
        let mut source=FullDataSource::empty(pos::encode(6,0,0));
        let known_air=column_from_runs(&mut source.mapping,&[],-64,320);
        source.set_column(0,0,known_air.clone(),gen_step::LIGHT);
        // Reproduce old prepare's corrupted output, bypassing normal writes.
        source.gen_steps[index(0,1)]=gen_step::LIGHT;
        let encoded=storage::encode(&source,123);
        let mut recovered=storage::decode(&encoded,123).expect("legacy cache is readable");
        assert_eq!(recovered.gen_steps[index(0,1)],gen_step::EMPTY,"empty vectors must not stay complete forever");
        assert_eq!(recovered.column(0,0),known_air);
        assert_eq!(generate_missing(&mut recovered,&mut FakeSampler,-64,320),COLUMNS-1);
        assert_eq!(recovered.column(0,0),known_air,"actual air is not a missing preview");
        assert_eq!(recovered.missing_columns(),0);
        assert!(recovered.columns.iter().all(|c|!c.is_empty()));
    }

    #[test]
    fn unknown_vectors_never_replace_known_data_across_detail_levels() {
        let blocks=blocks();
        let parent_pos=pos::encode(7,0,0);
        let mut parent=FullDataSource::empty(parent_pos);
        let original=column_from_runs(&mut parent.mapping,&[SampledRun {min:-64,max:75,state:STONE,biome:1,sky:15,block:0}],-64,320);
        parent.set_column(0,0,original.clone(),gen_step::LIGHT);
        let mut invalid=FullDataSource::empty(parent_pos);
        invalid.mapping.add_if_not_present(1,AIR);
        invalid.gen_steps[index(0,0)]=gen_step::LIGHT;
        assert!(!parent.update_from(&invalid,&blocks));
        assert_eq!(parent.column(0,0),original,"empty same-detail update erased known data");

        let mut child=FullDataSource::empty(pos::child(parent_pos,0));
        assert!(!child.update_from(&invalid,&blocks));
        assert_eq!(child.missing_columns(),COLUMNS,"empty parent acquired down-sampled completeness");
        let solid=column_from_runs(&mut child.mapping,&[SampledRun {min:-64,max:60,state:STONE,biome:1,sky:15,block:0}],-64,320);
        for (x,z) in [(0,0),(0,1),(1,0)] {child.set_column(x,z,solid.clone(),gen_step::LIGHT);}
        child.gen_steps[index(1,1)]=gen_step::LIGHT;
        assert!(!parent.update_from(&child,&blocks));
        assert_eq!(parent.column(0,0),original,"incomplete 2x2 children erased the known parent");
    }

    #[test]
    fn prepared_and_cached_neighbors_update_underwater_border_visibility() {
        struct LakeSampler;
        impl ColumnSampler for LakeSampler {
            fn column(&mut self, x: i32, _: i32, out: &mut Vec<SampledRun>) {
                let floor = if x < 64 { 60 } else { 50 };
                out.extend([
                    SampledRun { min: -64, max: floor, state: STONE, biome: 1, sky: 15, block: 0 },
                    SampledRun { min: floor, max: 63, state: WATER, biome: 1, sky: 15, block: 0 },
                ]);
            }
        }
        struct LakeHost { blocks: BlockTable }
        impl Host for LakeHost {
            type Mesh = Vec<Quad>;
            fn blocks(&self) -> &BlockTable { &self.blocks }
            fn build_height(&self) -> (i32, i32) { (-64, 320) }
            fn sampler(&self) -> Option<Box<dyn ColumnSampler>> { Some(Box::new(LakeSampler)) }
            fn mesh(&self, _: i64, _: &FullDataSource, quads: &[Quad], _: bool) -> Vec<Quad> { quads.to_vec() }
        }
        let target = pos::encode(6, 0, 0);
        let settings = engine::WorkSettings {
            render: RenderSettings { cave_culling: false, cave_culling_height: 60, min_y: -64 },
            generation: true,
            debug: false,
        };
        let plan = DetailPlan::new(HorizontalQuality::Low, 8);
        let settled = |engine: &mut engine::Engine<LakeHost>, near: engine::Near| {
            let mut last = None;
            let start = std::time::Instant::now();
            loop {
                let (meshes, _) = engine.update([8., 8.], plan, near.clone());
                for mesh in meshes {
                    if mesh.section == target { last = mesh.mesh; }
                }
                if engine.stats().queued == 0 && last.is_some() { break; }
                assert!(start.elapsed().as_secs() < 10, "lake border did not settle");
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            let quads = last.unwrap();
            let edge: Vec<_> = quads.iter().filter(|q| q.dir == Dir::East && q.plane == 64).collect();
            assert_eq!(edge.len(), 1, "prepared neighbour did not remesh the older tile");
            assert_eq!((edge[0].v0, edge[0].v1), (50, 60), "border still exposes hidden deep material");
        };
        let mut engine = engine::Engine::new(LakeHost { blocks: blocks() }, None, 1, settings, 1);
        settled(&mut engine, engine::Near::default());
        drop(engine);

        // The neighbouring tile is fully near and never prepared by this
        // detail plan. Its existing disk data must still hide the deep wall.
        let dir = std::env::temp_dir().join(format!("dh-cached-water-border-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        storage::save(&dir, &lake_source(target, 60), 1).unwrap();
        storage::save(&dir, &lake_source(pos::encode(6, 1, 0), 50), 1).unwrap();
        let near = engine::Near {
            center: (0, 0), radius: 8,
            loaded: (4..8).flat_map(|x| (0..4).map(move |z| (x, z))).collect(),
        };
        let mut engine = engine::Engine::new(LakeHost { blocks: blocks() }, Some(dir.clone()), 1,
            engine::WorkSettings { generation: false, ..settings }, 1);
        settled(&mut engine, near);
        drop(engine);
        // Only this test's two files and level directory, never a recursive
        // deletion or any gameplay world.
        for s in [target, pos::encode(6, 1, 0)] { std::fs::remove_file(storage::path(&dir, s)).unwrap(); }
        std::fs::remove_dir(dir.join("6")).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn side_faces_are_cut_by_the_adjacent_column() {
        let b = blocks();
        let mut mapping = IdMap::default();
        let air = mapping.add_if_not_present(1, AIR);
        let stone = mapping.add_if_not_present(1, STONE);
        let mut source = FullDataSource::empty(pos::encode(6, 0, 0));
        source.mapping = mapping;
        for x in 0..pos::WIDTH {
            for z in 0..pos::WIDTH {
                let h = if x < 32 { 100 } else { 90 };
                let column = vec![
                    full_point::encode(air, 384 - h, h, 0, 15).unwrap(),
                    full_point::encode(stone, h, 0, 0, 15).unwrap(),
                ];
                source.set_column(x, z, column, gen_step::LIGHT);
            }
        }
        let settings = RenderSettings {
            cave_culling: false,
            cave_culling_height: 60,
            min_y: -64,
        };
        let render = RenderSource::from_full(&source, &b, &settings);
        let quads = build_quads(&render, [None; 4], &settings, |_, _| false);
        // The step between the two heights: one east face, 10 blocks tall.
        let step: Vec<_> = quads.iter().filter(|q| q.dir == Dir::East && q.plane == 32).collect();
        assert_eq!(step.len(), 1);
        assert_eq!(
            (step[0].v0, step[0].v1, step[0].u0, step[0].u1),
            (90 - 64, 100 - 64, 0, 64)
        );
    }

    #[test]
    fn sections_round_trip_through_their_files() {
        let chunk = FlatChunk {
            x: 2,
            z: 3,
            height: |x, z| 60 + (x + z) % 7,
        };
        let source = create_from_chunk(&chunk, &blocks());
        let bytes = storage::encode(&source, 42);
        assert_eq!(storage::decode(&bytes, 42), Some(source.clone()));
        assert_eq!(storage::decode(&bytes, 43), None);
        assert_eq!(storage::decode(&bytes[..bytes.len() - 4], 42), None);
        let dir = std::env::temp_dir().join(format!("dh-test-{}", std::process::id()));
        storage::save(&dir, &source, 7).unwrap();
        assert_eq!(storage::load(&dir, source.pos, 7), Some(source));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn settings_read_back_what_they_store() {
        let mut s = Settings::default();
        s.click(1);
        s.click(2);
        s.click(5);
        let stored: std::collections::HashMap<_, _> = s.stored().into_iter().collect();
        let read = Settings::read(|k| stored.get(k).cloned());
        assert_eq!(read, s);
        assert_eq!(read.radius_chunks(), 512);
        assert_eq!(read.quality, HorizontalQuality::High);
        assert!(read.debug);
        assert_eq!(Settings::read(|_| Some("junk".into())), Settings::default());
        let mut c = Settings::default();
        assert!(c.command(&["distance", "100"]).is_some());
        assert_eq!(c.radius_chunks(), 128);
        assert!(c.command(&["quality", "extreme"]).is_some());
        assert!(c.command(&["nonsense"]).is_none());
    }

    #[test]
    fn every_completed_command_is_one_the_mod_runs() {
        // `status`, `save` and `clear` act on the session (bridge.rs); every
        // other completion must change a setting.
        for &(word, values) in COMMANDS {
            if matches!(word, "status" | "save" | "clear") {
                assert!(values.is_empty());
                continue;
            }
            let runs = |args: &[&str]| Settings::default().command(args).is_some();
            if values.is_empty() {
                assert!(runs(&[word]), "{word}");
            }
            for value in values {
                assert!(runs(&[word, value]), "{word} {value}");
            }
        }
        // The distances offered are the settings' own.
        let distance = COMMANDS.iter().find(|c| c.0 == "distance").unwrap().1;
        assert_eq!(
            distance,
            &DISTANCES
                .map(|d| d.to_string())
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()[..]
        );
    }

    struct FakeHost {
        blocks: BlockTable,
    }

    #[test]
    fn parity_moving_near_holes_with_equal_column_counts_remeshes_their_positions() {
        struct GeometryHost(BlockTable);
        impl Host for GeometryHost {
            type Mesh = Vec<Quad>;
            fn blocks(&self) -> &BlockTable {
                &self.0
            }
            fn build_height(&self) -> (i32, i32) {
                (-64, 320)
            }
            fn sampler(&self) -> Option<Box<dyn ColumnSampler>> {
                Some(Box::new(FakeSampler))
            }
            fn mesh(&self, _: i64, _: &FullDataSource, quads: &[Quad], _: bool) -> Vec<Quad> {
                quads.to_vec()
            }
        }
        let settings = engine::WorkSettings {
            render: RenderSettings {
                cave_culling: false,
                cave_culling_height: 60,
                min_y: -64,
            },
            generation: true,
            debug: false,
        };
        let mut e = engine::Engine::new(GeometryHost(blocks()), None, 1, settings, 1);
        let plan = DetailPlan::new(HorizontalQuality::Low, 8);
        let mut near = engine::Near {
            center: (0, 0),
            radius: 8,
            loaded: [(0, 0)].into_iter().collect(),
        };
        let section = pos::encode(6, 0, 0);
        let covers = |quads: &[Quad], x, z| {
            quads
                .iter()
                .any(|q| q.dir == Dir::Up && q.u0 <= x && x < q.u1 && q.v0 <= z && z < q.v1)
        };
        let start = std::time::Instant::now();
        let mut initial = None;
        loop {
            let (meshes, _) = e.update([8., 8.], plan, near.clone());
            for mesh in meshes {
                if mesh.section == section {
                    initial = mesh.mesh;
                }
            }
            if initial.is_some() && e.stats().queued == 0 {
                break;
            }
            assert!(start.elapsed().as_secs() < 10, "initial near/far scene did not settle");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let initial = initial.unwrap();
        assert!(!covers(&initial, 8, 8));
        assert!(covers(&initial, 24, 8));
        // Still one GPU-complete near column in this section, but a different
        // one. The old hole must fill; the new near column must be cut out.
        near.loaded = [(1, 0)].into_iter().collect();
        let start = std::time::Instant::now();
        loop {
            let (meshes, _) = e.update([8., 8.], plan, near.clone());
            if let Some(quads) = meshes.into_iter().find(|m| m.section == section).and_then(|m| m.mesh) {
                assert!(
                    covers(&quads, 8, 8),
                    "departed near column still has a far-terrain hole"
                );
                assert!(!covers(&quads, 24, 8), "new near column has duplicate far geometry");
                break;
            }
            assert!(
                start.elapsed().as_secs_f32() < 2.,
                "equal-count near movement never remeshed the holes"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    #[test]
    fn parity_fluid_top_and_sides_meet_native_height_without_lowering_a_roof() {
        let mut full = FullDataSource::empty(pos::encode(6, 0, 0));
        let [air, water, stone] = [AIR, WATER, STONE].map(|s| full.mapping.add_if_not_present(1, s));
        for x in 0..pos::WIDTH {
            for z in 0..pos::WIDTH {
                let column = if x >= 32 {
                    vec![
                        full_point::encode(air, 120, 112, 0, 15).unwrap(),
                        full_point::encode(stone, 112, 0, 0, 0).unwrap(),
                    ]
                } else if z < 4 {
                    // A roof hides the top but not the open east side.
                    vec![
                        full_point::encode(air, 99, 133, 0, 15).unwrap(),
                        full_point::encode(stone, 1, 132, 0, 15).unwrap(),
                        full_point::encode(water, 20, 112, 0, 15).unwrap(),
                        full_point::encode(stone, 112, 0, 0, 0).unwrap(),
                    ]
                } else {
                    vec![
                        full_point::encode(air, 100, 132, 0, 15).unwrap(),
                        full_point::encode(water, 20, 112, 0, 15).unwrap(),
                        full_point::encode(stone, 112, 0, 0, 0).unwrap(),
                    ]
                };
                full.set_column(x, z, column, gen_step::LIGHT);
            }
        }
        let settings = RenderSettings {
            cave_culling: false,
            cave_culling_height: 60,
            min_y: -64,
        };
        let render = RenderSource::from_full(&full, &blocks(), &settings);
        let quads = build_quads(&render, [None; 4], &settings, |_, _| false);
        // Horizon's detailed still-fluid interior: own_height 8/9 minus
        // FluidRenderer's 0.001 epsilon. Expected value is independent of DH.
        let native_surface = 67. + 8. / 9. - 0.001;
        let top = quads.iter().find(|q| q.id == water && q.dir == Dir::Up).unwrap();
        assert!(
            (top.geometry().0 - native_surface).abs() < 0.00002,
            "far water top differs from detailed water"
        );
        let sides: Vec<_> = quads
            .iter()
            .filter(|q| q.id == water && q.dir == Dir::East && q.plane == 32)
            .collect();
        assert!(
            sides
                .iter()
                .any(|q| q.u0 >= 4 && (q.geometry().1[3] - native_surface).abs() < 0.00002),
            "open water side must meet its lowered top"
        );
        assert!(
            sides.iter().any(|q| q.u1 <= 4 && q.geometry().1[3] == 68.),
            "water beneath a solid roof must still fill the whole block"
        );
    }

    #[test]
    fn parity_a_worker_cannot_publish_holes_from_an_old_near_snapshot() {
        use std::sync::{
            Arc,
            atomic::{AtomicU8, Ordering},
        };
        struct DelayedHost {
            blocks: BlockTable,
            gate: Arc<AtomicU8>,
        }
        impl Host for DelayedHost {
            type Mesh = Vec<Quad>;
            fn blocks(&self) -> &BlockTable {
                &self.blocks
            }
            fn build_height(&self) -> (i32, i32) {
                (-64, 320)
            }
            fn sampler(&self) -> Option<Box<dyn ColumnSampler>> {
                Some(Box::new(FakeSampler))
            }
            fn mesh(&self, section: i64, _: &FullDataSource, quads: &[Quad], _: bool) -> Vec<Quad> {
                if section == pos::encode(6, 0, 0)
                    && self
                        .gate
                        .compare_exchange(1, 2, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                {
                    let start = std::time::Instant::now();
                    while self.gate.load(Ordering::SeqCst) == 2 && start.elapsed().as_secs() < 5 {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
                quads.to_vec()
            }
        }
        let gate = Arc::new(AtomicU8::new(1));
        let host = DelayedHost {
            blocks: blocks(),
            gate: gate.clone(),
        };
        let settings = engine::WorkSettings {
            render: RenderSettings {
                cave_culling: false,
                cave_culling_height: 60,
                min_y: -64,
            },
            generation: true,
            debug: false,
        };
        let mut e = engine::Engine::new(host, None, 1, settings, 1);
        let plan = DetailPlan::new(HorizontalQuality::Low, 8);
        let mut near = engine::Near {
            center: (0, 0),
            radius: 8,
            loaded: [(0, 0)].into_iter().collect(),
        };
        let start = std::time::Instant::now();
        while gate.load(Ordering::SeqCst) != 2 {
            e.update([8., 8.], plan, near.clone());
            assert!(
                start.elapsed().as_secs() < 10,
                "worker never reached the controlled handoff"
            );
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        // The old mesh already contains its holes. Move near coverage before
        // letting that worker return, including on the first mesh of a tile.
        near.loaded = [(1, 0)].into_iter().collect();
        e.update([8., 8.], plan, near.clone());
        gate.store(3, Ordering::SeqCst);
        let start = std::time::Instant::now();
        loop {
            let (meshes, _) = e.update([8., 8.], plan, near.clone());
            if let Some(quads) = meshes
                .into_iter()
                .find(|m| m.section == pos::encode(6, 0, 0))
                .and_then(|m| m.mesh)
            {
                let covers = |x, z| {
                    quads
                        .iter()
                        .any(|q| q.dir == Dir::Up && q.u0 <= x && x < q.u1 && q.v0 <= z && z < q.v1)
                };
                assert!(
                    covers(8, 8),
                    "a stale worker published a hole in the departed near column"
                );
                assert!(
                    !covers(24, 8),
                    "a stale worker published water over the new near column"
                );
                break;
            }
            assert!(start.elapsed().as_secs() < 10, "updated geometry never arrived");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    #[test]
    fn shrinking_near32_retains_detail_until_its_delayed_far_remesh_is_published() {
        use std::sync::{Arc, atomic::{AtomicU8, Ordering}};
        struct DelayedHost { blocks: BlockTable, gate: Arc<AtomicU8> }
        impl Host for DelayedHost {
            type Mesh = Vec<Quad>;
            fn blocks(&self) -> &BlockTable { &self.blocks }
            fn build_height(&self) -> (i32, i32) { (-64, 320) }
            fn sampler(&self) -> Option<Box<dyn ColumnSampler>> { Some(Box::new(FakeSampler)) }
            fn mesh(&self, _: i64, _: &FullDataSource, quads: &[Quad], _: bool) -> Vec<Quad> {
                if self.gate.compare_exchange(1,2,Ordering::SeqCst,Ordering::SeqCst).is_ok() {
                    let start = std::time::Instant::now();
                    while self.gate.load(Ordering::SeqCst) == 2 && start.elapsed().as_secs() < 5 {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
                quads.to_vec()
            }
        }
        let gate = Arc::new(AtomicU8::new(0));
        let settings = engine::WorkSettings {
            render: RenderSettings { cave_culling:false, cave_culling_height:60, min_y:-64 },
            generation:true, debug:false,
        };
        let mut e = engine::Engine::new(DelayedHost { blocks:blocks(), gate:gate.clone() },None,1,settings,1);
        let plan = DetailPlan::new(HorizontalQuality::Low,8);
        let mut near = engine::Near { center:(0,0), radius:32, loaded:[(3,0)].into_iter().collect() };
        let mut published = std::collections::HashSet::new();
        let start = std::time::Instant::now();
        loop {
            e.update([8.,8.],plan,near.clone());
            if let Some(cutout) = e.take_cutout() { published = cutout; }
            if e.stats().queued == 0 && e.stats().shown > 0 { break; }
            assert!(start.elapsed().as_secs() < 10,"initial handoff did not settle");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(published.contains(&(3,0)));
        gate.store(1,Ordering::SeqCst);
        near.radius = 1;
        let start = std::time::Instant::now();
        while gate.load(Ordering::SeqCst) != 2 {
            e.update([8.,8.],plan,near.clone());
            if let Some(cutout) = e.take_cutout() { published = cutout; }
            assert!(start.elapsed().as_secs() < 5,"remesh never reached gate");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(published.contains(&(3,0)),"near geometry was released before its far replacement");
        // More than Horizon's ordinary 0.6s fade may pass with no replacement.
        std::thread::sleep(std::time::Duration::from_millis(650));
        let (meshes,_) = e.update([8.,8.],plan,near.clone());
        assert!(meshes.is_empty());
        assert!(e.take_cutout().is_none());
        gate.store(3,Ordering::SeqCst);
        let start = std::time::Instant::now();
        loop {
            let (meshes,_) = e.update([8.,8.],plan,near.clone());
            if let Some(cutout) = e.take_cutout()
                && !cutout.contains(&(3,0)) {
                assert!(!meshes.is_empty(),"cutout must share its replacement mesh batch");
                break;
            }
            assert!(start.elapsed().as_secs() < 5,"departing near column retained forever");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        // A quality change can retire a holed tile, then make that same tile
        // wanted again before its replacements finish. Its published mask,
        // not the immediately previous plan's missing entry, must win.
        let coarse = DetailPlan::new(HorizontalQuality::Lowest,32);
        let fine = DetailPlan::new(HorizontalQuality::Extreme,32);
        near.radius = 32;
        near.loaded = [(15,0)].into_iter().collect();
        let start = std::time::Instant::now();
        loop {
            e.update([8.,8.],coarse,near.clone());
            if let Some(cutout) = e.take_cutout() { published = cutout; }
            if e.stats().queued == 0 { break; }
            assert!(start.elapsed().as_secs() < 10,"coarse scene did not settle");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(published.contains(&(15,0)));
        gate.store(1,Ordering::SeqCst);
        let start = std::time::Instant::now();
        while gate.load(Ordering::SeqCst) != 2 {
            e.update([8.,8.],fine,near.clone());
            if let Some(cutout) = e.take_cutout() { published = cutout; }
            assert!(start.elapsed().as_secs() < 5,"quality replacement never reached gate");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        near.radius = 2;
        e.update([8.,8.],coarse,near.clone());
        if let Some(cutout) = e.take_cutout() { published = cutout; }
        assert!(published.contains(&(15,0)),"a retiring tile forgot its published hole");
        gate.store(3,Ordering::SeqCst);
        let start = std::time::Instant::now();
        loop {
            let (meshes,_) = e.update([8.,8.],coarse,near.clone());
            if let Some(cutout) = e.take_cutout()
                && !cutout.contains(&(15,0)) {
                assert!(!meshes.is_empty(),"reused stale tile was not remeshed");
                break;
            }
            assert!(start.elapsed().as_secs() < 10,"quality reversal left a permanent stale hole");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    #[test]
    fn fully_near_tiles_keep_detail_until_their_first_far_mesh_after_near32_to8() {
        use std::sync::{Arc, atomic::{AtomicU8, Ordering}};
        struct DelayedHost { blocks: BlockTable, gate: Arc<AtomicU8> }
        impl Host for DelayedHost {
            type Mesh = Vec<Quad>;
            fn blocks(&self) -> &BlockTable { &self.blocks }
            fn build_height(&self) -> (i32, i32) { (-64, 320) }
            fn sampler(&self) -> Option<Box<dyn ColumnSampler>> { Some(Box::new(FakeSampler)) }
            fn mesh(&self, _: i64, _: &FullDataSource, quads: &[Quad], _: bool) -> Vec<Quad> {
                if self.gate.compare_exchange(1,2,Ordering::SeqCst,Ordering::SeqCst).is_ok() {
                    let start = std::time::Instant::now();
                    while self.gate.load(Ordering::SeqCst) == 2 && start.elapsed().as_secs() < 5 {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
                quads.to_vec()
            }
        }
        let gate = Arc::new(AtomicU8::new(0));
        let settings = engine::WorkSettings {
            render: RenderSettings { cave_culling:false, cave_culling_height:60, min_y:-64 },
            generation:true, debug:false,
        };
        let mut e = engine::Engine::new(DelayedHost { blocks:blocks(), gate:gate.clone() },None,1,settings,1);
        let plan = DetailPlan::new(HorizontalQuality::Low,16);
        let mut near = engine::Near { center:(0,0), radius:32, loaded:Default::default() };
        near.loaded.extend((-32..=32).flat_map(|x| (-32..=32).map(move |z| (x,z))));
        let (initial, _) = e.update([8.,8.],plan,near.clone());
        assert!(initial.is_empty());
        assert_eq!(e.stats().wanted,0,"the full near view must skip whole far tiles");
        let mut published = e.take_cutout().expect("initial near coverage");
        assert!(published.contains(&(12,0)) && published.contains(&(-13,0)));
        assert!(published.contains(&(31,0)));
        // These columns have no old far mesh or published mask. Shrinking
        // creates their first job; a partial-hole remesh cannot test this case.
        gate.store(1,Ordering::SeqCst);
        near.radius = 8;
        let previous = near.clone();
        near.loaded.retain(|&(x,z)| previous.covers(x,z));
        let start = std::time::Instant::now();
        while gate.load(Ordering::SeqCst) != 2 {
            e.update([8.,8.],plan,near.clone());
            if let Some(cutout) = e.take_cutout() { published = cutout; }
            assert!(start.elapsed().as_secs() < 5,"first far mesh never reached gate");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(published.contains(&(12,0)),"fully near tile lost its positive column");
        assert!(published.contains(&(-13,0)),"fully near tile lost its negative column");
        assert!(!published.contains(&(31,0)),"a column outside DH range retained its mesh");
        std::thread::sleep(std::time::Duration::from_millis(650));
        assert!(e.update([8.,8.],plan,near.clone()).0.is_empty());
        assert!(e.take_cutout().is_none(),"detail left after the ordinary renderer fade");
        gate.store(3,Ordering::SeqCst);
        let start = std::time::Instant::now();
        let mut waiting: std::collections::HashSet<_> = [(12,0),(-13,0)].into_iter().collect();
        while !waiting.is_empty() {
            let (meshes,_) = e.update([8.,8.],plan,near.clone());
            if let Some(cutout) = e.take_cutout() {
                waiting.retain(|&(x,z)| {
                    if cutout.contains(&(x,z)) { return true; }
                    let replacement = meshes.iter().find(|m| {
                        pos::contains(m.section,pos::containing_block(6,x*16,z*16))
                    }).expect("detail released without its first far mesh in this batch");
                    let local_x = x*16 + 8 - pos::min_corner_block_x(replacement.section);
                    let local_z = z*16 + 8 - pos::min_corner_block_z(replacement.section);
                    assert!(replacement.mesh.as_ref().is_some_and(|quads| quads.iter().any(|q| {
                        q.dir == Dir::Up && q.u0 <= local_x && local_x < q.u1
                            && q.v0 <= local_z && local_z < q.v1
                    })),"replacement still has a hole in the released column");
                    false
                });
                published = cutout;
            }
            assert!(start.elapsed().as_secs() < 10,"first far mesh never released its stand-in");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(!published.contains(&(12,0)) && !published.contains(&(-13,0)));
        // Moving far away must also drop every retained column; only current
        // wanted coverage may hold a near stand-in, never the travel history.
        let elsewhere = engine::Near { center:(1000,1000), radius:8, loaded:Default::default() };
        e.update([16008.,16008.],plan,elsewhere);
        assert!(e.take_cutout().is_some_and(|cutout| cutout.is_empty()));
    }

    #[test]
    fn clearing_a_stationary_view_regenerates_its_still_visible_sections() {
        let settings = engine::WorkSettings {
            render: RenderSettings { cave_culling:false, cave_culling_height:60, min_y:-64 },
            generation:true, debug:false,
        };
        let mut e = engine::Engine::new(FakeHost { blocks:blocks() },None,1,settings,1);
        let plan = DetailPlan::new(HorizontalQuality::Low,8);
        let start = std::time::Instant::now();
        loop {
            e.update([8.,8.],plan,engine::Near::default());
            if e.stats().queued == 0 && e.stats().shown > 0 { break; }
            assert!(start.elapsed().as_secs() < 10,"initial view did not settle");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let before = e.stats().generated;
        e.wipe();
        let start = std::time::Instant::now();
        let mut replacement = false;
        loop {
            let (meshes,_) = e.update([8.,8.],plan,engine::Near::default());
            replacement |= !meshes.is_empty();
            if replacement && e.stats().generated > before && e.stats().queued == 0 { break; }
            assert!(start.elapsed().as_secs() < 10,"clear retained old meshes without regenerating their data");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    #[test]
    fn completed_features_replace_preview_while_player_is_stationary() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        struct RefinedSampler {
            ready: Arc<AtomicBool>,
            sent: bool,
        }
        impl ColumnSampler for RefinedSampler {
            fn column(&mut self, x: i32, z: i32, out: &mut Vec<SampledRun>) {
                FakeSampler.column(x, z, out);
            }
            fn drain_refined(&mut self) -> Vec<FullDataSource> {
                if self.sent || !self.ready.load(Ordering::Relaxed) {
                    return Vec::new();
                }
                self.sent = true;
                let mut source = FullDataSource::empty(pos::encode(6, 0, 0));
                let column = column_from_runs(
                    &mut source.mapping,
                    &[SampledRun {
                        min: -64,
                        max: 80,
                        state: GRASS,
                        biome: 1,
                        sky: 15,
                        block: 0,
                    }],
                    -64,
                    320,
                );
                source.set_column(0, 0, column, gen_step::FEATURES);
                vec![source]
            }
        }
        struct RefinedHost {
            blocks: BlockTable,
            ready: Arc<AtomicBool>,
        }
        impl Host for RefinedHost {
            type Mesh = bool;
            fn blocks(&self) -> &BlockTable {
                &self.blocks
            }
            fn build_height(&self) -> (i32, i32) {
                (-64, 320)
            }
            fn sampler(&self) -> Option<Box<dyn ColumnSampler>> {
                Some(Box::new(RefinedSampler {
                    ready: self.ready.clone(),
                    sent: false,
                }))
            }
            fn mesh(&self, _: i64, data: &FullDataSource, _: &[Quad], _: bool) -> bool {
                data.gen_steps.contains(&gen_step::FEATURES)
            }
        }
        let ready = Arc::new(AtomicBool::new(false));
        let host = RefinedHost {
            blocks: blocks(),
            ready: ready.clone(),
        };
        let settings = engine::WorkSettings {
            render: RenderSettings {
                cave_culling: false,
                cave_culling_height: 60,
                min_y: -64,
            },
            generation: true,
            debug: false,
        };
        let mut e = engine::Engine::new(host, None, 1, settings, 1);
        let plan = DetailPlan::new(HorizontalQuality::Low, 8);
        let started = std::time::Instant::now();
        loop {
            let (meshes, _) = e.update([8., 8.], plan, engine::Near::default());
            if meshes.iter().any(|m| m.mesh == Some(true)) {
                break;
            }
            let stats = e.stats();
            if stats.shown > 0 && stats.queued == 0 {
                ready.store(true, Ordering::Relaxed);
            }
            assert!(
                started.elapsed().as_secs() < 10,
                "stationary refinement never appeared: {stats:?}"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(e.stats().refined > 0);
    }
    #[test]
    fn delayed_older_mesh_cannot_replace_newer_data_and_settings() {
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
        struct BlockingHost {
            blocks: BlockTable,
            armed: Arc<AtomicBool>,
            held: Arc<AtomicBool>,
            release: Arc<AtomicBool>,
        }
        impl Host for BlockingHost {
            type Mesh = (bool, bool);
            fn blocks(&self) -> &BlockTable { &self.blocks }
            fn build_height(&self) -> (i32,i32) { (-64,320) }
            fn sampler(&self) -> Option<Box<dyn ColumnSampler>> { Some(Box::new(FakeSampler)) }
            fn mesh(&self, section: i64, data: &FullDataSource, _: &[Quad], debug: bool) -> Self::Mesh {
                let updated = data.gen_steps[index(0,0)] == gen_step::LIGHT;
                if section == pos::encode(6,0,0) && !debug && self.armed.swap(false, Ordering::SeqCst) {
                    self.held.store(true, Ordering::SeqCst);
                    while !self.release.load(Ordering::SeqCst) {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
                (debug, updated)
            }
        }
        // On any failed assertion, release the blocked worker before Engine::drop joins it.
        struct ReleaseOnDrop(Arc<AtomicBool>);
        impl Drop for ReleaseOnDrop {
            fn drop(&mut self) { self.0.store(true, Ordering::SeqCst); }
        }
        let armed = Arc::new(AtomicBool::new(false));
        let held = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let host = BlockingHost { blocks: blocks(), armed: armed.clone(), held: held.clone(), release: release.clone() };
        let mut settings = engine::WorkSettings {
            render: RenderSettings { cave_culling:false, cave_culling_height:60, min_y:-64 },
            generation:true, debug:false,
        };
        let mut e = engine::Engine::new(host, None, 1, settings, 2);
        let _release_on_drop = ReleaseOnDrop(release.clone());
        let plan = DetailPlan::new(HorizontalQuality::Low, 8);
        let section = pos::encode(6,0,0);
        let tick = std::time::Duration::from_millis(2);
        let started = std::time::Instant::now();
        loop {
            e.update([8.,8.],plan,engine::Near::default());
            if e.stats().shown > 0 && e.stats().queued == 0 { break; }
            assert!(started.elapsed().as_secs() < 10, "initial mesh did not settle");
            std::thread::sleep(tick);
        }
        e.update([8.,8.],plan,engine::Near::default());
        armed.store(true,Ordering::SeqCst);
        settings.render.cave_culling = true;
        e.set_settings(settings);
        let started = std::time::Instant::now();
        while !held.load(Ordering::SeqCst) {
            e.update([8.,8.],plan,engine::Near::default());
            assert!(started.elapsed().as_secs() < 5, "older mesh was not held");
            std::thread::sleep(tick);
        }
        // A second worker handles real chunk data and new settings while the
        // old host-side mesh is deliberately delayed. It must remain current
        // after the older result finally arrives, even with identical cutouts.
        e.add_chunk(Box::new(FlatChunk { x:0, z:0, height:|_,_|80 }));
        settings.debug = true;
        e.set_settings(settings);
        let started = std::time::Instant::now();
        loop {
            let (done,_) = e.update([8.,8.],plan,engine::Near::default());
            if done.iter().any(|m|m.section==section && m.mesh==Some((true,true))) { break; }
            assert!(started.elapsed().as_secs() < 5, "newer mesh did not publish on the second worker");
            std::thread::sleep(tick);
        }
        release.store(true,Ordering::SeqCst);
        let started = std::time::Instant::now();
        let mut obsolete = false;
        loop {
            let (done,_) = e.update([8.,8.],plan,engine::Near::default());
            obsolete |= done.iter().any(|m|m.section==section && m.mesh.is_some_and(|state|state!=(true,true)));
            if e.stats().queued == 0 {
                let (done,_) = e.update([8.,8.],plan,engine::Near::default());
                obsolete |= done.iter().any(|m|m.section==section && m.mesh.is_some_and(|state|state!=(true,true)));
                break;
            }
            assert!(started.elapsed().as_secs() < 5, "queue remained stuck after delayed mesh finished");
            std::thread::sleep(tick);
        }
        assert!(!obsolete, "older data/settings overwrote an already published current mesh");
    }

    struct FakeSampler;
    impl ColumnSampler for FakeSampler {
        fn column(&mut self, x: i32, z: i32, out: &mut Vec<SampledRun>) {
            let h = 60 + ((x / 64 + z / 64).rem_euclid(5));
            out.push(SampledRun {
                min: -64,
                max: h,
                state: GRASS,
                biome: 1,
                sky: 15,
                block: 0,
            });
        }
    }
    impl Host for FakeHost {
        type Mesh = usize;
        fn blocks(&self) -> &BlockTable {
            &self.blocks
        }
        fn build_height(&self) -> (i32, i32) {
            (-64, 320)
        }
        fn sampler(&self) -> Option<Box<dyn ColumnSampler>> {
            Some(Box::new(FakeSampler))
        }
        fn mesh(&self, _: i64, _: &FullDataSource, quads: &[Quad], _: bool) -> usize {
            quads.len()
        }
    }

    #[test]
    fn refused_columns_remain_missing_and_can_be_recovered() {
        struct Gate(bool);
        impl ColumnSampler for Gate {
            fn column(&mut self, x: i32, z: i32, out: &mut Vec<SampledRun>) {
                FakeSampler.column(x, z, out);
            }
            fn try_column(&mut self, x: i32, z: i32, out: &mut Vec<SampledRun>) -> bool {
                if !self.0 {
                    return false;
                }
                self.column(x, z, out);
                true
            }
        }
        let mut source = FullDataSource::empty(pos::encode(6, -1, -1));
        let count = source.missing_columns();
        assert!(count > 0);
        let mut gate = Gate(false);
        assert_eq!(generate_missing(&mut source, &mut gate, -64, 2032), 0);
        assert_eq!(source.missing_columns(), count);
        gate.0 = true;
        assert_eq!(generate_missing(&mut source, &mut gate, -64, 2032), count);
        assert_eq!(source.missing_columns(), 0);
    }

    #[test]
    fn stationary_player_recovers_after_server_refusal() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        struct RecoveringHost {
            inner: FakeHost,
            ready: Arc<AtomicBool>,
        }
        struct RecoveringSampler(Arc<AtomicBool>);
        impl ColumnSampler for RecoveringSampler {
            fn column(&mut self, x: i32, z: i32, out: &mut Vec<SampledRun>) {
                FakeSampler.column(x, z, out);
            }
            fn try_column(&mut self, x: i32, z: i32, out: &mut Vec<SampledRun>) -> bool {
                if !self.0.load(Ordering::Relaxed) {
                    return false;
                }
                self.column(x, z, out);
                true
            }
        }
        impl Host for RecoveringHost {
            type Mesh = usize;
            fn blocks(&self) -> &BlockTable {
                self.inner.blocks()
            }
            fn build_height(&self) -> (i32, i32) {
                (-64, 320)
            }
            fn sampler(&self) -> Option<Box<dyn ColumnSampler>> {
                Some(Box::new(RecoveringSampler(self.ready.clone())))
            }
            fn mesh(&self, _: i64, _: &FullDataSource, quads: &[Quad], _: bool) -> usize {
                quads.len()
            }
        }
        let ready = Arc::new(AtomicBool::new(false));
        let settings = engine::WorkSettings {
            render: RenderSettings {
                cave_culling: false,
                cave_culling_height: 60,
                min_y: -64,
            },
            generation: true,
            debug: false,
        };
        let host = RecoveringHost {
            inner: FakeHost { blocks: blocks() },
            ready: ready.clone(),
        };
        let mut e = engine::Engine::new(host, None, 1, settings, 1);
        let plan = DetailPlan::new(HorizontalQuality::Low, 8);
        let start = std::time::Instant::now();
        let mut released = false;
        loop {
            e.update([-8.0, -8.0], plan, engine::Near::default());
            let stats = e.stats();
            if !released && stats.wanted > 0 && stats.queued == 0 {
                assert_eq!(stats.shown, 0);
                ready.store(true, Ordering::Relaxed);
                released = true;
            }
            if released && stats.shown == stats.wanted && stats.wanted > 0 {
                break;
            }
            assert!(
                start.elapsed() < std::time::Duration::from_secs(20),
                "server data never recovered"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn retiring_a_busy_engine_does_not_wait_and_reopens_only_after_cache_flush() {
        use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}, mpsc};
        use std::time::{Duration, Instant};
        struct BusyHost {
            blocks: BlockTable,
            first: AtomicBool,
            entered: mpsc::Sender<()>,
            release: Arc<Mutex<mpsc::Receiver<()>>>,
        }
        impl Host for BusyHost {
            type Mesh = usize;
            fn blocks(&self) -> &BlockTable { &self.blocks }
            fn build_height(&self) -> (i32, i32) { (-64, 320) }
            fn sampler(&self) -> Option<Box<dyn ColumnSampler>> { Some(Box::new(FakeSampler)) }
            fn mesh(&self, _: i64, _: &FullDataSource, quads: &[Quad], _: bool) -> usize {
                if self.first.swap(false, Ordering::AcqRel) {
                    self.entered.send(()).unwrap();
                    self.release.lock().unwrap().recv_timeout(Duration::from_secs(10)).unwrap();
                }
                quads.len()
            }
        }
        let (entered, entry) = mpsc::channel();
        let (release, waiting) = mpsc::channel();
        let dir = std::env::temp_dir().join(format!("dh-retirement-{}", std::process::id()));
        assert!(!dir.exists(), "retirement fixture must own a new directory");
        let host = BusyHost { blocks: blocks(), first: AtomicBool::new(true), entered, release: Arc::new(Mutex::new(waiting)) };
        let settings = engine::WorkSettings {
            render: RenderSettings { cave_culling: false, cave_culling_height: 60, min_y: -64 },
            generation: true, debug: false,
        };
        let mut e = engine::Engine::new(host, Some(dir.clone()), 1, settings, 1);
        e.add_chunk(Box::new(FlatChunk { x: 0, z: 0, height: |_, _| 70 }));
        let start = Instant::now();
        while e.stats().chunks == 0 {
            assert!(start.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(2));
        }
        e.update([8., 8.], DetailPlan::new(HorizontalQuality::Low, 8), engine::Near::default());
        entry.recv_timeout(Duration::from_secs(5)).unwrap();
        let mut retiring = engine::Retirement::new().unwrap();
        assert!(retiring.try_retire(e).is_ok());
        // This is reached while the real mesh worker is still blocked. Joining
        // Engine synchronously here would prevent us from releasing that worker.
        assert!(retiring.busy());
        assert!(!retiring.ready(), "rapid off/on must not start a second cache writer");
        release.send(()).unwrap();
        let start = Instant::now();
        while !retiring.ready() {
            assert!(!retiring.failed());
            assert!(start.elapsed() < Duration::from_secs(10));
            std::thread::sleep(Duration::from_millis(2));
        }
        // Readiness follows the actual Engine destructor, including pending
        // hierarchy propagation and persisted near data, not merely worker exit.
        assert!(storage::load(&dir, pos::encode(6, 0, 0), 1).is_some());
        // One visited chunk covers only part of the coarser hierarchy. Its
        // immediate parent is valid; forcing a root-level record would turn
        // incomplete children into invented terrain.
        assert!(storage::load(&dir, pos::encode(7, 0, 0), 1).is_some());
        assert!(retiring.finish());
        let target = dir.canonicalize().unwrap();
        let temporary_root = std::env::temp_dir().canonicalize().unwrap();
        assert!(target.starts_with(&temporary_root) && target != temporary_root);
        std::fs::remove_dir_all(target).unwrap();
    }

    struct DropAction(Option<Box<dyn FnOnce() + Send>>);
    impl Drop for DropAction {
        fn drop(&mut self) {
            if let Some(action) = self.0.take() { action(); }
        }
    }

    #[test]
    fn retirement_is_bounded_and_final_shutdown_drains_the_accepted_session() {
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc};
        use std::time::Duration;
        let (entered, entry) = mpsc::channel();
        let (release, waiting) = mpsc::channel();
        let finished = Arc::new(AtomicBool::new(false));
        let saved = finished.clone();
        let first = DropAction(Some(Box::new(move || {
            entered.send(()).unwrap();
            waiting.recv_timeout(Duration::from_secs(5)).unwrap();
            saved.store(true, Ordering::Release);
        })));
        let rejected_dropped = Arc::new(AtomicBool::new(false));
        let rejected_flag = rejected_dropped.clone();
        let second = DropAction(Some(Box::new(move || { rejected_flag.store(true, Ordering::Release); })));
        let mut retiring = engine::Retirement::new().unwrap();
        assert!(retiring.try_retire(first).is_ok());
        entry.recv_timeout(Duration::from_secs(5)).unwrap();
        let rejected = match retiring.try_retire(second) {
            Err(value) => value,
            Ok(()) => panic!("retirement accepted a second outstanding session"),
        };
        assert!(!rejected_dropped.load(Ordering::Acquire));
        let (closed, close) = mpsc::channel();
        let shutdown = std::thread::spawn(move || {
            assert!(retiring.finish());
            assert!(!retiring.ready());
            closed.send(()).unwrap();
        });
        assert!(matches!(close.recv_timeout(Duration::from_millis(30)), Err(mpsc::RecvTimeoutError::Timeout)));
        release.send(()).unwrap();
        close.recv_timeout(Duration::from_secs(5)).unwrap();
        shutdown.join().unwrap();
        assert!(finished.load(Ordering::Acquire), "shutdown returned before the accepted save");
        drop(rejected);
        assert!(rejected_dropped.load(Ordering::Acquire));
    }

    #[test]
    fn a_retirement_panic_resets_busy_but_blocks_reopening_an_uncertain_cache() {
        use std::time::{Duration, Instant};
        let mut retiring = engine::Retirement::new().unwrap();
        assert!(retiring.try_retire(DropAction(Some(Box::new(|| panic!("fixture save failure"))))).is_ok());
        let start = Instant::now();
        while retiring.busy() {
            assert!(start.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(retiring.failed());
        assert!(!retiring.ready());
        assert!(retiring.try_retire(DropAction(None)).is_err());
        assert!(!retiring.finish());
        assert!(!retiring.busy());
    }

    #[test]
    fn the_engine_generates_and_meshes_every_wanted_section() {
        let settings = engine::WorkSettings {
            render: RenderSettings {
                cave_culling: true,
                cave_culling_height: 60,
                min_y: -64,
            },
            generation: true,
            debug: false,
        };
        let dir = std::env::temp_dir().join(format!("dh-engine-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let plan = DetailPlan::new(HorizontalQuality::Low, 64);
        let mut wanted = 0;
        {
            let mut e = engine::Engine::new(FakeHost { blocks: blocks() }, Some(dir.clone()), 1, settings, 2);
            // As a world loading does: hundreds of chunks at once, which
            // must not starve the sections in view.
            for x in -12..=12 {
                for z in -12..=12 {
                    e.add_chunk(Box::new(FlatChunk {
                        x,
                        z,
                        height: |_, _| 70,
                    }));
                }
            }
            let start = std::time::Instant::now();
            let mut meshed = std::collections::HashSet::new();
            while start.elapsed() < std::time::Duration::from_secs(60) {
                let (meshes, _) = e.update([8.0, 8.0], plan, engine::Near::default());
                // A neighbour of the chunk may mesh blank before it is
                // generated; its generation meshes it again.
                for m in meshes {
                    if m.mesh.unwrap_or(0) > 0 {
                        meshed.insert(m.section);
                    } else {
                        meshed.remove(&m.section);
                    }
                }
                wanted = e.stats().wanted;
                if wanted > 0 && meshed.len() == wanted && e.stats().chunks == 625 && e.stats().queued == 0 {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            assert!(wanted > 0);
            assert_eq!(meshed.len(), wanted);
            assert_eq!(e.stats().chunks, 625);
            assert_eq!(e.stats().queued, 0);
        }
        // Dropping the engine saved every section it made.
        let files = |d: &str| std::fs::read_dir(dir.join(d)).map(|d| d.count()).unwrap_or(0);
        assert!(files("6") > 0);
        // The chunks reached the coarsest level.
        assert!(files("14") > 0);
        let _ = std::fs::remove_dir_all(dir);
    }
}
