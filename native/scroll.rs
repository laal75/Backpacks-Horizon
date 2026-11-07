// The mouse wheel over the backpack's rows (glue.rs has it all).
fn backpacks_core_scroll(
    game: &crate::game::Game<'_>,
    scroll: [f64; 2],
    canceled: &mut bool,
) {
    crate::game::backpacks::scroll(game, scroll[1], canceled);
}
