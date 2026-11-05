# Backpacks

Backpacks for [Horizon](https://github.com/horizon-rust/horizon) that add rows to your own
inventory. Wear one in the chest slot and its rows appear beside your inventory whenever you open
it, with whatever key you open it with. In creative, the **Backpacks** tab shows them instead of its
items while you wear one. Everything is original: names, numbers, code and textures.

| Backpack | Slots | Crafted from |
|---|---|---|
| Leather Backpack | 10 (5 x 2) | A chest, 7 leather and a string |
| Iron Backpack | 20 (5 x 4) | A Leather Backpack and an iron block, or a chest, 7 iron ingots and a string |
| Gold Backpack | 30 (5 x 6) | An Iron Backpack and a gold block |
| Diamond Backpack | 40 (5 x 8) | A Gold Backpack and a diamond block |
| Obsidian Backpack | 45 (5 x 9) | A Diamond Backpack and obsidian; fire, lava and explosions can't destroy it |

The slots sit in rows of 5 beside the inventory; in the creative tab they fill its 9-wide grid.

What a backpack holds stays in the backpack: take it off, drop it or upgrade it, and the items go
with it. Its tooltip says how many slots are used.

## Using it

- Click the backpack's slots as you would a chest's: left to pick up or put down a stack, right
  for half a stack or one item.
- Shift-click a backpack slot to send it to your inventory, and shift-click an inventory slot to
  send it to the backpack.
- An empty backpack can go inside another backpack; one that holds anything can't.
- The slots show on every screen with your inventory in it: your own, a crafting table's, a
  chest's or a mod's.
- The sort button on the backpack's panel sorts the backpack on its own; Horizon's own sort buttons
  sort the rest. With Horizon's "Sort" extra on, this mod also adds them where Horizon doesn't: the
  main inventory on mod machines' screens, and the chest on Expanded Chests' chests.
- The rows hide while the recipe book is open; close the book to use them.

## Limits

- Single player and LAN hosts only: on a server, or as a LAN guest, the rows don't show.
- It is mixin code: it reaches into the game's inventory screen, so a new Horizon version may need
  it updated.

## Install

### Easy install

The ready-made packs are in the [`dist`](dist) folder, so there is nothing to clone:

| File | What it is | Where it goes in the game folder |
|---|---|---|
| [`horizon_backpacks-0.1.1.hmod`](dist/horizon_backpacks-0.1.1.hmod) | The content: blocks, items, recipes, textures | `horizon-mods/` |
| [`horizon_backpacks_core-0.1.1.hnmod`](dist/horizon_backpacks_core-0.1.1.hnmod) | The code | `horizon-native-mods/` |

1. Download both files. The game folder of the main Horizon instance is `%APPDATA%\horizon` on
   Windows; `horizon-launcher instance list` shows your instances.
2. Copy each file into the folder the table names.
3. Turn both halves on in the game folder's `horizon-mods.txt`:

   ```text
   mod.horizon_backpacks:true
   mod.horizon_backpacks_core:true
   ```

4. Build the code into the instance. This also rebuilds your other installed native mods, so they
   stay in the game. The first build downloads a Rust toolchain and takes 20 to 30 minutes; later
   ones take a few minutes.

   ```sh
   horizon-launcher native-build --instance <instance id> --trust-native-code --allow-version-mismatch
   ```

   `--trust-native-code` confirms that you trust the code: native mods run with your user's rights.
5. Play the instance: `horizon-launcher play --instance <instance id>`.

### From source

```sh
horizon-launcher native-build native --instance <instance> --trust-native-code
```

Copy `content/` to the instance's `horizon-mods/horizon_backpacks/` (or put
`horizon_backpacks-<version>.hmod` from `python tools/pack.py` there), and turn on both
`mod.horizon_backpacks` and `mod.horizon_backpacks_core` in its `horizon-mods.txt`.

The code builds on Horizon commit `67dad0d` (the 2026-11-04 Nightly), set in
`native/horizon-mixin.toml`.

## Development

- `content/` is **generated**: edit `tools/gen_content.py`, then
  `python tools/gen_content.py content`.
- `native/glue.rs` is the code, a module of the game; `click.rs`, `draw.rs` and `tooltip.rs` call
  it from the mouse, screen and tooltip hooks. The tiers' rows are in both the code and the
  generator.
- `VERSION` holds the version; `python tools/version.py <version>` sets it everywhere and dates
  [CHANGELOG.md](CHANGELOG.md). `python tools/pack.py` writes the `.hmod` and `.hnmod` to `dist/`.

## License

MIT; see [LICENSE](LICENSE).
