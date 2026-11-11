// Part of Distant Horizons for Horizon Client, LGPL-3.0-only (see runtime.rs).
fn aquatic_generated(
    chunk: &horizon_world::worldgen::generator::ProtoChunk,
    biome_map: &[u16],
    dimension: &str,
) -> std::ops::ControlFlow<Arc<[Column; 256]>> {
    use crate::lod::distant_horizons_aquatic_columns as aquatic;
    let Some(_guard) = aquatic::enter(0) else {
        return std::ops::ControlFlow::Continue(());
    };
    let mut value = crate::vanillagen::lod::columns(chunk, biome_map, dimension);
    let registry = chunk.data().registry();
    let Some(water) = registry.default_state("minecraft:water") else {
        return std::ops::ControlFlow::Break(value);
    };
    for index in 0..256 {
        let (x, z) = ((index & 15) as i32, (index >> 4) as i32);
        let changes = aquatic::changes(&value[index], water, |y| {
            let state = chunk.block(x, y, z);
            (state != water && registry.state(state).is_some_and(|s| aquatic::is_aquatic(&s.name))).then_some(state)
        });
        if !changes.is_empty() {
            aquatic::restore(&mut std::sync::Arc::make_mut(&mut value)[index], water, &changes);
        }
    }
    std::ops::ControlFlow::Break(value)
}
