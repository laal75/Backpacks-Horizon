// Backpacks, inside the game (crate::game::horizon_backpacks): a backpack worn in the
// chest slot adds rows of slots to the player's own inventory screen, beside it on the left, and
// to the content mod's "Backpacks" creative tab, in place of its grid. The rows are the backpack
// item's own: its custom data keeps them (`backpack_items`), so they go wherever it goes.
//
// The trait API can't reach the cursor stack or the screen's slots, so this is mixin code. The
// client draws the rows from its copy of the worn backpack (draw.rs) and turns a click on them
// into a mod message (click.rs); the server, single player's integrated one included, applies it
// to the player's inventory, cursor and backpack and sends the inventory back (net.rs).
//
// An Ender Backpack holds nothing itself. Its rows are the player's own store, kept on the server
// in the world's horizon_backpacks/ender folder, one file a player. So the client's drawing and
// clicks work as for any backpack, the server copies what the rows show onto the worn item (tick.rs).
// That copy is never read back: only the file counts, so a copied or dropped pack makes no items.
use horizon_client::creative::Creative;
use horizon_client::hud::{HudBatch, rgba};
use horizon_client::items::ItemStack;
use horizon_client::mod_api::Host;
use horizon_world::nbt::{Compound, JavaString, Limits, NamedTag, Tag};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

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
const TIERS: [(&str, &str, usize); 10] = [
    ("horizon_backpacks:leather_backpack", "Leather Backpack", 10),
    ("horizon_backpacks:iron_backpack", "Iron Backpack", 20),
    ("horizon_backpacks:gold_backpack", "Gold Backpack", 30),
    ("horizon_backpacks:diamond_backpack", "Diamond Backpack", 40),
    ("horizon_backpacks:obsidian_backpack", "Obsidian Backpack", 45),
    ("horizon_backpacks:ender_leather_backpack", "Ender Backpack", 30),
    ("horizon_backpacks:ender_iron_backpack", "Ender Backpack", 35),
    ("horizon_backpacks:ender_gold_backpack", "Ender Backpack", 40),
    ("horizon_backpacks:ender_diamond_backpack", "Ender Backpack", 45),
    ("horizon_backpacks:ender_obsidian_backpack", "Ender Backpack", 50),
];
/// Every Ender Backpack's id starts so.
const ENDER_PREFIX: &str = "horizon_backpacks:ender_";
/// The slots of a player's store: the biggest Ender Backpack's. A smaller one shows the first few.
const STORE_SLOTS: usize = 50;
/// The custom data key that says which store and state an Ender Backpack's copy shows.
const SEEN_KEY: &str = "ender_seen";
/// What a backpack held when it became an Ender Backpack that its owner's store had no room for.
const PENDING_KEY: &str = "ender_pending";
/// What the creative tab's grid shows at most.
const CREATIVE_SLOTS: usize = 45;
/// Slots to a row on the inventory screen.
const COLUMNS: usize = 5;
/// How far a trader's screen reaches left of the inventory's origin: it is 276 wide, not 176
/// (the game's `trading::SHIFT`).
const TRADER_SHIFT: f32 = 50.0;

fn is_ender(item: &str) -> bool {
    item.starts_with(ENDER_PREFIX) && slots_of(item).is_some()
}

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

/// Changes the item's custom data.
fn edit(stack: &mut ItemStack, change: impl FnOnce(&mut Compound)) {
    let mut data = match stack.data().and_then(|d| d.other("minecraft:custom_data")) {
        Some(Tag::Compound(c)) => c.clone(),
        _ => Compound::default(),
    };
    change(&mut data);
    stack.data_mut().set_other("minecraft:custom_data", Some(Tag::Compound(data)));
}

fn store(stack: &mut ItemStack, items: &[Option<ItemStack>]) {
    edit(stack, |data| {
        data.insert(KEY.into(), horizon_client::save::encode_items(items));
    });
}

/// A backpack that holds something, which never goes in another backpack (an empty one does). An
/// Ender Backpack holds nothing: its items are in its owner's store.
fn full_backpack(stack: &ItemStack) -> bool {
    !is_ender(&stack.item)
        && slots_of(&stack.item).is_some_and(|slots| contents(stack, slots).iter().any(Option::is_some))
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
        let trader = game.station.as_ref().is_some_and(|s| s.merchant.is_some());
        let left = if trader { ox - TRADER_SHIFT } else { ox };
        let panel = [(left - w - 4.0).max(2.0), oy, w, h];
        let first = (panel[0] + 8.0, oy + 18.0);
        return Some(Layout { slots, cols: COLUMNS, first, panel: Some(panel) });
    }
    if game.creative_tab_open() && game.creative_screen.tab().id == TAB {
        let (ox, oy) = Game::creative_origin(batch);
        let (x, y) = Creative::tab_slot_position(Creative::DISPLAY)?;
        let slots = slots.min(CREATIVE_SLOTS);
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

/// The sort buttons this mod adds, by their top left corner, with the sort extra on: the
/// backpack panel's own, and on a mod's screen (where Horizon's own sort buttons don't go) the
/// main inventory's, on the inventory's own label row at its right end, clear of the screen's own
/// sort button, which stays at the top right of the main panel.
fn sort_buttons(game: &Game<'_>, batch: &HudBatch) -> Vec<((f32, f32), Sorts)> {
    let mut buttons = Vec::new();
    if !game.settings.extras.sort || game.screen != Screen::Inventory {
        return buttons;
    }
    if let Some(Layout { panel: Some([px, py, w, _]), .. }) = layout(game, batch) {
        buttons.push(((px + w - SORT_BUTTON - 7.0, py + 5.0), Sorts::Backpack));
    }
    if game.native_screen_slots().is_some()
        && let (Some(ix), Some(iy)) = (game.native_screen_inventory_x(), game.native_screen_inventory_y())
    {
        let (ox, oy) = game.inventory_origin(batch);
        let main: Vec<usize> = (9..36).collect();
        // The last slot's right edge, 8 columns of 18 and its 16 along.
        let at = (ox + ix + 160.0 - SORT_BUTTON, oy + iy - SORT_BUTTON - 1.0);
        buttons.push((at, Sorts::Slots(main)));
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
    let mut hovered = None;
    for ((x, y), sorts) in sort_buttons(game, batch) {
        if on_button((x, y), cx, cy) {
            hovered = Some(match sorts {
                Sorts::Slots(_) => "Sort inventory",
                Sorts::Backpack => "Sort backpack",
            });
        }
        let face = if on_button((x, y), cx, cy) { rgba(220, 220, 220, 255) } else { rgba(160, 160, 160, 255) };
        batch.fill(atlas, x, y, SORT_BUTTON, SORT_BUTTON, rgba(55, 55, 55, 255));
        batch.fill(atlas, x + 1.0, y + 1.0, SORT_BUTTON - 2.0, SORT_BUTTON - 2.0, face);
        for (i, w) in [5.0, 4.0, 3.0].into_iter().enumerate() {
            batch.fill(atlas, x + 2.0, y + 2.0 + 2.0 * i as f32, w, 1.0, rgba(55, 55, 55, 255));
        }
    }
    if let Some(label) = hovered {
        game.tooltip(batch, label, cx, cy);
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
    // The hovered item's tooltip, as any slot's: not with something in the hand.
    if game.carried.is_none()
        && let Some(stack) = layout.slot_at(cx, cy).and_then(|i| items.get(i)).and_then(Option::as_ref)
    {
        game.draw_tooltip_lines(batch, &game.stack_tooltip(stack), cx, cy);
    }
}

/// The tooltip hook (tooltip.rs): how full a backpack is.
pub(super) fn tooltip(stack: &ItemStack, lines: &mut Vec<(horizon_client::mod_api::Text, u32)>) {
    let Some(slots) = slots_of(&stack.item) else {
        return;
    };
    if is_ender(&stack.item) {
        let line = format!("{slots} slots of your own Ender store");
        lines.push((horizon_client::mod_api::Text::literal(line), rgba(170, 170, 170, 255)));
        return;
    }
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
        let changed = apply(game, player, &mut g.inventory, &mut g.armor[CHEST], &mut g.carried, action, slot as usize);
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
    player: u128,
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
    let ender = is_ender(&pack.item);
    let world = Host::world_dir(game);
    let mut items = match (ender, &world) {
        (false, _) => contents(pack, slots),
        (true, Some(world)) => with_store(world, player, |s| s.items.clone()),
        (true, None) => return false,
    };
    let before = (items.clone(), carried.clone(), inventory.to_vec());
    match action {
        PICK | PICK_RIGHT if slot < slots => {
            click(&mut items[slot], carried, action == PICK_RIGHT, &max);
        }
        SHIFT_OUT if slot < slots => {
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
    match (ender, &world) {
        (true, Some(world)) => {
            with_store(world, player, |s| s.set(world, player, items));
            sync(game, world, player, pack);
        }
        _ => store(pack, &items),
    }
    true
}

// ---- the server: Ender Backpacks -------------------------------------------------------------

/// A player's Ender store, as its file holds it.
struct EnderStore {
    items: Vec<Option<ItemStack>>,
    /// Changes with what it holds, to tell a worn backpack's copy is behind.
    hash: u64,
}

impl EnderStore {
    fn encoded(items: &[Option<ItemStack>]) -> Vec<u8> {
        let root = NamedTag {
            name: JavaString::from(""),
            tag: Tag::compound([("Items", horizon_client::save::encode_items(items))]),
        };
        horizon_world::nbt::write_named(&root, Limits::saved()).unwrap_or_default()
    }

    fn hash_of(bytes: &[u8]) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hasher);
        hasher.finish()
    }

    fn load(world: &PathBuf, player: u128) -> Self {
        let path = store_path(world, player);
        let mut items = vec![None; STORE_SLOTS];
        if let Ok(bytes) = std::fs::read(&path) {
            match horizon_world::nbt::read_named(&bytes, Limits::saved()) {
                Ok((root, _)) => items = horizon_client::save::decode_items(root.tag.get("Items"), STORE_SLOTS),
                Err(e) => {
                    // Kept for a look, not written over.
                    log::warn!("Backpacks: {}: {e:?}", path.display());
                    let _ = std::fs::rename(&path, path.with_extension("bad"));
                }
            }
        }
        let hash = Self::hash_of(&Self::encoded(&items));
        Self { items, hash }
    }

    /// Takes on `items` and writes them to its file.
    fn set(&mut self, world: &PathBuf, player: u128, items: Vec<Option<ItemStack>>) {
        let bytes = Self::encoded(&items);
        self.hash = Self::hash_of(&bytes);
        self.items = items;
        let path = store_path(world, player);
        let new = path.with_extension("new");
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&new, &bytes))
            .and_then(|()| std::fs::rename(&new, &path));
        if let Err(e) = written {
            log::warn!("Backpacks: {}: {e}", path.display());
        }
    }
}

fn store_path(world: &PathBuf, player: u128) -> PathBuf {
    world.join("horizon_backpacks").join("ender").join(format!("{player:032x}.dat"))
}

thread_local! {
    /// The stores in use, by world and player. Only the server's thread reaches them.
    static STORES: RefCell<HashMap<(PathBuf, u128), EnderStore>> = RefCell::new(HashMap::new());
}

fn with_store<R>(world: &PathBuf, player: u128, f: impl FnOnce(&mut EnderStore) -> R) -> R {
    STORES.with(|stores| {
        let mut stores = stores.borrow_mut();
        let store = stores
            .entry((world.clone(), player))
            .or_insert_with(|| EnderStore::load(world, player));
        f(store)
    })
}

/// Brings a worn Ender Backpack up to date: what a backpack held when it became one moves into
/// its owner's store (what doesn't fit waits on the item), and the item gets a copy of the rows
/// it shows. True when the item changed.
fn sync(game: &Game<'_>, world: &PathBuf, player: u128, pack: &mut ItemStack) -> bool {
    let Some(slots) = slots_of(&pack.item) else {
        return false;
    };
    let custom = pack.data().and_then(|d| d.other("minecraft:custom_data")).cloned();
    let tag = |key: &str| custom.as_ref().and_then(|c| c.get(key));
    let seen_now = match tag(SEEN_KEY) {
        Some(Tag::String(s)) => Some(s.to_string_lossy().to_string()),
        _ => None,
    };
    let mut changed = false;
    // Never synced: it was a plain backpack, and what's under KEY is real.
    let mut waiting = horizon_client::save::decode_items(tag(PENDING_KEY), STORE_SLOTS);
    if seen_now.is_none() {
        let held = horizon_client::save::decode_items(tag(KEY), STORE_SLOTS);
        let mut free = waiting.iter().enumerate().filter(|(_, s)| s.is_none()).map(|(i, _)| i).collect::<Vec<_>>().into_iter();
        for stack in held.into_iter().flatten() {
            if let Some(i) = free.next() {
                waiting[i] = Some(stack);
            }
        }
        changed = true;
    }
    if waiting.iter().any(Option::is_some) {
        let max = |s: &ItemStack| Host::max_stack_size(game, &s.item).unwrap_or(64);
        let mut items = with_store(world, player, |s| s.items.clone());
        let before = items.clone();
        for stack in waiting.iter_mut() {
            if let Some(s) = stack.take() {
                *stack = insert(&mut items, s, &max, 0..STORE_SLOTS);
            }
        }
        if items != before {
            with_store(world, player, |s| s.set(world, player, items));
            changed = true;
        }
    }
    let (rows, hash) = with_store(world, player, |s| (s.items[..slots.min(STORE_SLOTS)].to_vec(), s.hash));
    let seen = format!("{player:032x}:{hash:016x}:{slots}");
    if !changed && seen_now.as_deref() == Some(seen.as_str()) {
        return false;
    }
    edit(pack, |data| {
        data.insert(KEY.into(), horizon_client::save::encode_items(&rows));
        data.insert(SEEN_KEY.into(), Tag::String(seen.as_str().into()));
        if waiting.iter().any(Option::is_some) {
            data.insert(PENDING_KEY.into(), horizon_client::save::encode_items(&waiting));
        } else {
            data.remove(&JavaString::from(PENDING_KEY));
        }
    });
    true
}

/// The server tick hook (tick.rs): keeps the Ender Backpacks being worn up to date.
pub(super) fn tick(game: &mut Game<'_>) {
    if !game.native_on(MOD) {
        return;
    }
    let Some(world) = Host::world_dir(&*game) else {
        return;
    };
    let Some(mut host) = game.lan.take() else {
        return;
    };
    for gi in 0..host.hub.guests.len() {
        let player = host.hub.guests[gi].uuid;
        let Some(pack) = host.hub.guests[gi].armor[CHEST].as_mut() else {
            continue;
        };
        if is_ender(&pack.item) && sync(game, &world, player, pack) {
            host.hub.send_inventory(gi);
        }
    }
    game.lan = Some(host);
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
