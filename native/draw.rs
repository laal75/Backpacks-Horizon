// Over the open screen: the worn backpack's rows (glue.rs).
fn horizon_backpacks_core_draw(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
) {
    crate::game::horizon_backpacks::draw(game, batch);
}
