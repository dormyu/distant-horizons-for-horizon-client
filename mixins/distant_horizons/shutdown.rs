// Close the thread-local DH engine before process/thread-local destruction.
// Part of the dormyu adaptation, LGPL-3.0-only, retaining upstream notices.
fn distant_horizons_shutdown(game: &mut crate::game::Game<'_>) {
    crate::game::distant_horizons_bridge::shutdown(game);
}
