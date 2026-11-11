// Part of Distant Horizons for Horizon Client, LGPL-3.0-only (see runtime.rs).
fn aquatic_cache_key(states: usize, biomes: usize) -> std::ops::ControlFlow<u64> {
    let Some(_guard) = crate::lod::distant_horizons_aquatic_columns::enter(3) else {
        return std::ops::ControlFlow::Continue(());
    };
    let value = crate::lod::cache::build_key(states, biomes);
    std::ops::ControlFlow::Break(crate::lod::distant_horizons_aquatic_columns::cache_key(value))
}
