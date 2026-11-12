# Art: pirate kit style sheet

Style rules for Kit 4 (pirates; see the art pipeline and Kit 4 in
[design](../design.md)). Every tile in the kit, generated or hand-drawn,
follows this sheet, so a chest, a marine and a sea tile look like they came
from one book. Licensed CC BY 4.0.

## 1. Direction

**Bright, slightly weathered storybook adventure.** Neutral and fairly
cheerful at base; darkness comes from the engine, not the art.

- A generic kit has to serve stories we can't predict: a comic treasure hunt,
  a ghost-ship horror, a tutorial. Bright art can be darkened; dark art can't
  be brightened.
- Moods are palette ramps per region (night, fog, cursed, underwater) plus
  torch light from normal maps and emissive layers. A cursed island is a ramp
  and a light, not a second art set.
- At 64 to 128 px, dark and moody art loses readability first.

## 2. Setting

- **Era:** Golden Age Caribbean, about 1700 to 1720: tricorns, frock coats,
  bandanas, sashes, flintlocks, cutlasses, sloops and galleons. No modern
  objects, no dollar signs; coins are plain doubloons.
- **The supernatural** (ghosts, skeleton crew, kraken, curses) is the one
  allowed departure and has its own ramp: cool, desaturated, glowing eyes
  on the emissive layer.
- **Island peoples** are people with their own aims: traders, fishers,
  navigators, chiefs. Never hazards or scenery, never caricature.
- **Crews are mixed** in gender, age and skin tone.

## 3. Characters

Characters drift most between realistic, cartoon and anime, so these are fixed.

| Rule | Value |
| --- | --- |
| Proportions | Adult, about 6 heads tall, slightly broad build. 4.5 heads read as children; 7 heads turn thin, realistic and unreadable at 64 px. |
| Height | Every adult human the same height; the boss and big creatures may exceed it. |
| Pose | Standing, front-facing (a slight three-quarter turn is fine), full body, feet on the baseline. |
| Face | Always drawn, mature, with clear simple features: eyes, brows, nose and mouth. No anime eyes, no blank faces, no realistic skin. |
| People | Every subject line states gender, age, build, skin tone and hair; vary them across the kit. |
| Hands | Hold the defining item (cutlass, musket, lantern) so the silhouette names the role. |
| Silhouette | One read per role: hat shape, coat length and held item must tell crew, officer, marine and captain apart in solid black. |

## 4. Rendering

- **Cel-painted**, 2 or 3 tone steps per material, no soft gradients, no
  photo texture.
- **Light from the top left**; shadows lean cool (blue, purple), highlights
  warm (yellow, orange).
- **Outline:** dark warm brown, about 1 px at 64 px; inner lines thinner
  and coloured by the fill. Not pure black.
- **Shadow:** none baked in; the engine draws it.

## 5. Palette

About 28 colours in hue-shifted ramps. Mid-tones saturated, darks muted:
bright without looking like candy. Hex values get fixed after the first
hand-finished tiles; the ramps are:

| Ramp | Steps | Use |
| --- | --- | --- |
| Sea | 4 | Deep blue to teal: water, blue cloth |
| Sand and skin, light | 3 | Sand, canvas, light skin |
| Skin, medium and dark | 4 | Skin tones, leather |
| Wood | 4 | Sun-bleached to dark tarred wood |
| Rust red | 3 | Coats, bandanas, marines |
| Brass and gold | 3 | Coins, hilts, buttons |
| Jungle green | 3 | Plants, green cloth |
| Bone and stone | 3 | Skeletons, rock, steel (with a cool tint) |
| Outline | 1 | Dark warm brown |

## 6. Framing

| Group | Framing |
| --- | --- |
| Terrain | Fills the tile edge to edge, seen from above, no border; seams fixed by half-shift and inpaint. |
| Objects | Front view tilted slightly from above so chests and barrels show their tops; centred with a small margin. |
| Weapons | Diagonal, point up and to the right. |
| Creatures | As characters: front-facing, feet on the baseline. |

## 7. Prompt template

Z-Image Turbo at CFG 1 ignores negative prompts, so every constraint is
stated positively. Generation is 1024 px, 8 steps, CFG 1, no LoRA, on flat
magenta (`#FF00FF`) for cutting out. Only the subject changes; the prefix and
suffix are fixed per group.

**Character prefix (v3):**

> Illustrated character sprite for a classic adventure video game. One adult
> person, full body, standing upright and facing the viewer, stylized adult
> proportions about six heads tall with a slightly broad build, a mature face
> with clear simple features. Flat cel shading with two or three solid tones
> per colour and hard edges, no texture, no brush strokes, cool shadows and
> warm highlights, light from the top left, a bold clean dark brown outline
> around the whole figure. Caribbean, around 1710.

**Character suffix:**

> Both feet on the ground at the bottom of the figure, the whole figure
> visible with a margin around it, isolated on a plain flat solid magenta
> background, no frame, no border, nothing else in the image.

**Subject line:** role first, then gender, age, build, skin, hair, clothes
from top to bottom, and the held item. Example: "A woman pirate quartermaster
with a feminine face, a tall woman in her forties with olive skin and a dark
braid, wearing a weathered long brown coat over a waistcoat, a wide leather
belt and tall boots, holding a flintlock pistol pointed down."

Never name pirate games, films or their characters in a prompt.

**What the test batches taught** (6 roles each: deckhand, quartermaster,
captain, marine, skeleton, navigator):

| Batch | Template | Result |
| --- | --- | --- |
| 1 | 4.5 heads, "friendly face", roles only | One style, but adults read as children; every crew member light-skinned; the navigator a cartoon stereotype; captains in the famous film look. |
| 2 | 7 heads, "grounded illustrated", full descriptions | Adult and varied, but painterly and semi-realistic, outline lost, too thin at 64 px; "woman" ignored. |
| 3 | v3 above | Adult, flat, outlined, consistent across roles and seeds; reads at 64 px. Quartermaster female once the subject starts with "A woman". |
| 3q | v3 on Qwen-Image 2512 (Q4_K_M GGUF, 4-step Lightning bf16) | Dark, gritty and textured despite "flat"; crimson instead of magenta backgrounds, some framed; the quartermaster male again. Z-Image follows this sheet better. |

Qwen-Image 2512 on a 16 GB card: the 19 GB fp8 build runs alone (about 340 s
an image) but any LoRA runs out of memory; the Q4_K_M GGUF with the bf16
Lightning LoRA runs in about 15 s an image.

- **Say "flat" and "no texture" outright**, or the model drifts painterly.
- **Put gender first** in the subject line; buried in the middle, it loses.
  The same holds for any detail that matters: opening the cook's line with
  "a one-legged cook with a wooden peg leg" got the peg leg where the end of
  the line didn't. Ethnicity is the exception: "East Asian man with Chinese
  features", even first, still gave a European face, so that needs
  Qwen-Image-Edit on a good figure. Qwen-Image-Edit 2511 (Q4_K_M GGUF,
  4-step Lightning, about 24 s) kept the pose, coat, chart, outline and
  background exactly and changed only the face, but added wrinkles and
  detailed eyes the flat style doesn't have: retouch the face to the sheet
  afterwards. At 64 px a face is about 4 px wide, so skin, hair and clothing
  carry who a person is more than the eyes do.
- **A skeleton needs "smooth bare skull, completely bald"**; some seeds still
  add film-pirate hair, so pick those out.
- **Z-Image Turbo varies little between seeds**: good for consistency, so
  change the subject line for variety, not the seed.
- Some seeds frame the figure as a card with a white border; drop those.

## 8. Asset list

What the kit and the first adventure, *The King's Pardon*, need, by picture.
**Kit** pictures are generic and reused by any pirate adventure; **KP**
pictures belong to that adventure alone. About 135 pictures in all, over
Classic's 110, so the adventure declares an Extended picture limit
(design, Kit 4). Counts are first estimates.

ACS has no room backgrounds: a room is its wall picture plus the one floor
picture the master list allows (manual). Ground variety (sand, mud,
flagstone) comes from spaces drawn as floor tiles and placed on the floor.

| Group | Pictures | Kit | KP | Contents |
| --- | --- | --- | --- | --- |
| Terrain | 16 | 16 | | Deep sea, shallows, reef, reef channel, beach, jungle, cliff, swamp, volcanic rock, port town, fort, island village, wreck site, sea-cave mouth, hidden cove, open grassland |
| Floor | 1 | 1 | | Packed sand and planks, neutral enough for every room |
| Walls | 8 | 8 | | Ship's hull, cave rock, palisade logs, stilt hut planks and reeds, stone fort, mansion panelling, woven village wall, dressed stone |
| Floor spaces | 8 | 8 | | Sand, mud, scorched sand, lava vent, flagstone, carpet, boardwalk, shallow water |
| Doors | 7 | 7 | | Wooden door, fort gate, gangplank, cave passage, double door, iron vault door, trapdoor |
| Store | 1 | 1 | | A trader's counter under an awning |
| Set dressing | 22 | 17 | 5 | Kit: cannon, barrels, powder barrels, crate stack, chest, rigging, ship's wheel, map table, tavern table, bar counter, bookcase, nets and fish rack, dive platform, smokehouse rack, bones, coral, flagpole. KP: notice board, Vail's desk, his portrait, vault plinth, pile of signed pardons |
| Documents | 7 | 2 | 5 | Kit: scroll, book. KP: letter of passage, ledger, governor's seal, sea log, pardon receipt (the three that open gates must look distinct) |
| Weapons | 11 | 11 | | Cutlass, flintlock pistol, blunderbuss, musket with bayonet, officer's sabre, boarding axe, gutting knife, throwing knives, harpoon, gaff hook, halberd |
| Armor | 4 | 4 | | Buff coat, oilskin, leather jerkin, navy coat |
| Magic and tools | 11 | 9 | 2 | Kit: powder keg, lantern, spyglass, ship's bell, rum flask, salve jar, charm, diving helm, small boat. KP: the flat-bottomed punt, the shark-tooth charm's own look if the kit charm doesn't fit |
| Treasures | 8 | 3 | 5 | Kit: gold pile, coin chest, crate of goods. KP: Harrow's strongbox, Quill plate, fort pay chest, drowned gold, Reef Pearl |
| Creatures, generic | 22 | 22 | | Deckhand, gunner, boarder, rifleman, smuggler, slinker thief, navy marine, sergeant, house guard, diver, fisher, trader, tavern keeper, skeleton or drowned crew, giant crab, kraken arm, reef shark, sea serpent, leech, gull, ship's cat, parrot |
| Creatures, named | 8 | | 8 | Meg Harrow, Jory Fenn, Abel and Cass Quill, Sabine Duroc, Hob Varrick, Governor Vail, Warden Kesi |
| **Total** | **134** | **109** | **25** | |

Outside the picture list: title art, the *Magpie* as the player's world-map
marker (a player picture), the UI font, and later, if Extended adds scene
pictures, an illustration for the vault reveal.

Creature rules for the named eight: each needs a silhouette no generic role
shares (section 3), and house guards must read apart from navy marines, since
the story turns on who serves Vail.

## 9. Consistency

1. Pick one reference figure from a character batch: the model sheet.
2. Make other characters from it with Qwen-Image-Edit 2511 ("same style and
   proportions, now a navy marine with a musket"), not from new seeds alone.
3. Hand-finish about 30 tiles to this sheet, then train the kit LoRA on only
   those (design, art pipeline step 3).
4. Check each batch against the sheet: same height, outline colour,
   light direction and face style.

**From 1024 px to a tile:** `scripts/sprite64.py` cuts the figure off the
magenta, crops it, shrinks it to 60 px tall with k-centroid downscaling (each
pixel takes its block's dominant colour, not the average) and places it on a
64 px tile with the feet on the baseline; `--palette N` snaps a batch to one
shared palette, `--sheet` writes a before-and-after contact sheet. Averaging
the whole frame down, as the first contact sheets did, smears outlines and
faces; cropping first does most of the good. Faces are still about 4 px
wide at 64, so eyes and mouths get hand-placed pixels.

## 10. Kit LoRA

**v2 (November 2026), trigger `acspirates style`.** A bootstrap: trained on
40 picked raw generations (14 crew, 26 objects and creatures), not yet on
hand-finished tiles as section 9 plans.

- **Captions** say what is in the picture and nothing about the style, so the
  style goes into the trigger: "acspirates style. <subject line>. On a plain
  flat magenta background." Where the model drifted, the caption describes the
  result, not the request.
- **Training:** OneTrainer's "z-image DeTurbo LoRA 16GB" preset on
  ostris's Z-Image De-Turbo (de-distilled Turbo, so the LoRA suits 8-step
  Turbo; Base-trained LoRAs are reported to come out soft on Turbo), int8
  weights, rank 16, alpha 16, learning rate 1e-4, batch 1, buckets 512 and
  768, no flip (light stays top left), no crop jitter. About 1.3 steps a
  second on a 16 GB card. Checkpoints at 600, 1200 and 1800 steps; the run
  stopped at epoch 55 of 60, before the 2400-step save.
- **v1**, the same set on Z-Image Base at 512 and 768 with batch 2 and fp8,
  spilled out of VRAM (14 to 40 s a step) and was stopped.

**Test** (6 subjects not in the set, seed 401, FP8 Turbo, 8 steps): the
short trigger prompt alone gives the kit's outline and flat tones, close to
the full v3 template. 1800 steps is the best so far; at 600 steps new
subjects (a palm tree) still come out painterly. Two things it learned that
the sheet doesn't want: a soft ground shadow under every subject (the
training images had one, and no caption said so) and pirate dress on anyone
(the fiddler got a red bandana). Next set: caption the shadow or remove it,
and add non-pirate people.

**v3 (November 2026), the current one.** 54 images: v2's 40 plus 8
townsfolk (innkeeper, merchant, governor, fisherwoman, blacksmith, fruit
seller, priest, farmer) and 6 tavern objects. Every background is flattened
before training: each pixel near the background's hue and connected to the
image border becomes pure `#FF00FF`, which removes the ground shadow and the
background's noise but not magenta inside the subject. Same settings as v2,
run to the end: 45 epochs, 2430 steps, about 40 minutes.

Test (the 6 new subjects, seeds 401 and 402, plus a baker and a scholar):

- **The final save is the best**: no ground shadow, a near-flat magenta
  background (easier to cut out), crisper outlines than v2.
- **Townsfolk stay townsfolk** with v2 and v3 alike. The fiddler's bandana
  and tricorn came from the test prompt itself ("a pirate fiddler"), not the
  LoRA.
- **People come out half-length** with the short prompt. Add "full body,
  standing, feet on the ground" to character prompts, or caption the framing
  in the next set.
- **Plants and ground are its weak spot**: the palm tree gets detailed,
  painterly leaves and sand that runs to the image edges. The set has no
  plants or terrain; add some before the next round.
