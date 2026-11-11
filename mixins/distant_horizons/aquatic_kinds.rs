// Part of Distant Horizons for Horizon Client, LGPL-3.0-only (see runtime.rs).
fn aquatic_kinds(facts: &crate::blocks::BlockFacts) -> std::ops::ControlFlow<Vec<u8>> {
    let Some(_guard) = crate::lod::distant_horizons_aquatic_columns::enter(2) else {
        return std::ops::ControlFlow::Continue(());
    };
    let mut value = crate::lod::state_kinds(facts);
    crate::lod::distant_horizons_aquatic_columns::retain_kinds(facts, &mut value);
    std::ops::ControlFlow::Break(value)
}
