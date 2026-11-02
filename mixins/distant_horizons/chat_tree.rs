// dormyu adaptation for Horizon c9d474d: command trees are built per connection.
// Preserve the complete DH grammar and alias without touching removed Remote fields.
fn distant_horizons_chat_tree(
    game: &mut crate::game::Game<'_>,
    value: horizon_client::command::Dispatcher,
) -> horizon_client::command::Dispatcher {
    use crate::game::{distant_horizons as m, distant_horizons_bridge as b};
    if game.native_on(m::ID) {
        b::with_commands(&value).unwrap_or(value)
    } else {
        value
    }
}
