// Horizon Backpacks, inside the game (crate::game::horizon_backpacks): a backpack worn in the
// chest slot adds rows of slots to the player's own inventory screen, beside it on the left, and
// to the content mod's "Backpacks" creative tab, in place of its grid. The rows are the backpack
// item's own: its custom data keeps them (`backpack_items`), so they go wherever it goes.
//
// The trait API can't reach the cursor stack or the screen's slots, so this is mixin code. The
// client draws the rows from its copy of the worn backpack (draw.rs) and turns a click on them
// into a mod message (click.rs); the server, single player's integrated one included, applies it
// to the player's inventory, cursor and backpack and sends the inventory back (net.rs).
use horizon_client::creative::Creative;
use horizon_client::hud::{HudBatch, rgba};
use horizon_client::items::ItemStack;
use horizon_client::mod_api::Host;
use horizon_world::nbt::Tag;

use super::{Game, Screen};

pub(super) const MOD: &str = "horizon_backpacks_core";
/// The mod message a click on the rows becomes.
const CHANNEL: &str = "horizon_backpacks:click";
/// The content mod's creative tab.
const TAB: &str = "horizon_backpacks:backpacks";
/// The custom data key the rows are kept under.
const KEY: &str = "backpack_items";
/// The armour slot a backpack is worn in: the chest (0 is the head).
const CHEST: usize = 1;

/// Each backpack, its title and its rows of 9 (tools/gen_content.py has the same).
const TIERS: [(&str, &str, usize); 5] = [
    ("horizon_backpacks:leather_backpack", "Leather Backpack", 1),
    ("horizon_backpacks:iron_backpack", "Iron Backpack", 2),
    ("horizon_backpacks:gold_backpack", "Gold Backpack", 3),
    ("horizon_backpacks:diamond_backpack", "Diamond Backpack", 4),
    ("horizon_backpacks:obsidian_backpack", "Obsidian Backpack", 5),
];

fn rows_of(item: &str) -> Option<usize> {
    TIERS.iter().find(|(id, ..)| *id == item).map(|&(.., rows)| rows)
}

fn title_of(item: &str) -> &'static str {
    TIERS.iter().find(|(id, ..)| *id == item).map_or("Backpack", |&(_, title, _)| title)
}

/// What a backpack item holds, slot by slot.
fn contents(stack: &ItemStack, rows: usize) -> Vec<Option<ItemStack>> {
    let tag = stack
        .data()
        .and_then(|d| d.other("minecraft:custom_data"))
        .and_then(|data| data.get(KEY));
    horizon_client::save::decode_items(tag, rows * 9)
}

fn store(stack: &mut ItemStack, items: &[Option<ItemStack>]) {
    let mut data = match stack.data().and_then(|d| d.other("minecraft:custom_data")) {
        Some(Tag::Compound(c)) => c.clone(),
        _ => Default::default(),
    };
    data.insert(KEY.into(), horizon_client::save::encode_items(items));
    stack.data_mut().set_other("minecraft:custom_data", Some(Tag::Compound(data)));
}

/// A backpack that holds something, which never goes in another backpack (an empty one does).
fn full_backpack(stack: &ItemStack) -> bool {
    rows_of(&stack.item).is_some_and(|rows| contents(stack, rows).iter().any(Option::is_some))
}

fn same_item(a: &ItemStack, b: &ItemStack) -> bool {
    a.item == b.item && a.damage == b.damage && a.extra == b.extra
}

// ---- the client: where the rows are, drawing them, and clicks ---------------------------------

/// What a click asks of the server: the message's first byte, then a slot.
const PICK: u8 = 0;
const PICK_RIGHT: u8 = 1;
/// A backpack slot to the inventory.
const SHIFT_OUT: u8 = 2;
/// An inventory slot (0..36, the hotbar first) to the backpack.
const SHIFT_IN: u8 = 3;

/// Where the worn backpack's slots are on the open screen.
struct Layout {
    rows: usize,
    /// Slots to a row on screen: 9, or fewer where the window is too narrow for 9 beside the
    /// inventory. The slots keep their order; only how they wrap changes.
    cols: usize,
    /// The first slot's 16 x 16 item box; the rest follow, 18 apart.
    first: (f32, f32),
    /// The panel drawn around them, on the inventory screen (the creative tab has its own).
    panel: Option<[f32; 4]>,
}

impl Layout {
    fn slot(&self, i: usize) -> (f32, f32) {
        (self.first.0 + (i % self.cols) as f32 * 18.0, self.first.1 + (i / self.cols) as f32 * 18.0)
    }

    fn slot_at(&self, x: f32, y: f32) -> Option<usize> {
        (0..self.rows * 9).find(|&i| {
            let (sx, sy) = self.slot(i);
            x >= sx - 1.0 && x < sx + 17.0 && y >= sy - 1.0 && y < sy + 17.0
        })
    }

    fn covers(&self, x: f32, y: f32) -> bool {
        self.panel.is_some_and(|[px, py, w, h]| x >= px && x < px + w && y >= py && y < py + h)
            || self.slot_at(x, y).is_some()
    }
}

/// The player's own inventory screen: not a container's, a station's or a mount's.
fn own_inventory(game: &Game<'_>) -> bool {
    game.screen == Screen::Inventory
        && game.container.is_none()
        && game.station_slots().is_none()
        && game.mount_slots().is_none()
        && game.native_screen_slots().is_none()
        && game.craft_size == 2
}

/// The worn backpack's rows, on a screen that shows them: none on a server without this mod
/// (it couldn't apply the clicks).
fn layout(game: &Game<'_>, batch: &HudBatch) -> Option<Layout> {
    if !game.native_on(MOD) || game.remote.as_ref().is_some_and(|r| !r.mod_net) {
        return None;
    }
    let rows = rows_of(&game.armor[CHEST].as_ref()?.item)?;
    if own_inventory(game) {
        // Hidden while the recipe book is open: it takes the same side.
        if game.recipe_book_shift(batch.width) > 0.0 {
            return None;
        }
        // On the left, as many columns as the room there takes (9 at most, 2 px from the edge).
        let (ox, oy) = game.inventory_origin(batch);
        let room = ((ox - 6.0 - 14.0) / 18.0).floor().max(1.0) as usize;
        let cols = room.min(9);
        let shown_rows = (rows * 9).div_ceil(cols);
        let (w, h) = (cols as f32 * 18.0 + 14.0, 24.0 + shown_rows as f32 * 18.0);
        let panel = [ox - w - 4.0, oy, w, h];
        let first = (panel[0] + 8.0, oy + 18.0);
        return Some(Layout { rows, cols, first, panel: Some(panel) });
    }
    if game.creative_tab_open() && game.creative_screen.tab().id == TAB {
        let (ox, oy) = Game::creative_origin(batch);
        let (x, y) = Creative::tab_slot_position(Creative::DISPLAY)?;
        return Some(Layout { rows, cols: 9, first: (ox + x, oy + y), panel: None });
    }
    None
}

fn batch(game: &Game<'_>) -> Option<HudBatch> {
    let r = game.renderer.as_ref()?;
    Some(HudBatch::new([r.config.width, r.config.height]))
}

/// The mouse hook (click.rs): a press or release over the rows is the backpack's and the game
/// never sees it; a shift-click on an inventory slot sends that stack into the backpack.
pub(super) fn mouse(game: &mut Game<'_>, button: i32, pressed: bool, canceled: &mut bool) {
    let Some(batch) = batch(game) else {
        return;
    };
    let Some(layout) = layout(game, &batch) else {
        return;
    };
    let (x, y) = game.cursor;
    if !layout.covers(x, y) {
        let shift_in = pressed && button == 0 && game.has_shift_down() && game.carried.is_none();
        if shift_in
            && own_inventory(game)
            && let Some(slot) = game.slot_at(&batch, x, y)
            && slot < 36
            && game.inventory[slot].as_ref().is_some_and(|s| !full_backpack(s))
        {
            send(game, SHIFT_IN, slot);
            *canceled = true;
        }
        return;
    }
    *canceled = true;
    if !pressed {
        return;
    }
    let Some(i) = layout.slot_at(x, y) else {
        return;
    };
    let action = match (button, game.has_shift_down()) {
        (0, true) => SHIFT_OUT,
        (0, false) => PICK,
        (1, _) => PICK_RIGHT,
        _ => return,
    };
    send(game, action, i);
}

fn send(game: &mut Game<'_>, action: u8, slot: usize) {
    if let Err(e) = Host::net_send_to_server(game, CHANNEL, &[action, slot as u8]) {
        log::warn!("Horizon Backpacks: {e}");
    }
}

/// The screen hook (draw.rs): the panel, its slots and items, the hovered slot's highlight, and
/// the cursor stack again over the panel (the game drew it before the panel).
pub(super) fn draw(game: &Game<'_>, batch: &mut HudBatch) {
    let Some(layout) = layout(game, batch) else {
        return;
    };
    let Some(pack) = game.armor[CHEST].as_ref() else {
        return;
    };
    let items = contents(pack, layout.rows);
    let atlas = &game.gui.atlas;
    if let Some([px, py, w, h]) = layout.panel {
        batch.fill(atlas, px, py, w, h, rgba(0, 0, 0, 255));
        batch.fill(atlas, px + 1.0, py + 1.0, w - 2.0, h - 2.0, rgba(198, 198, 198, 255));
        batch.text(atlas, title_of(&pack.item), px + 8.0, py + 6.0, rgba(64, 64, 64, 255), false);
    }
    for i in 0..layout.rows * 9 {
        let (x, y) = layout.slot(i);
        let (fx, fy) = (x - 1.0, y - 1.0);
        batch.fill(atlas, fx, fy, 18.0, 18.0, rgba(55, 55, 55, 255));
        batch.fill(atlas, fx + 1.0, fy + 1.0, 17.0, 17.0, rgba(255, 255, 255, 255));
        batch.fill(atlas, fx + 1.0, fy + 1.0, 16.0, 16.0, rgba(139, 139, 139, 255));
    }
    for (i, stack) in items.iter().enumerate() {
        if let Some(stack) = stack {
            let (x, y) = layout.slot(i);
            game.draw_item(batch, stack, x, y);
        }
    }
    let (cx, cy) = game.cursor;
    if let Some(i) = layout.slot_at(cx, cy) {
        let (x, y) = layout.slot(i);
        batch.fill(&game.gui.atlas, x, y, 16.0, 16.0, rgba(255, 255, 255, 128));
    }
    if layout.covers(cx, cy)
        && let Some(carried) = &game.carried
    {
        game.draw_item(batch, carried, cx - 8.0, cy - 8.0);
    }
}

/// The tooltip hook (tooltip.rs): how full a backpack is.
pub(super) fn tooltip(stack: &ItemStack, lines: &mut Vec<(horizon_client::mod_api::Text, u32)>) {
    let Some(rows) = rows_of(&stack.item) else {
        return;
    };
    let used = contents(stack, rows).iter().flatten().count();
    let line = format!("{used} / {} slots used", rows * 9);
    lines.push((horizon_client::mod_api::Text::literal(line), rgba(170, 170, 170, 255)));
}

// ---- the server: applying a click ------------------------------------------------------------

/// The mod message hook (net.rs): a player's click on their backpack's rows, applied to their
/// inventory, cursor and worn backpack, then their inventory sent back whole.
pub(super) fn net(game: &mut Game<'_>, player: u128, channel: &str, data: &[u8]) {
    let (Some(&action), Some(&slot), CHANNEL) = (data.first(), data.get(1), channel) else {
        return;
    };
    let Some(mut host) = game.lan.take() else {
        return;
    };
    if let Some(gi) = host.hub.guests.iter().position(|g| g.uuid == player) {
        let g = &mut host.hub.guests[gi];
        let changed = apply(game, &mut g.inventory, &mut g.armor[CHEST], &mut g.carried, action, slot as usize);
        if changed {
            host.hub.send_inventory(gi);
        }
    }
    game.lan = Some(host);
}

/// One click, on the player's `inventory` (36 slots, the hotbar first), worn `chest` and
/// `carried` cursor stack. True when anything changed.
fn apply(
    game: &Game<'_>,
    inventory: &mut [Option<ItemStack>],
    chest: &mut Option<ItemStack>,
    carried: &mut Option<ItemStack>,
    action: u8,
    slot: usize,
) -> bool {
    let Some(pack) = chest.as_mut() else {
        return false;
    };
    let Some(rows) = rows_of(&pack.item) else {
        return false;
    };
    let max = |s: &ItemStack| Host::max_stack_size(game, &s.item).unwrap_or(64);
    let mut items = contents(pack, rows);
    let before = (items.clone(), carried.clone(), inventory.to_vec());
    match action {
        PICK | PICK_RIGHT if slot < items.len() => {
            click(&mut items[slot], carried, action == PICK_RIGHT, &max);
        }
        SHIFT_OUT if slot < items.len() => {
            if let Some(stack) = items[slot].take() {
                items[slot] = insert(inventory, stack, &max, (9..36).chain(0..9));
            }
        }
        SHIFT_IN if slot < inventory.len() => {
            if let Some(stack) = inventory[slot].take() {
                inventory[slot] = if full_backpack(&stack) {
                    Some(stack)
                } else {
                    insert(&mut items, stack, &max, 0..rows * 9)
                };
            }
        }
        _ => return false,
    }
    if (items.clone(), carried.clone(), inventory.to_vec()) == before {
        return false;
    }
    store(pack, &items);
    true
}

/// A left (or `right`) click on a backpack slot with the cursor stack, as a chest's slot takes
/// it: pick up all (half), put down all (one), top up, or swap. A backpack holding anything
/// never goes in.
fn click(
    slot: &mut Option<ItemStack>,
    carried: &mut Option<ItemStack>,
    right: bool,
    max: &impl Fn(&ItemStack) -> u32,
) {
    match (carried.take(), slot.take()) {
        (None, None) => {}
        (None, Some(mut held)) => {
            if right && held.count > 1 {
                let mut taken = held.clone();
                taken.count = held.count.div_ceil(2);
                held.count -= taken.count;
                *carried = Some(taken);
                *slot = Some(held);
            } else {
                *carried = Some(held);
            }
        }
        (Some(c), held) if full_backpack(&c) => {
            *carried = Some(c);
            *slot = held;
        }
        (Some(mut c), None) => {
            if right {
                let mut one = c.clone();
                one.count = 1;
                c.count -= 1;
                *slot = Some(one);
                *carried = (c.count > 0).then_some(c);
            } else {
                *slot = Some(c);
            }
        }
        (Some(mut c), Some(mut held)) if same_item(&c, &held) => {
            let room = max(&held).saturating_sub(held.count);
            let moved = if right { room.min(1) } else { room.min(c.count) };
            held.count += moved;
            c.count -= moved;
            *slot = Some(held);
            *carried = (c.count > 0).then_some(c);
        }
        (Some(c), Some(held)) => {
            *slot = Some(c);
            *carried = Some(held);
        }
    }
}

/// Puts `stack` into `slots` in `order` as a shift-click does: onto equal stacks, then the first
/// empty slot. What didn't fit.
fn insert(
    slots: &mut [Option<ItemStack>],
    mut stack: ItemStack,
    max: &impl Fn(&ItemStack) -> u32,
    order: impl Iterator<Item = usize> + Clone,
) -> Option<ItemStack> {
    let limit = max(&stack);
    for i in order.clone() {
        if let Some(s) = &mut slots[i]
            && same_item(s, &stack)
        {
            let moved = limit.saturating_sub(s.count).min(stack.count);
            s.count += moved;
            stack.count -= moved;
            if stack.count == 0 {
                return None;
            }
        }
    }
    for i in order {
        if slots[i].is_none() {
            slots[i] = Some(stack);
            return None;
        }
    }
    Some(stack)
}
