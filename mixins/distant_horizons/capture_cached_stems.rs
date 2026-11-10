// Keep exact native log footprints before the preview's majority reduction.
// Part of Distant Horizons for Horizon Client, LGPL-3.0-only (see runtime.rs).
fn capture_cached_stems(
    sampler: &crate::vanillagen::lod::VanillaLodSampler,
    x: i32,
    z: i32,
    width: i32,
    side: usize,
) {
    use crate::distant_horizons_stem_capture::{self as capture, Candidate};
    if !capture::enabled() || !(1..=16).contains(&width) || side > 16 { return; }
    let registry = sampler.generator.generator.data().registry();
    let extent = width * side as i32;
    for cz in z.div_euclid(16)..=(z + extent - 1).div_euclid(16) {
        for cx in x.div_euclid(16)..=(x + extent - 1).div_euclid(16) {
            let Some(columns) = sampler.cache.get(&(cx, cz)) else { continue };
            for dz in 0..16 {
                for dx in 0..16 {
                    let (wx, wz) = (cx * 16 + dx, cz * 16 + dz);
                    if wx < x || wx >= x + extent || wz < z || wz >= z + extent { continue; }
                    for span in &columns[(dz * 16 + dx) as usize].spans {
                        if !registry.state(span.state).is_some_and(|s| capture::is_stem(&s.name)) { continue; }
                        capture::record(Candidate {
                            x: wx, z: wz, min: i32::from(span.min), max: i32::from(span.max),
                            crown_max: i32::from(span.max), log: span.state.0, leaves: 0,
                            biome: span.biome, sky: ((span.light >> 4) & 15) as u8,
                            block: ((span.light >> 12) & 15) as u8, exact: true,
                        });
                    }
                }
            }
        }
    }
}
