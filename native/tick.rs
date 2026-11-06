// A worn Ender Backpack's rows, kept up to date (glue.rs has it all).
fn backpacks_core_tick(
    game: &mut crate::game::Game<'_>,
) {
    crate::game::backpacks::tick(game);
}
