// Tree selection adapted from Horizon f42ef42 vanillagen/lod/vegetation.rs.
// Copyright (c) Ludovic Marie and Horizon contributors. MIT; see third-party/
// horizon-LICENSE. This hook observes the same predictor without altering it.
fn capture_vegetation(
    vegetation: &crate::vanillagen::lod::vegetation::Vegetation,
    column: &crate::lod::Column,
    x: i32,
    z: i32,
    width: i32,
    light: u16,
) {
    use crate::distant_horizons_stem_capture::{self as capture, Candidate};
    if !capture::enabled() || !(1..16).contains(&width) {
        return;
    }
    let Some(surface) = column.surface else { return };
    if surface.water.is_some_and(|(y, _)| y >= surface.height) {
        return;
    }
    let ground = vegetation.registry.state(surface.state).map_or("", |s| s.name.as_str());
    if !matches!(ground, "minecraft:grass_block" | "minecraft:dirt" | "minecraft:coarse_dirt"
        | "minecraft:podzol" | "minecraft:mycelium" | "minecraft:rooted_dirt" | "minecraft:moss_block") {
        return;
    }
    let Some(trees) = vegetation.trees.get(&surface.biome) else { return };
    let total: f64 = trees.iter().map(|tree| tree.count).sum();
    let choose = |n: u64| {
        let mut value = (n as u32 as f64 / u32::MAX as f64) * total;
        for tree in trees {
            value -= tree.count;
            if value <= 0. { return tree; }
        }
        &trees[trees.len() - 1]
    };
    let radius = trees.iter().map(|t| t.radius).max().unwrap_or(2);
    let mut best: Option<(i32, Candidate)> = None;
    for cz in (z - radius).div_euclid(4)..=(z + radius).div_euclid(4) {
        for cx in (x - radius).div_euclid(4)..=(x + radius).div_euclid(4) {
            let n = crate::vanillagen::lod::vegetation::hash(vegetation.seed, cx, cz);
            if (n as u32 as f64 / u32::MAX as f64) >= (total / 16.).min(1.) { continue; }
            let tree = choose(n.rotate_left(21));
            let tx = cx * 4 + ((n >> 32) & 3) as i32;
            let tz = cz * 4 + ((n >> 34) & 3) as i32;
            let d2 = (x - tx).pow(2) + (z - tz).pow(2);
            if d2 > tree.radius.pow(2) { continue; }
            let height = tree.height + ((n >> 40) % (tree.variation.max(0) + 1) as u64) as i32;
            let cap = ((tree.radius.pow(2) - d2) as f32).sqrt().ceil() as i32;
            let top = surface.height + height + cap.min(2);
            let bottom = (surface.height + height - tree.radius).max(surface.height + 1);
            if best.as_ref().is_none_or(|(y, _)| top > *y) {
                best = Some((top, Candidate {
                    x: tx, z: tz, min: surface.height, max: bottom,
                    crown_max: top.max(bottom + 1), log: tree.log.0, leaves: tree.leaves.0,
                    biome: surface.biome, sky: ((light >> 4) & 15) as u8,
                    block: ((light >> 12) & 15) as u8, exact: false,
                }));
            }
        }
    }
    if let Some((_, candidate)) = best { capture::record(candidate); }
}
