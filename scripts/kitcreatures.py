"""Writes kits/pirates/creatures.toml from the table below (docs/content.md 4.3).

Usage: python scripts/kitcreatures.py   (from the repository root)

The table is the source; edit it here and rerun. Bands: townsfolk, crew,
officers and the law for people (gear from the kit's things), beasts with
natural weapons, the dead with magic defence.
"""
from pathlib import Path

# key, name, picture, (con, str, dex, spd, wis, size),
# (dodge, parry, melee, missile, armor_skill), alignment, flags, wealth,
# weapon, armor, extra carries
P = [
    # Townsfolk
    ("trader", "TRADER", "trader", (10, 8, 10, 5, 13, 9), (14, 10, 18, 10, 0), "friend", "", 120, "gutting_knife", None, []),
    ("tavern_keeper", "TAVERN KEEPER", "tavernkeep", (12, 12, 9, 5, 10, 12), (12, 14, 24, 8, 0), "friend", "", 60, "gutting_knife", None, ["rum_flask"]),
    ("fisher", "FISHER", "fisher", (10, 10, 10, 5, 9, 10), (15, 12, 20, 15, 0), "neutral", "", 10, "gaff_hook", None, []),
    ("cook", "SHIP'S COOK", "cook", (11, 11, 8, 4, 8, 12), (10, 10, 20, 6, 0), "neutral", "", 15, "gutting_knife", None, []),
    ("surgeon", "SURGEON", "surgeon", (9, 7, 12, 5, 15, 9), (16, 12, 16, 10, 0), "neutral", "", 80, "gutting_knife", None, ["salve"]),
    ("navigator", "NAVIGATOR", "navigator", (10, 8, 12, 5, 16, 9), (18, 14, 20, 18, 0), "neutral", "", 90, "cutlass", None, ["compass"]),
    ("cabin_boy", "CABIN BOY", "cabinboy", (7, 6, 12, 6, 8, 6), (24, 8, 12, 8, 0), "neutral", "", 2, "gutting_knife", None, []),
    ("diver", "PEARL DIVER", "diver", (11, 9, 14, 6, 11, 8), (24, 14, 22, 18, 0), "neutral", "", 30, "gutting_knife", None, ["pearls"]),
    # Crew
    ("deckhand", "DECKHAND", "deckhand", (11, 11, 11, 5, 8, 10), (18, 14, 26, 14, 0), "neutral", "aggressive", 15, "cutlass", None, []),
    ("deckhand_night", "NIGHT WATCH", "deckhand_night", (11, 11, 11, 5, 8, 10), (18, 14, 26, 14, 0), "enemy", "aggressive", 15, "cutlass", None, []),
    ("powder_monkey", "POWDER MONKEY", "powdermonkey", (8, 7, 13, 6, 8, 7), (24, 10, 16, 16, 0), "neutral", "", 4, "gutting_knife", None, ["powder_keg"]),
    ("boarder", "BOARDER", "boarder", (13, 14, 11, 5, 8, 11), (16, 18, 34, 10, 10), "enemy", "aggressive brave", 25, "boarding_axe", "leather_jerkin", []),
    ("gunner", "GUNNER", "gunner", (12, 12, 12, 5, 10, 11), (16, 14, 24, 30, 0), "enemy", "aggressive", 25, "pistol", None, ["cutlass"]),
    ("smuggler", "SMUGGLER", "smuggler", (11, 10, 13, 6, 11, 9), (22, 16, 26, 22, 0), "neutral", "", 60, "cutlass", "oilskin", ["pistol"]),
    ("thief", "SLINKER", "thief", (9, 8, 15, 7, 11, 8), (30, 14, 20, 20, 0), "thief", "", 30, "gutting_knife", None, []),
    ("bosun", "BOSUN", "bosun", (15, 15, 11, 5, 10, 12), (16, 20, 38, 12, 10), "neutral", "aggressive brave", 50, "cutlass", "leather_jerkin", []),
    ("quartermaster", "QUARTERMASTER", "quartermaster", (14, 12, 12, 5, 13, 11), (18, 22, 34, 20, 10), "neutral", "brave", 120, "cutlass", "leather_jerkin", ["pistol"]),
    # Officers and the law
    ("house_guard", "HOUSE GUARD", "houseguard", (14, 13, 11, 5, 9, 11), (16, 22, 34, 14, 20), "enemy", "brave", 20, "halberd", "buff_coat", []),
    ("marine", "MARINE", "marine", (13, 12, 12, 5, 10, 11), (18, 20, 32, 26, 15), "enemy", "aggressive brave", 20, "musket", "navy_coat", ["cutlass"]),
    ("marine_blue", "MARINE", "marine_blue", (13, 12, 12, 5, 10, 11), (18, 20, 32, 26, 15), "enemy", "aggressive brave", 20, "musket", "navy_coat", ["cutlass"]),
    ("rifleman", "RIFLEMAN", "rifleman", (12, 11, 14, 5, 11, 10), (20, 14, 22, 38, 10), "enemy", "aggressive", 20, "musket", "navy_coat", []),
    ("sergeant", "SERGEANT", "sergeant", (17, 15, 13, 6, 12, 12), (22, 28, 44, 26, 25), "enemy", "aggressive brave", 60, "sabre", "navy_coat", ["pistol"]),
    ("captain_f", "CAPTAIN", "captainf", (19, 13, 16, 7, 15, 10), (30, 32, 46, 32, 20), "neutral", "brave", 300, "sabre", "buff_coat", ["pistol"]),
    ("captain_m", "CAPTAIN", "captainm", (20, 16, 13, 6, 14, 12), (24, 32, 48, 28, 20), "neutral", "brave", 300, "sabre", "buff_coat", ["pistol"]),
    # Beasts
    ("ships_cat", "SHIP'S CAT", "cat", (4, 3, 18, 9, 6, 2), (40, 0, 20, 0, 0), "neutral", "", 0, "claws", None, []),
    ("parrot", "PARROT", "parrot", (3, 2, 16, 10, 9, 1), (44, 0, 12, 0, 0), "neutral", "", 0, "beak", None, []),
    ("parrot_blue", "BLUE PARROT", "parrot_blue", (3, 2, 16, 10, 9, 1), (44, 0, 12, 0, 0), "neutral", "", 0, "beak", None, []),
    ("gull", "GULL", "gull", (3, 2, 15, 11, 4, 1), (40, 0, 12, 0, 0), "neutral", "aggressive", 0, "beak", None, []),
    ("monkey", "MONKEY", "monkey", (6, 6, 18, 9, 7, 3), (40, 0, 18, 0, 0), "thief", "", 0, "claws", None, []),
    ("bat", "BAT", "bat", (3, 2, 17, 11, 3, 1), (44, 0, 14, 0, 0), "enemy", "aggressive", 0, "teeth", None, []),
    ("giant_crab", "GIANT CRAB", "crab", (14, 14, 6, 3, 2, 12), (6, 0, 30, 0, 40), "enemy", "aggressive brave", 0, "pincers", None, []),
    ("blue_crab", "BLUE CRAB", "crab_blue", (16, 15, 6, 3, 2, 12), (6, 0, 32, 0, 40), "enemy", "aggressive brave", 0, "pincers", None, []),
    ("leech", "GIANT LEECH", "leech", (6, 4, 6, 3, 1, 4), (10, 0, 24, 0, 0), "enemy", "aggressive brave", 0, "teeth", None, []),
    ("eel", "MORAY EEL", "eel", (10, 10, 14, 7, 2, 8), (24, 0, 32, 0, 0), "enemy", "aggressive", 0, "teeth", None, []),
    ("jellyfish", "JELLYFISH", "jellyfish", (5, 2, 4, 2, 1, 6), (6, 0, 30, 0, 0), "enemy", "aggressive brave", 0, "sting", None, []),
    ("spider", "GIANT SPIDER", "spider", (9, 9, 16, 8, 3, 7), (28, 0, 30, 0, 0), "enemy", "aggressive", 0, "teeth", None, []),
    ("boar", "WILD BOAR", "boar", (14, 15, 9, 7, 3, 12), (12, 0, 34, 0, 10), "enemy", "aggressive brave", 0, "teeth", None, []),
    ("jaguar", "JAGUAR", "jaguar", (15, 16, 17, 9, 5, 12), (30, 0, 44, 0, 0), "enemy", "aggressive brave", 0, "claws", None, []),
    ("caiman", "CAIMAN", "caiman", (18, 18, 8, 4, 2, 16), (10, 0, 40, 0, 30), "enemy", "aggressive brave", 0, "teeth", None, []),
    ("reef_shark", "REEF SHARK", "shark", (16, 16, 14, 9, 2, 16), (20, 0, 44, 0, 10), "enemy", "aggressive brave", 0, "teeth", None, []),
    ("octopus", "OCTOPUS", "octopus", (12, 10, 15, 6, 6, 10), (26, 0, 34, 0, 0), "enemy", "aggressive", 0, "tentacle", None, []),
    ("octopus_rose", "ROSE OCTOPUS", "octopus_rose", (12, 10, 15, 6, 6, 10), (26, 0, 34, 0, 0), "enemy", "aggressive", 0, "tentacle", None, []),
    ("sea_serpent", "SEA SERPENT", "serpent", (28, 22, 12, 8, 6, 24), (10, 0, 52, 0, 30), "enemy", "aggressive brave", 0, "teeth", None, []),
    ("red_serpent", "RED SERPENT", "serpent_red", (32, 24, 12, 8, 7, 24), (10, 0, 56, 0, 30), "enemy", "aggressive brave", 0, "teeth", None, []),
    ("kraken_arm", "KRAKEN ARM", "krakenarm", (24, 22, 10, 0, 3, 20), (0, 0, 48, 0, 20), "enemy", "aggressive brave", 0, "tentacle", None, []),
    ("kraken", "KRAKEN", "krakenhead", (50, 28, 10, 0, 12, 31), (0, 0, 60, 0, 40), "enemy", "aggressive brave", 0, "tentacle", None, []),
    # The sea's other folk
    ("mermaid", "MERMAID", "mermaid", (12, 8, 16, 7, 18, 9), (30, 10, 18, 0, 0), "neutral", "", 0, "claws", None, ["mermaid_comb"]),
    # The dead
    ("skeleton", "SKELETON", "skeleton", (12, 12, 10, 5, 4, 10), (12, 14, 30, 0, 0), "enemy", "aggressive brave", 5, "cutlass", None, []),
    ("cursed_skeleton", "CURSED SKELETON", "skeleton_cursed", (14, 13, 10, 5, 4, 10), (12, 16, 34, 0, 0), "enemy", "aggressive brave", 10, "cutlass", None, []),
    ("skeleton_musketeer", "SKELETON MUSKETEER", "skelmusket", (12, 11, 12, 5, 4, 10), (14, 10, 22, 30, 0), "enemy", "aggressive brave", 5, "musket", None, []),
    ("skeleton_captain", "SKELETON CAPTAIN", "skelcapt", (20, 15, 12, 6, 10, 11), (18, 28, 46, 20, 0), "enemy", "aggressive brave", 200, "sabre", None, []),
    ("drowned_captain", "DROWNED CAPTAIN", "drownedcapt", (24, 16, 11, 5, 14, 12), (16, 26, 48, 0, 0), "enemy", "aggressive brave", 300, "grave_touch", None, []),
    ("ghost", "GHOST", "ghost", (14, 6, 16, 7, 16, 9), (40, 0, 30, 0, 0), "enemy", "aggressive brave", 0, "grave_touch", None, []),
]

# Magic defence by band (content 4.3): the dead; ghosts only magic hurts.
DEFENCE = {
    "skeleton": "non_magic_half",
    "cursed_skeleton": "non_magic_half",
    "skeleton_musketeer": "non_magic_half",
    "skeleton_captain": "non_magic_half",
    "drowned_captain": "non_magic_half",
    "ghost": "only_magic",
}


def main():
    out = [
        "# The pirate kit's creatures (docs/content.md 4.3). CC BY-SA 4.0.",
        "# Written by scripts/kitcreatures.py: edit its table, not this file.",
        "# Power is Wisdom and Life Force Constitution (content 3.6) unless set.",
        "",
    ]
    for key, name, pic, (con, st, dx, sp, wi, sz), (do, pa, me, mi, ar), al, flags, wealth, weapon, armor, extra in P:
        out += [
            "[[creature]]",
            f'key = "{key}"',
            f'name = "{name}"',
            f'picture = "{pic}"',
            f"constitution = {con}",
            f"strength = {st}",
            f"dexterity = {dx}",
            f"speed = {sp}",
            f"wisdom = {wi}",
            f"size = {sz}",
            f"dodge = {do}",
            f"parry = {pa}",
            f"melee = {me}",
            f"missile = {mi}",
            f"armor_skill = {ar}",
            f'alignment = "{al}"',
        ]
        for f in flags.split():
            out.append(f"{f} = true")
        if key in DEFENCE:
            out.append(f'magic_defense = "{DEFENCE[key]}"')
        if wealth:
            out.append(f"wealth = {wealth}")
        carries = [weapon] + ([armor] if armor else []) + extra
        out.append(f'weapon = "{weapon}"')
        if armor:
            out.append(f'armor = "{armor}"')
        out.append("carries = [" + ", ".join(f'{{ thing = "{c}" }}' for c in carries) + "]")
        out.append("")
    Path("kits/pirates/creatures.toml").write_text("\n".join(out), encoding="utf-8")
    print(f"{len(P)} creatures")


if __name__ == "__main__":
    main()
