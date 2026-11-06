// A backpack's tooltip: how full it is (glue.rs).
fn backpacks_core_tooltip(
    stack: &horizon_client::items::ItemStack,
    lines: &mut Vec<(horizon_client::text::Text, u32)>,
) {
    crate::game::backpacks::tooltip(stack, lines);
}
