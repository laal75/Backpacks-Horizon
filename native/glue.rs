// Backpacks, inside the game (crate::game::horizon_backpacks): a backpack worn in the
// chest slot adds rows of slots to the player's own inventory screen, beside it on the left, and
// to the content mod's "Backpacks" creative tab, in place of its grid. The rows are the backpack
// item's own: its custom data keeps them (`backpack_items`), so they go wherever it goes.
//
// The trait API can't reach the cursor stack or the screen's slots, so this is mixin code. The
// client draws the rows from its copy of the worn backpack (draw.rs) and turns a click on them
// into a mod message (click.rs); the server, single player's integrated one included, applies it
// to the player's inventory, cursor and backpack and sends the inventory back (net.rs).
use horizon_client::containers::BlockEntity;
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

/// Each backpack, its title and its slots, rows of 5 on the inventory screen
/// (tools/gen_content.py has the same). 45 at most: the creative tab's 9 by 5 grid.
const TIERS: [(&str, &str, usize); 5] = [
    ("horizon_backpacks:leather_backpack", "Leather Backpack", 10),
    ("horizon_backpacks:iron_backpack", "Iron Backpack", 20),
    ("horizon_backpacks:gold_backpack", "Gold Backpack", 30),
    ("horizon_backpacks:diamond_backpack", "Diamond Backpack", 40),
    ("horizon_backpacks:obsidian_backpack", "Obsidian Backpack", 45),
];
/// Slots to a row on the inventory screen.
const COLUMNS: usize = 5;

fn slots_of(item: &str) -> Option<usize> {
    TIERS.iter().find(|(id, ..)| *id == item).map(|&(.., slots)| slots)
}

fn title_of(item: &str) -> &'static str {
    TIERS.iter().find(|(id, ..)| *id == item).map_or("Backpack", |&(_, title, _)| title)
}

/// What a backpack item holds, slot by slot.
fn contents(stack: &ItemStack, slots: usize) -> Vec<Option<ItemStack>> {
    let tag = stack
        .data()
        .and_then(|d| d.other("minecraft:custom_data"))
        .and_then(|data| data.get(KEY));
    horizon_client::save::decode_items(tag, slots)
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
    slots_of(&stack.item).is_some_and(|slots| contents(stack, slots).iter().any(Option::is_some))
}

fn same_item(a: &ItemStack, b: &ItemStack) -> bool {
    a.item == b.item && a.damage == b.damage && a.extra == b.extra
}

// ---- the client: where the rows are, drawing them, and clicks ---------------------------------

/// What a click asks of the server: the message's first byte, then a slot.
const PICK: u8 = 0;
const PICK_RIGHT: u8 = 1;
/// A backpack slot to the inventory: the hotbar first, then the main rows.
const SHIFT_OUT: u8 = 2;
/// A main inventory slot (9..36) to the backpack. The hotbar's shift-clicks stay the game's
/// (to the main rows), so items go round: backpack → hotbar, hotbar → main rows, main rows →
/// backpack.
const SHIFT_IN: u8 = 3;
/// The backpack's own slots sorted, as Horizon's sort orders a chest.
const SORT: u8 = 4;
/// Horizon's sort button: a square this size.
const SORT_BUTTON: f32 = 9.0;

/// Where the worn backpack's slots are on the open screen.
struct Layout {
    slots: usize,
    /// Slots to a row: 5 beside the inventory, 9 in the creative tab's grid.
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
        (0..self.slots).find(|&i| {
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

/// This mod's half on the server answers its messages: not a server without it.
fn served(game: &Game<'_>) -> bool {
    game.native_on(MOD) && game.remote.as_ref().is_none_or(|r| r.mod_net)
}

/// The worn backpack's rows, on a screen that shows them: every screen with the player's
/// inventory in it (their own, a crafting table's, a chest's, a mod's) and the creative tab.
fn layout(game: &Game<'_>, batch: &HudBatch) -> Option<Layout> {
    if !served(game) {
        return None;
    }
    let slots = slots_of(&game.armor[CHEST].as_ref()?.item)?;
    if game.screen == Screen::Inventory {
        // Hidden while the recipe book is open: it takes the same side.
        if game.recipe_book_shift(batch.width) > 0.0 {
            return None;
        }
        // On the left, in full rows of 5: every tier's slots are a multiple of 5.
        let (ox, oy) = game.inventory_origin(batch);
        let rows = slots.div_ceil(COLUMNS);
        let (w, h) = (COLUMNS as f32 * 18.0 + 14.0, 24.0 + rows as f32 * 18.0);
        let panel = [ox - w - 4.0, oy, w, h];
        let first = (panel[0] + 8.0, oy + 18.0);
        return Some(Layout { slots, cols: COLUMNS, first, panel: Some(panel) });
    }
    if game.creative_tab_open() && game.creative_screen.tab().id == TAB {
        let (ox, oy) = Game::creative_origin(batch);
        let (x, y) = Creative::tab_slot_position(Creative::DISPLAY)?;
        return Some(Layout { slots, cols: 9, first: (ox + x, oy + y), panel: None });
    }
    None
}

/// What a sort button sorts: game slots (with clicks, as Horizon's sort does) or the worn
/// backpack (on the server).
enum Sorts {
    Slots(Vec<usize>),
    Backpack,
}

/// The open screen is a Expanded Chests chest's.
fn chest_screen(game: &Game<'_>) -> bool {
    let entity = game.container.and_then(|k| game.world.block_entities.get(&k));
    matches!(entity, Some(BlockEntity::Mod(m)) if m.id.starts_with("horizon_chests:") && m.id.ends_with("_chest"))
}

/// The sort buttons this mod adds, by their top left corner, with the sort extra on: the
/// backpack panel's own, and on a mod's screen (where Horizon's own sort buttons don't go) the
/// main inventory's, by its label as Horizon places it, and a Expanded Chests chest's, by its
/// title.
fn sort_buttons(game: &Game<'_>, batch: &HudBatch) -> Vec<((f32, f32), Sorts)> {
    let mut buttons = Vec::new();
    if !game.settings.extras.sort || game.screen != Screen::Inventory {
        return buttons;
    }
    if let Some(Layout { panel: Some([px, py, w, _]), .. }) = layout(game, batch) {
        buttons.push(((px + w - SORT_BUTTON - 7.0, py + 5.0), Sorts::Backpack));
    }
    if let (Some(container), Some(ix), Some(iy)) = (
        game.native_screen_slots(),
        game.native_screen_inventory_x(),
        game.native_screen_inventory_y(),
    ) {
        let (ox, oy) = game.inventory_origin(batch);
        let width = game.native_screen_width().unwrap_or(176.0);
        if chest_screen(game) {
            buttons.push(((ox + width - SORT_BUTTON - 7.0, oy + 5.0), Sorts::Slots(container)));
        }
        let main: Vec<usize> = (9..36).collect();
        buttons.push(((ox + ix + 9.0 * 18.0 - SORT_BUTTON - 1.0, oy + iy - 13.0), Sorts::Slots(main)));
    }
    buttons
}

fn on_button(at: (f32, f32), x: f32, y: f32) -> bool {
    x >= at.0 && x < at.0 + SORT_BUTTON && y >= at.1 && y < at.1 + SORT_BUTTON
}

/// Sorts game slots as Horizon's sort extra does: the clicks `inv_sort` plans, each an ordinary
/// left click (needs an empty hand), in the creative screen's order, then by name.
fn sort_slots(game: &mut Game<'_>, slots: &[usize]) {
    if game.carried.is_some() {
        return;
    }
    let views: Vec<Option<ItemStack>> = slots.iter().map(|&s| game.slot_view(s)).collect();
    let clicks = {
        let names = |id: &str| game.pack.names.display_item(id);
        let rank = |s: &ItemStack| {
            let order = game.creative_screen.order_of(s).unwrap_or(usize::MAX);
            (order, horizon_client::stations::hover_name(&names, s))
        };
        let max = |s: &ItemStack| game.max_stack(s);
        horizon_client::inv_sort::plan(&views, &rank, &max)
    };
    for i in clicks {
        game.tweak_click(slots[i], 0, horizon_client::menu::Input::Pickup);
    }
}

fn batch(game: &Game<'_>) -> Option<HudBatch> {
    let r = game.renderer.as_ref()?;
    Some(HudBatch::new([r.config.width, r.config.height]))
}

/// The mouse hook (click.rs): a press or release over the rows is the backpack's and the game
/// never sees it; on the player's own inventory screen, a shift-click on a main inventory slot
/// sends that stack into the backpack while it has room (when it hasn't, the game moves it to the
/// hotbar as usual).
pub(super) fn mouse(game: &mut Game<'_>, button: i32, pressed: bool, canceled: &mut bool) {
    let Some(batch) = batch(game) else {
        return;
    };
    if !game.native_on(MOD) {
        return;
    }
    let (x, y) = game.cursor;
    let hit = sort_buttons(game, &batch).into_iter().find(|(at, _)| on_button(*at, x, y));
    if let Some((_, sorts)) = hit {
        *canceled = true;
        if pressed && button == 0 {
            match sorts {
                Sorts::Slots(slots) => sort_slots(game, &slots),
                Sorts::Backpack => send(game, SORT, 0),
            }
        }
        return;
    }
    let Some(layout) = layout(game, &batch) else {
        return;
    };
    if !layout.covers(x, y) {
        let shift_in = pressed && button == 0 && game.has_shift_down() && game.carried.is_none();
        if shift_in
            && own_inventory(game)
            && let Some(slot) = game.slot_at(&batch, x, y)
            && (9..36).contains(&slot)
            && let Some(stack) = game.inventory[slot].as_ref()
            && !full_backpack(stack)
            && game.armor[CHEST].as_ref().is_some_and(|pack| has_room(game, pack, layout.slots, stack))
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

/// Whether the worn backpack has room for some of `stack`: an empty slot, or an equal stack that
/// isn't full.
fn has_room(game: &Game<'_>, pack: &ItemStack, slots: usize, stack: &ItemStack) -> bool {
    let max = Host::max_stack_size(game, &stack.item).unwrap_or(64);
    contents(pack, slots).iter().any(|s| match s {
        None => true,
        Some(s) => same_item(s, stack) && s.count < max,
    })
}

fn send(game: &mut Game<'_>, action: u8, slot: usize) {
    if let Err(e) = Host::net_send_to_server(game, CHANNEL, &[action, slot as u8]) {
        log::warn!("Backpacks: {e}");
    }
}

/// The screen hook (draw.rs): the panel, its slots and items, the hovered slot's highlight, and
/// the cursor stack again over the panel (the game drew it before the panel).
pub(super) fn draw(game: &Game<'_>, batch: &mut HudBatch) {
    if !game.native_on(MOD) {
        return;
    }
    draw_rows(game, batch);
    draw_sort_buttons(game, batch);
}

/// The sort buttons, as Horizon draws its own: a bordered square with three bars, lighter
/// under the cursor.
fn draw_sort_buttons(game: &Game<'_>, batch: &mut HudBatch) {
    let atlas = &game.gui.atlas;
    let (cx, cy) = game.cursor;
    for ((x, y), _) in sort_buttons(game, batch) {
        let face = if on_button((x, y), cx, cy) { rgba(220, 220, 220, 255) } else { rgba(160, 160, 160, 255) };
        batch.fill(atlas, x, y, SORT_BUTTON, SORT_BUTTON, rgba(55, 55, 55, 255));
        batch.fill(atlas, x + 1.0, y + 1.0, SORT_BUTTON - 2.0, SORT_BUTTON - 2.0, face);
        for (i, w) in [5.0, 4.0, 3.0].into_iter().enumerate() {
            batch.fill(atlas, x + 2.0, y + 2.0 + 2.0 * i as f32, w, 1.0, rgba(55, 55, 55, 255));
        }
    }
}

fn draw_rows(game: &Game<'_>, batch: &mut HudBatch) {
    let Some(layout) = layout(game, batch) else {
        return;
    };
    let Some(pack) = game.armor[CHEST].as_ref() else {
        return;
    };
    let items = contents(pack, layout.slots);
    let atlas = &game.gui.atlas;
    if let Some([px, py, w, h]) = layout.panel {
        batch.fill(atlas, px, py, w, h, rgba(0, 0, 0, 255));
        batch.fill(atlas, px + 1.0, py + 1.0, w - 2.0, h - 2.0, rgba(198, 198, 198, 255));
        batch.text(atlas, title_of(&pack.item), px + 8.0, py + 6.0, rgba(64, 64, 64, 255), false);
    }
    for i in 0..layout.slots {
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
    let Some(slots) = slots_of(&stack.item) else {
        return;
    };
    let used = contents(stack, slots).iter().flatten().count();
    let line = format!("{used} / {slots} slots used");
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
            // With a chest's or a crafting table's menu open, the inventory menu's own update
            // doesn't set the cursor.
            let carried = host.hub.guests[gi].carried.clone();
            let item = host.hub.stack(&carried);
            use horizon_protocol::packets::play::{Clientbound, SetCursorItem};
            host.hub.guests[gi].send(&Clientbound::SetCursorItem(SetCursorItem { item }));
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
    let Some(slots) = slots_of(&pack.item) else {
        return false;
    };
    let max = |s: &ItemStack| Host::max_stack_size(game, &s.item).unwrap_or(64);
    let mut items = contents(pack, slots);
    let before = (items.clone(), carried.clone(), inventory.to_vec());
    match action {
        PICK | PICK_RIGHT if slot < items.len() => {
            click(&mut items[slot], carried, action == PICK_RIGHT, &max);
        }
        SHIFT_OUT if slot < items.len() => {
            if let Some(stack) = items[slot].take() {
                items[slot] = insert(inventory, stack, &max, (0..9).chain(9..36));
            }
        }
        SHIFT_IN if (9..36).contains(&slot) && slot < inventory.len() => {
            if let Some(stack) = inventory[slot].take() {
                inventory[slot] = if full_backpack(&stack) {
                    Some(stack)
                } else {
                    insert(&mut items, stack, &max, 0..slots)
                };
            }
        }
        SORT => items = sorted(game, items, &max),
        _ => return false,
    }
    if (items.clone(), carried.clone(), inventory.to_vec()) == before {
        return false;
    }
    store(pack, &items);
    true
}

/// The backpack's slots sorted: equal stacks merged, in the creative screen's order (the
/// server's, so by id where it has none), then by id, the empty slots last.
fn sorted(
    game: &Game<'_>,
    items: Vec<Option<ItemStack>>,
    max: &impl Fn(&ItemStack) -> u32,
) -> Vec<Option<ItemStack>> {
    let n = items.len();
    let mut merged: Vec<ItemStack> = Vec::new();
    for stack in items.into_iter().flatten() {
        insert_merged(&mut merged, stack, max);
    }
    merged.sort_by(|a, b| {
        let key = |s: &ItemStack| (game.creative_screen.order_of(s).unwrap_or(usize::MAX), s.item.to_string());
        key(a).cmp(&key(b)).then(b.count.cmp(&a.count))
    });
    let mut out: Vec<Option<ItemStack>> = merged.into_iter().map(Some).collect();
    out.resize(n, None);
    out
}

/// Adds `stack` to `stacks`, topping up equal ones first, each to its stack size.
fn insert_merged(stacks: &mut Vec<ItemStack>, mut stack: ItemStack, max: &impl Fn(&ItemStack) -> u32) {
    let limit = max(&stack);
    for s in stacks.iter_mut() {
        if !same_item(s, &stack) {
            continue;
        }
        let moved = limit.saturating_sub(s.count).min(stack.count);
        s.count += moved;
        stack.count -= moved;
    }
    if stack.count > 0 {
        stacks.push(stack);
    }
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
