// A mouse button over the screen: the backpack's rows take their own clicks (glue.rs).
fn horizon_backpacks_core_click(
    game: &mut crate::game::Game<'_>,
    button: i32,
    pressed: bool,
    canceled: &mut bool,
) {
    crate::game::horizon_backpacks::mouse(game, button, pressed, canceled);
}
