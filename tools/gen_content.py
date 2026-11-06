"""Writes the Backpacks content mod (data, assets and original textures) into argv[1]."""
import json
import os
import random
import struct
import sys
import zlib

ROOT = sys.argv[1]
ID = "horizon_backpacks"
# The one place the version is set (tools/version.py keeps the rest in step).
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "VERSION"), encoding="utf-8") as f:
    VERSION = f.read().strip()


def write(rel, obj):
    path = os.path.join(ROOT, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        if isinstance(obj, str):
            f.write(obj)
        else:
            json.dump(obj, f, indent=2)
            f.write("\n")


def png(rel, pixels):
    """pixels: 16 rows of 16 (r, g, b, a)."""
    raw = b"".join(b"\x00" + bytes(c for px in row for c in px) for row in pixels)
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    body = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 16, 16, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    path = os.path.join(ROOT, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(body)


def shade(c, k):
    return tuple(max(0, min(255, int(v * k))) for v in c[:3]) + (255,)


# ---- tiers ----------------------------------------------------------------------------------
# name: (title, colour, slots). Rows of 5 beside the inventory; 45 at most, the creative
# screen's 9 by 5 grid. native/glue.rs has the same slots.
TIERS = {
    "leather_backpack": ("Leather Backpack", (150, 92, 52), 10),
    "iron_backpack": ("Iron Backpack", (200, 202, 206), 20),
    "gold_backpack": ("Gold Backpack", (240, 200, 60), 30),
    "diamond_backpack": ("Diamond Backpack", (92, 222, 214), 40),
    "obsidian_backpack": ("Obsidian Backpack", (60, 44, 88), 45),
}
# The Ender tiers: each tier's Ender version, by an Ender Upgrade on it. They open their owner's
# store, so they hold more and native/glue.rs has the same slots.
ENDER = {
    "ender_leather_backpack": ("Ender Leather Backpack", "leather_backpack", 30),
    "ender_iron_backpack": ("Ender Iron Backpack", "iron_backpack", 35),
    "ender_gold_backpack": ("Ender Gold Backpack", "gold_backpack", 40),
    "ender_diamond_backpack": ("Ender Diamond Backpack", "diamond_backpack", 45),
    "ender_obsidian_backpack": ("Ender Obsidian Backpack", "obsidian_backpack", 50),
}
ENDER_TEAL = (24, 96, 86)
# Each tier past leather: the one before it and this block, by crafting_transmute, which keeps
# what the backpack holds (its custom data).
UPGRADES = {
    "iron_backpack": ("leather_backpack", "minecraft:iron_block"),
    "gold_backpack": ("iron_backpack", "minecraft:gold_block"),
    "diamond_backpack": ("gold_backpack", "minecraft:diamond_block"),
    "obsidian_backpack": ("diamond_backpack", "minecraft:obsidian"),
}
STRAP = (70, 46, 30)


def backpack(colour, seed):
    """A pack with a flap, two straps and a buckle, on a clear background."""
    rng = random.Random(seed)
    px = [[(0, 0, 0, 0)] * 16 for _ in range(16)]
    for y in range(3, 15):
        for x in range(3, 13):
            edge = x in (3, 12) or y in (3, 14)
            px[y][x] = shade(colour, 0.6 if edge else 0.88 + rng.random() * 0.14)
    # The flap over the top third.
    for y in range(3, 7):
        for x in range(4, 12):
            px[y][x] = shade(colour, 1.1 if y < 6 else 0.7)
    # Carrying loop.
    for x in range(6, 10):
        px[1][x] = shade(STRAP, 1.0)
    px[2][6] = px[2][9] = shade(STRAP, 1.0)
    # Straps and buckle.
    for y in range(6, 14):
        px[y][5] = px[y][10] = shade(STRAP, 0.95)
    px[9][7] = px[9][8] = (220, 200, 120, 255)
    # A pocket.
    for x in range(6, 10):
        px[12][x] = shade(colour, 0.65)
    return px


for i, (name, (_, colour, _)) in enumerate(TIERS.items()):
    png(f"assets/{ID}/textures/item/{name}.png", backpack(colour, 10 + i))
for i, (name, (_, base, _)) in enumerate(ENDER.items()):
    # Dark teal over the tier's colour, with an ender eye for a buckle.
    colour = tuple((a + 2 * b) // 3 for a, b in zip(TIERS[base][1], ENDER_TEAL))
    px = backpack(colour, 30 + i)
    px[9][7] = px[9][8] = (60, 220, 170, 255)
    px[10][7] = px[10][8] = (30, 150, 120, 255)
    png(f"assets/{ID}/textures/item/{name}.png", px)
_px = [[(0, 0, 0, 0)] * 16 for _ in range(16)]
for _y in range(3, 13):
    for _x in range(3, 13):
        _edge = _x in (3, 12) or _y in (3, 12)
        _px[_y][_x] = shade(ENDER_TEAL, 0.7 if _edge else 1.0)
for _y in range(6, 10):
    for _x in range(6, 10):
        _px[_y][_x] = (60, 220, 170, 255)
_px[7][7] = _px[8][8] = (20, 60, 55, 255)
png(f"assets/{ID}/textures/item/ender_upgrade.png", _px)

# ---- manifest -------------------------------------------------------------------------------
write("horizon-mod.toml", f'''id = "{ID}"
name = "Backpacks"
version = "{VERSION}"
description = "Backpacks worn in the chest slot that add rows to the inventory. Content half; the code is horizon_backpacks_core."
authors = []
api = "0.2"
side = "both"
capabilities = ["registries"]
''')

# ---- items ----------------------------------------------------------------------------------
# What fire, lava and explosions can't destroy: the Obsidian Backpack's.
write(f"data/{ID}/tags/damage_type/backpack_proof.json",
      {"values": ["#minecraft:is_fire", "#minecraft:is_explosion"]})
lang = {}
for name, (title, _, _) in TIERS.items():
    components = {
        "minecraft:equippable": {"slot": "chest", "equip_sound": "minecraft:item.armor.equip_leather"},
    }
    item = {"template": "minecraft:sugar", "max_stack_size": 1, "components": components}
    if name == "obsidian_backpack":
        components["minecraft:damage_resistant"] = {"types": f"#{ID}:backpack_proof"}
        item["rarity"] = "rare"
    write(f"data/{ID}/horizon/item/{name}.json", item)
    write(f"assets/{ID}/models/item/{name}.json",
          {"parent": "minecraft:item/generated", "textures": {"layer0": f"{ID}:item/{name}"}})
    write(f"assets/{ID}/items/{name}.json", {"model": {"type": "minecraft:model", "model": f"{ID}:item/{name}"}})
    lang[f"item.{ID}.{name}"] = title
for name, (title, base, _) in ENDER.items():
    components = {
        "minecraft:equippable": {"slot": "chest", "equip_sound": "minecraft:item.armor.equip_leather"},
    }
    item = {"template": "minecraft:sugar", "max_stack_size": 1, "components": components, "rarity": "uncommon"}
    if base == "obsidian_backpack":
        components["minecraft:damage_resistant"] = {"types": f"#{ID}:backpack_proof"}
        item["rarity"] = "rare"
    write(f"data/{ID}/horizon/item/{name}.json", item)
    write(f"assets/{ID}/models/item/{name}.json",
          {"parent": "minecraft:item/generated", "textures": {"layer0": f"{ID}:item/{name}"}})
    write(f"assets/{ID}/items/{name}.json", {"model": {"type": "minecraft:model", "model": f"{ID}:item/{name}"}})
    lang[f"item.{ID}.{name}"] = title
write(f"data/{ID}/horizon/item/ender_upgrade.json", {"template": "minecraft:sugar", "max_stack_size": 16})
write(f"assets/{ID}/models/item/ender_upgrade.json",
      {"parent": "minecraft:item/generated", "textures": {"layer0": f"{ID}:item/ender_upgrade"}})
write(f"assets/{ID}/items/ender_upgrade.json", {"model": {"type": "minecraft:model", "model": f"{ID}:item/ender_upgrade"}})
lang[f"item.{ID}.ender_upgrade"] = "Ender Upgrade"


# ---- crafting and the recipe book -----------------------------------------------------------
def unlock(name, item):
    """Vanilla's recipe advancement: holding `item` unlocks the recipe."""
    criteria = {"has_the_recipe": {"trigger": "minecraft:recipe_unlocked",
                                   "conditions": {"recipe": f"{ID}:{name}"}},
                "has_item": {"trigger": "minecraft:inventory_changed",
                             "conditions": {"items": [{"items": item}]}}}
    write(f"data/{ID}/advancement/recipes/misc/{name}.json", {
        "parent": "minecraft:recipes/root",
        "criteria": criteria,
        "requirements": [list(criteria)],
        "rewards": {"recipes": [f"{ID}:{name}"]},
    })


write(f"data/{ID}/recipe/leather_backpack.json", {
    "type": "minecraft:crafting_shaped", "category": "equipment",
    "pattern": ["LSL", "LCL", "LLL"],
    "key": {"L": "minecraft:leather", "S": "minecraft:string", "C": "minecraft:chest"},
    "result": {"id": f"{ID}:leather_backpack"},
})
unlock("leather_backpack", "minecraft:leather")
for name, (before, material) in UPGRADES.items():
    write(f"data/{ID}/recipe/{name}.json", {
        "type": "minecraft:crafting_transmute", "category": "equipment",
        "input": f"{ID}:{before}", "material": material,
        "result": {"id": f"{ID}:{name}"},
    })
    unlock(name, f"{ID}:{before}")
# The Iron Backpack is also made outright, as the Leather one is, with iron for leather.
write(f"data/{ID}/recipe/iron_backpack_from_iron.json", {
    "type": "minecraft:crafting_shaped", "category": "equipment",
    "pattern": ["ISI", "ICI", "III"],
    "key": {"I": "minecraft:iron_ingot", "S": "minecraft:string", "C": "minecraft:chest"},
    "result": {"id": f"{ID}:iron_backpack"},
})
unlock("iron_backpack_from_iron", "minecraft:iron_ingot")

# An ender chest and 4 leather make the upgrade, and the upgrade on any backpack makes it Ender.
write(f"data/{ID}/recipe/ender_upgrade.json", {
    "type": "minecraft:crafting_shaped", "category": "equipment",
    "pattern": [" L ", "LCL", " L "],
    "key": {"L": "minecraft:leather", "C": "minecraft:ender_chest"},
    "result": {"id": f"{ID}:ender_upgrade"},
})
unlock("ender_upgrade", "minecraft:ender_chest")
for name, (_, base, _) in ENDER.items():
    write(f"data/{ID}/recipe/{name}.json", {
        "type": "minecraft:crafting_transmute", "category": "equipment",
        "input": f"{ID}:{base}", "material": f"{ID}:ender_upgrade",
        "result": {"id": f"{ID}:{name}"},
    })
    unlock(name, f"{ID}:ender_upgrade")
# Up the tiers as a plain backpack goes.
for name, (before, material) in UPGRADES.items():
    write(f"data/{ID}/recipe/ender_{name}_from_ender.json", {
        "type": "minecraft:crafting_transmute", "category": "equipment",
        "input": f"{ID}:ender_{before}", "material": material,
        "result": {"id": f"{ID}:ender_{name}"},
    })
    unlock(f"ender_{name}_from_ender", f"{ID}:ender_{before}")

# ---- the creative tab -----------------------------------------------------------------------
# While the player wears a backpack, the code shows its rows in this tab's grid instead.
lang[f"itemGroup.{ID}.backpacks"] = "Backpacks"
write(f"assets/{ID}/lang/en_us.json", lang)
write(f"data/{ID}/horizon/creative_tab/backpacks.json", {
    "title": "Backpacks",
    "icon": f"{ID}:leather_backpack",
    "items": [f"{ID}:{n}" for n in TIERS] + [f"{ID}:ender_upgrade"] + [f"{ID}:{n}" for n in ENDER],
})
