# Horizon Backpacks

Backpacks for [Horizon](https://github.com/horizon-rust/horizon) that add rows to your own
inventory. Wear one in the chest slot and its rows appear beside your inventory whenever you open
it, with whatever key you open it with. In creative, the **Backpacks** tab shows them instead of its
items while you wear one. Everything is original: names, numbers, code and textures.

| Backpack | Rows | Slots | Crafted from |
|---|---|---|---|
| Leather Backpack | 1 | 9 | A chest, 7 leather and a string |
| Iron Backpack | 2 | 18 | A Leather Backpack and an iron block |
| Gold Backpack | 3 | 27 | An Iron Backpack and a gold block |
| Diamond Backpack | 4 | 36 | A Gold Backpack and a diamond block |
| Obsidian Backpack | 5 | 45 | A Diamond Backpack and obsidian; fire, lava and explosions can't destroy it |

What a backpack holds stays in the backpack: take it off, drop it or upgrade it, and the items go
with it. Its tooltip says how many slots are used.

## Using it

- Click the backpack's slots as you would a chest's: left to pick up or put down a stack, right
  for half a stack or one item.
- Shift-click a backpack slot to send it to your inventory, and shift-click an inventory slot to
  send it to the backpack.
- A backpack never goes inside another backpack.
- The rows hide while the recipe book is open; close the book to use them.

## Limits

- Single player and LAN hosts only: on a server, or as a LAN guest, the rows don't show.
- It is mixin code: it reaches into the game's inventory screen, so a new Horizon version may need
  it updated.

## Build and play

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
