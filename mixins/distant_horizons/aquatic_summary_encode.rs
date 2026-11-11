// Part of Distant Horizons for Horizon Client, LGPL-3.0-only (see runtime.rs).
fn aquatic_summary_encode(x: i32, z: i32, columns: &[crate::lod::Column; 256]) -> std::ops::ControlFlow<Tag> {
    use crate::lod::distant_horizons_aquatic_columns as aquatic;
    let Some(_guard) = aquatic::enter(4) else {
        return std::ops::ControlFlow::Continue(());
    };
    let mut value = crate::server_terrain::encode(x, z, columns);
    if let horizon_world::nbt::Tag::Compound(map) = &mut value {
        map.insert(
            aquatic::SUMMARY_KEY.into(),
            horizon_world::nbt::Tag::Int(aquatic::SUMMARY_REVISION),
        );
    }
    std::ops::ControlFlow::Break(value)
}
