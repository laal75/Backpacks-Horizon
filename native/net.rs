// On the server: a player's click on their backpack's rows, sent by click.rs (glue.rs).
fn horizon_backpacks_core_net(
    game: &mut crate::game::Game<'_>,
    player: u128,
    channel: &str,
    data: &[u8],
) {
    crate::game::horizon_backpacks::net(game, player, channel, data);
}
