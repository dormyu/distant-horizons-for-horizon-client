// Reuse the native model lighter for small far-water cutout models. The
// scratch pad is allocated once per mesh job, never once per face/frame.
use super::{MeshTables, Padded, lighting, pi};
use horizon_assets::bake::ModelQuad;
use horizon_world::registry::BlockStateId;

pub struct Probe { pad: Padded }
impl Default for Probe {
    fn default() -> Self {
        Self { pad: Padded { blocks: vec![0; super::P * super::P * super::P],
            light: vec![0; super::P * super::P * super::P] } }
    }
}
impl Probe {
    pub fn fill(&mut self, sample: impl Fn([i32; 3]) -> (u32, u8)) {
        // Native AO may read two steps along a normal. Its origin is safely
        // inside the regular pad; only this 5-cube can be addressed below.
        for y in -2..=2 { for z in -2..=2 { for x in -2..=2 {
            let (state, light) = sample([x, y, z]);
            let i = pi(8 + x, 8 + y, 8 + z);
            self.pad.blocks[i] = state;
            self.pad.light[i] = light;
        }}}
    }
    pub fn face(&self, tables: &MeshTables, state: BlockStateId, q: &ModelQuad,
        model_ao: bool) -> ([u8; 4], [u16; 4])
    {
        let own = &tables.info[state.0 as usize];
        let ao = q.ambient_occlusion.unwrap_or(model_ao) && own.emission == 0;
        let directional = if q.shade {
            lighting::cardinal(false, q.shade_direction_override.unwrap_or(q.facing))
        } else { 1. };
        let (gray, coords) = if ao {
            let (brightness, coords) = lighting::ambient_occlusion(
                &self.pad, &tables.info, own, [8; 3], &q.positions, q.facing);
            (brightness.map(|b| lighting::scale(lighting::gray(b), directional) as u8), coords)
        } else {
            let coords = match q.cull {
                Some(d) => lighting::coords(&self.pad, own, lighting::step([8; 3], d)),
                None => lighting::flat(&self.pad, own, [8; 3], &q.positions, q.facing),
            };
            ([lighting::gray(directional) as u8; 4], [coords; 4])
        };
        (gray, coords.map(|c| super::pack_coords(lighting::with_min(c, q.light))))
    }
}
