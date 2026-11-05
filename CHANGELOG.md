# Changelog

Versions follow [semver](https://semver.org): `tools/version.py <version>` sets one everywhere,
and each release is tagged `v<version>`. New entries go under **Unreleased**.

## Unreleased

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
