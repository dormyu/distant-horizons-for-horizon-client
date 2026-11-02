fn distant_horizons_fog(game: &crate::game::Game<'_>, fog_type: horizon_client::fog::FogType, fog: &mut horizon_client::fog::FogData) {
    crate::game::distant_horizons_bridge::render_fog(game, fog_type, fog);
}
