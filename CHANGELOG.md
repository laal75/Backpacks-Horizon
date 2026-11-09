# Changelog

Versions follow [semver](https://semver.org): `tools/version.py <version>` sets one everywhere,
and each release is tagged `v<version>`. New entries go under **Unreleased**.

## Unreleased

## 0.4.1 - 2026-11-09

- Built for Horizon Nightly 2026-11-08 (fdc87ae): the code now pins that commit

## 0.4.0 - 2026-11-07

- Backpacks with more than 8 rows, the Obsidian one and the two biggest Ender ones, are now as tall as the Diamond
  Backpack, with a scroll bar. Drag it or use the mouse wheel over the panel.

## 0.3.0 - 2026-11-06

- **The mod's id is now `backpacks`**, and its items are `backpacks:...`. Worlds made with an older version
  lose its items and Ender stores, so start a new world.

## 0.2.0 - 2026-11-06

- **Ender Backpacks**: 30, 35, 40, 45 and 50 slots, opening one store for each player, saved with the
  world. Items stay in it through death, and each player sees their own. An **Ender Upgrade** (ender
  chest and 4 leather) on any backpack makes its Ender version; what it held moves into the store.
- On mod screens the inventory's sort button moved to the inventory's label row, so it no longer
  overlaps the chest's own Sort button at the top right.

## 0.1.2 - 2026-11-05

- On mod screens the inventory's sort button is at the top right of the screen, where it no longer
  covers a machine's upgrade slot; beside the screen's own sort button when it has one. Sort buttons
  name what they sort when hovered. The Expanded Chests chest's own Sort button replaces this mod's.

## 0.1.1 - 2026-11-05

- Hovering an item in the backpack shows its name and tooltip.
- On a trader's screen the backpack sits beside the screen, not over its trades.
- Shift-clicks go round as in vanilla: backpack → hotbar first, hotbar → main rows (the game's own),
  main rows → backpack (to the hotbar instead when the backpack has no room).

### Changed
- The mod is now called Backpacks (its ids are unchanged, so worlds keep their backpacks).

### Added
- The backpack's slots also show on the crafting table's, chests' and other containers' screens,
  not only the player's own inventory.
- A sort button on the backpack panel sorts the backpack's own slots (merging equal stacks).
- Horizon's sort buttons on mod screens, where Horizon leaves them out: the main inventory's on
  every mod machine's screen, and the chest's own on Horizon Chests' chests. They follow Horizon's
  "Sort" extra.
- The Iron Backpack can also be crafted outright: a chest, 7 iron ingots and a string.
- Leather, Iron, Gold, Diamond and Obsidian backpacks (10, 20, 30, 40 and 45 slots in rows of 5),
  worn in the chest slot. Their slots show beside the inventory screen and in the creative Backpacks tab. Upgrading keeps the
  contents.
- An empty backpack can go inside another; one holding anything can't.
- A backpack's tooltip says how many of its slots are used.
