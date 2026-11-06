// Over the open screen: the worn backpack's rows (glue.rs).
fn backpacks_core_draw(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
) {
    crate::game::backpacks::draw(game, batch);
}
