// The native plant model coexists with the continuous far-water volume.
// In the block-scale handoff ring, dropping this cutout exposes a pale bare
// seabed exactly at the near mask. No solid plant boxes or extra fluid faces.
use crate::game::distant_horizons as dh;
use horizon_client::{mesher::{SectionMesh, TerrainVertex, distant_horizons_aquatic_lighting::Probe}, streaming::WorkerContext};
use horizon_core::position::BlockPos;
use horizon_world::registry::BlockStateId;

pub fn is_plant(name: &str) -> bool {
    matches!(name.strip_prefix("minecraft:").unwrap_or(name),
        "kelp" | "kelp_plant" | "seagrass" | "tall_seagrass")
}

/// Only full-block columns carry exact plant positions. The existing Medium+
/// planner keeps these around the real near boundary regardless of distance.
/// Coarse cells must never inflate a sampled plant into a cell-sized shrub.
pub fn append(mesh: &mut SectionMesh, source: &dh::FullDataSource,
    context: &WorkerContext, blocks: &dh::BlockTable, min_y: i32,
    neighbors: [Option<&dh::FullDataSource>; 8], cutout: &dyn Fn(i32, i32) -> bool)
{
    if dh::pos::data_width(source.pos) != 1 { return; }
    let plants: Vec<bool> = source.mapping.entries().iter().map(|&(_, id)| {
        context.assets.registry.state(BlockStateId(id)).is_some_and(|s| is_plant(&s.name))
    }).collect();
    if !plants.iter().any(|&v| v) { return; }
    let bx = dh::pos::min_corner_block_x(source.pos);
    let bz = dh::pos::min_corner_block_z(source.pos);
    let mut models = Vec::new();
    let mut probe = Probe::default();
    let sample = |x: i32, y: i32, z: i32| {
        let nx = x.div_euclid(dh::pos::WIDTH);
        let nz = z.div_euclid(dh::pos::WIDTH);
        let neighbor = match (nx, nz) {
            (0, -1) => neighbors[0], (0, 1) => neighbors[1],
            (-1, 0) => neighbors[2], (1, 0) => neighbors[3],
            (-1, -1) => neighbors[4], (1, -1) => neighbors[5],
            (-1, 1) => neighbors[6], (1, 1) => neighbors[7], _ => Some(source),
        };
        let (data, x, z) = match neighbor.filter(|n| dh::pos::data_width(n.pos) == 1) {
            Some(n) => (n, x.rem_euclid(dh::pos::WIDTH), z.rem_euclid(dh::pos::WIDTH)),
            None => (source, x.clamp(0, dh::pos::WIDTH - 1), z.clamp(0, dh::pos::WIDTH - 1)),
        };
        for &p in data.column(x, z) {
            let top = dh::full_point::top_y(p) + min_y;
            if y < dh::full_point::bottom_y(p) + min_y || y >= top { continue; }
            let state = data.mapping.state(dh::full_point::id(p));
            let info = blocks.get(state);
            let above = dh::full_point::sky_light(p);
            let sky = if info.material == dh::material::WATER {
                above.saturating_sub((top - y).min(15) as u8)
            } else if info.opacity == 16 { 0 } else { above };
            return (state, sky << 4 | dh::full_point::block_light(p));
        }
        (0, if y >= min_y { 0xf0 } else { 0 })
    };
    for x in 0..dh::pos::WIDTH { for z in 0..dh::pos::WIDTH {
        if cutout(x, z) { continue; }
        for &p in source.column(x, z) {
            let id = dh::full_point::id(p);
            if !plants.get(id as usize).copied().unwrap_or(false) { continue; }
            let state = BlockStateId(source.mapping.state(id));
            let lo = dh::full_point::bottom_y(p) + min_y;
            let hi = dh::full_point::top_y(p) + min_y;
            for y in lo..hi {
                let world = BlockPos::new(bx + x, y, bz + z);
                models.clear();
                if context.render.model_catalog.models_at(state, world, &mut models).is_err() { continue; }
                let offset = context.tables.offset(state, world.x, world.z).unwrap_or([0.; 3]);
                let origin = [x as f32 + offset[0] as f32, y as f32 + offset[1] as f32,
                    z as f32 + offset[2] as f32];
                probe.fill(|d| sample(x + d[0], y + d[1], z + d[2]));
                for model in &models { for face in &model.faces {
                    let q = &face.quad;
                    let (gray, light) = probe.face(&context.tables, state, q, model.ambient_occlusion);
                    // The native block-colour table gives these four plants
                    // no biome tint; their green is already in the sprite.
                    let rgb = q.color.map_or(0xFF_FFFF, |c| c as u32);
                    let out = if face.translucent { &mut mesh.translucent } else { &mut mesh.solid };
                    for i in 0..4 {
                        let channel = |s: u32| u32::from(gray[i]) * ((rgb >> s) & 255) / 255;
                        let color = channel(16) | channel(8) << 8 | channel(0) << 16 | 0xFF00_0000;
                        let pos = std::array::from_fn(|k| q.positions[i][k] + origin[k]);
                        // LOD instances use a 1/32 UV scale, unlike native
                        // chunk instances. The same model UV must be tiled.
                        out.push(TerrainVertex::tiled(pos, q.uv[i], face.layer as u16, color, light[i]));
                    }
                }}
            }
        }
    }}
}
