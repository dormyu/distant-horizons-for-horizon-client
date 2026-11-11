// Part of Distant Horizons for Horizon Client, LGPL-3.0-only (see runtime.rs).
fn aquatic_summary_decode(tag: &horizon_world::nbt::Tag) -> std::ops::ControlFlow<Option<Columns>> {
    use crate::lod::distant_horizons_aquatic_columns as aquatic;
    let Some(_guard) = aquatic::enter(5) else {
        return std::ops::ControlFlow::Continue(());
    };
    if tag.get(aquatic::SUMMARY_KEY) != Some(&horizon_world::nbt::Tag::Int(aquatic::SUMMARY_REVISION)) {
        return std::ops::ControlFlow::Break(None);
    }
    std::ops::ControlFlow::Break(crate::server_terrain::decode(tag))
}
