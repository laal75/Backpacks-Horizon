"""Writes the Horizon Backpacks content mod (data, assets and original textures) into argv[1]."""
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

# ---- manifest -------------------------------------------------------------------------------
write("horizon-mod.toml", f'''id = "{ID}"
name = "Horizon Backpacks"
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

# ---- the creative tab -----------------------------------------------------------------------
# While the player wears a backpack, the code shows its rows in this tab's grid instead.
lang[f"itemGroup.{ID}.backpacks"] = "Backpacks"
write(f"assets/{ID}/lang/en_us.json", lang)
write(f"data/{ID}/horizon/creative_tab/backpacks.json", {
    "title": "Backpacks",
    "icon": f"{ID}:leather_backpack",
    "items": [f"{ID}:{n}" for n in TIERS],
})
