// Part of Distant Horizons for Horizon Client, LGPL-3.0-only (see runtime.rs).
fn aquatic_saved(
    root: &horizon_world::nbt::NamedTag,
    assets: &crate::assets::Assets,
    biomes: &crate::biome::Biomes,
    layout: crate::chunk::ColumnLayout,
    sky_light: bool,
) -> std::ops::ControlFlow<Option<LodSurfaces>> {
    use crate::lod::distant_horizons_aquatic_columns as aquatic;
    use horizon_world::{chunk::BlockState, nbt::Tag};
    let Some(_guard) = aquatic::enter(1) else {
        return std::ops::ControlFlow::Continue(());
    };
    let Some(mut value) = crate::save::decode_lod_surfaces(root, assets, biomes, layout, sky_light) else {
        return std::ops::ControlFlow::Break(None);
    };
    let Some(water) = assets.registry.default_state("minecraft:water") else {
        return std::ops::ControlFlow::Break(Some(value));
    };
    if !value.iter().any(|c| c.spans.iter().any(|s| s.state == water)) {
        return std::ops::ControlFlow::Break(Some(value));
    }
    let Some(Tag::List(sections)) = root.tag.get("sections") else {
        return std::ops::ControlFlow::Break(Some(value));
    };
    // Resolve only plant palettes once; no raw-volume allocation or NBT changes.
    let sections: Vec<_> = sections
        .iter()
        .filter_map(|section| {
            let section_y = crate::save::number(section.get("Y"))? as i32;
            let min = section_y.checked_mul(16)?;
            layout.section_index(min)?;
            let blocks = section.get("block_states")?;
            let Tag::List(palette) = blocks.get("palette")? else {
                return None;
            };
            let states: Vec<_> = palette
                .iter()
                .map(|entry| {
                    // f42's 26.3 descriptor uses id/properties (or a bare
                    // string), exactly as BlockState::from_nbt accepts.
                    let name = match entry {
                        Tag::String(name) => name,
                        _ => match entry.get("id")? {
                            Tag::String(name) => name,
                            _ => return None,
                        },
                    };
                    if !aquatic::is_aquatic(&name.to_string_lossy()) {
                        return None;
                    }
                    let descriptor = BlockState::from_nbt(entry).ok()?;
                    assets
                        .registry
                        .id(&descriptor)
                        .or_else(|| assets.registry.default_state(&descriptor.name))
                })
                .collect();
            states.iter().any(Option::is_some).then_some((min, blocks, states))
        })
        .collect();
    if sections.is_empty() {
        return std::ops::ControlFlow::Break(Some(value));
    }
    for cell in 0..256 {
        let changes = aquatic::changes(&value[cell], water, |y| {
            let (min, blocks, palette) = sections.iter().find(|(min, _, _)| (*min..*min + 16).contains(&y))?;
            let index = (y - min) as usize * 256 + cell;
            let index = crate::save::lod_palette_index(blocks, index, 4096, 4)?;
            palette.get(index).copied().flatten()
        });
        if !changes.is_empty() {
            aquatic::restore(&mut std::sync::Arc::make_mut(&mut value)[cell], water, &changes);
        }
    }
    std::ops::ControlFlow::Break(Some(value))
}
