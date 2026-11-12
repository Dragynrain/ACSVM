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

54 colours in hue-shifted ramps, light to dark. Mid-tones saturated, darks
muted and cooler, lights warmer: bright without looking like candy. The hex
values are in [pirates-palette.json](pirates-palette.json); they may still
move once tiles are hand-finished.

| Ramp | Steps | Use |
| --- | --- | --- |
| Outline | 2 | Dark warm brown; near-black for black cloth, flags, cave mouths |
| Sea | 9 | Foam to deep night blue, plus a muted teal for blue cloth |
| Sand | 3 | Sand, canvas, light skin |
| Skin | 4 | Skin tones |
| Wood | 4 | Sun-bleached to dark tarred wood |
| Leather | 3 | Red-brown leather, book covers, jerkins |
| Rust red | 5 | Coral orange to crimson: coats, bandanas, marines, lava |
| Brass and gold | 3 | Coins, hilts, buttons |
| Jungle green | 6 | Bright grass to dark canopy |
| Moss | 3 | Muted olive: swamp, olive cloth |
| Bone and stone | 8 | Bone white, rock, steel and charcoal, cool in the darks |
| Kraken | 4 | Pink to purple: the kraken, sea creatures, the supernatural |

The first plan was 28 colours. Snapping the 109 raw kit pictures showed
where that was too few: grey rock and flagstone turned blotchy with only
three greys, a black flag turned navy, and shallows, deep sea and blue cloth
need more blues than one ramp of four. The ramps were grown where
`_scripts/palgaps.py` (research folder) found the most pixels far from any
colour, not by fitting a palette to the images.

`scripts/palsnap.py PALETTE.json OUTDIR IMAGE...` snaps tiles to the ramps:
nearest colour in OKLab with the hue part weighted 1.5 times, so a small
purple arm stays purple instead of falling to a grey of the same lightness
(what the automatic 48-colour palette of `sprite64.py --palette` did to the
kraken). `--sheet` lists the tiles worst first; `--swatch` draws the ramps.

## 6. Framing

| Group | Framing |
| --- | --- |
| Terrain | Fills the tile edge to edge, seen from above, no border; repeating terrains made seamless by quilting (section 11). |
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

**Kit, second round (planned November 2026).** The first 109 cover the list
above but leave gaps against what Kit 4 in [design](../design.md) promises
(ships as portals, a cursed compass, a mermaid's charm, a ghost crew) and
against the stories section 1 says the kit must serve, such as a ghost-ship
horror. Generate these after the hand-finishing pass and LoRA v4, except the
ship, which *The King's Pardon* needs sooner. About 41 pictures, so a pirate
adventure that uses the whole kit needs an Extended picture limit near 180.

| Group | Pictures | Contents | Why |
| --- | --- | --- | --- |
| Ships | 2 | A ship at anchor (map place square), a ship as a portal thing | Ships are the portals between islands |
| Documents | 1 | Treasure map | The classic quest object; the kit has only a map table |
| Magic items | 8 | Cursed compass, conch shell, voodoo doll, tiki idol, the black spot, ghost lantern, crystal skull, mermaid's comb | Spells hang on things (364 magic spells and 134 magic items in the corpus); these carry any of the 15 actions without leaving the setting |
| Treasures | 5 | Loose gems, jewelled idol, jewelled crown, pearl necklace, gold chalice | A quest goal must look unlike every other treasure |
| Creatures, supernatural | 5 | Ghost pirate (pale, glowing eyes on the emissive layer), drowned captain (boss), skeleton musketeer, skeleton captain, ghost ship's lookout | The supernatural ramp (section 2) has one skeleton so far |
| Creatures, sea | 5 | Kraken head (boss), giant octopus, jellyfish, moray eel, mermaid | Mermaid written as a person with her own aims, like the island peoples |
| Creatures, land and cave | 6 | Caiman, giant bat, monkey (a thief), wild boar, giant spider, island jaguar | Swamp, cave and jungle have no creatures of their own |
| Crew | 9 | Captain (man and woman), quartermaster, cook, surgeon, navigator, cabin boy, powder monkey, bosun | The roles players most want to be, and the obvious bosses |
| **Total** | **41** | | Creatures reach 47, one over Classic's 46: drop one or share a picture |

Left out on purpose: more terrain (16 is the Classic limit) and food or
mundane items (no ACS rule uses them).

**KP objects (November 2026):** the 17 non-character KP pictures exist as
raw picks, made like the kit (LoRA v3, three seeds each). Two needed a new
prompt: a pink pearl in a pink shell vanished into the magenta background
(keep subjects off pink and purple, or they go with the background when
cut out), and a "brown sailcloth" log looked like the kit's book (a KP
object must differ from every kit picture in colour as well as shape). The
portrait is a placeholder until Governor Vail's own picture fixes his look.

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
64 px tile with the feet on the baseline; `--sheet` writes a
before-and-after contact sheet. Snap the result to the kit ramps with
`scripts/palsnap.py` (section 5); `--palette N` (a palette fitted to the
batch) is only for trying things out. Averaging
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

## 11. Terrain squares

**Recipe (November 2026).** The 16 terrain pictures are 64 px squares that
fill the square edge to edge. Two kinds:

- **Repeating terrains** (deep sea, shallows, reef, beach, jungle, cliff,
  swamp, volcanic rock, grassland) tile across the map, so they must be
  seamless.
- **Places** (reef channel, port town, fort, island village, wreck, sea-cave
  mouth, hidden cove) are one picture on one square.

Steps:

1. **Generate a texture, not a tile**, with LoRA v3: "acspirates style.
   <terrain>, zoomed in: <a few bold, dark-outlined features>, evenly spread
   across the whole image with no empty areas. Seen straight from above. A
   seamless game map texture that fills the entire image edge to edge, no
   border, no frame, no horizon, no single central object." plus the flat
   cel-shading words. Asking for a tile gave a framed square; asking for a
   texture fills the frame. Places say "a game world map location that fills
   the whole square image edge to edge" instead. The LoRA matched the kit
   better than plain Z-Image, whose textures were busy, repeating patterns.
2. **Make it seamless** with `scripts/quilt.py` (image quilting, no AI): roll
   by half, cover the seam cross with three patches of the image's own
   seam-free parts, each cut along the path of least difference. SwarmUI's
   Seamless Tileable works only on SDXL and crashed in its tiling node here;
   inpainting the seam with Z-Image left ghosted trees and visible band edges.
3. **Shrink** with `scripts/terrain64.py`, which crops, quilts and shrinks in
   one go (`--candidates` writes every combination and a contact sheet).
   Crop the centre to 0.5 to 0.75 first, or the features are too small for
   64 px. Bold outlined shapes (waves, tufts, tree crowns) stay crisp with
   k-centroid shrinking; small thin ones (shells, starfish) need the plain
   average, which k-centroid breaks into dots.

**What the runs taught:**

- **Prompt for features spread all over.** Quilting replaces the middle with
  the image's own seam-free parts, so sparse features near the edges vanish;
  two beaches came out blank sand.
- **Ask for "calm", "sparse", "low contrast" on the sea.** Bold wave crests
  made strong stripes that dominated the map.
- **"Strong contrast, saturated colours"** fixed washed-out shallows and
  near-black volcanic rock.
- **Z-Image will not draw a wreck**: "broken in two, lying on its side" gave
  an intact ship on four seeds. Qwen-Image-Edit 2511 (Lightning, 4 steps)
  fixed it from the best intact ship: "the hull is broken in two with a
  jagged gap in the middle, both halves tilted and half sunk, the masts
  snapped off short, the sails torn into rags, a few loose planks floating"
  plus "keep the same top-down view, water, island corners, style and
  colours" gave a clear wreck on all four seeds.
- **Editing works where generating won't.** The cutlass came out nearly
  straight from Z-Image whatever the prompt said. Asking Qwen-Image-Edit to
  bend the straight blade went wrong both ways: "like a crescent moon" gave
  sickles, and "a gentle, even curve" put all the bend in a hook at the tip.
  Starting from the kit's sabre, which already has the right even curve, and
  asking for "about twice as broad and a little shorter, keeping exactly the
  same gentle curve" gave a proper cutlass. Edit from the picture nearest the
  wanted shape, and change one property at a time.
- **Places carry their own ground**, which rarely matches the neighbouring
  terrain (the fort's grass is lighter than grassland). Once every tile is
  snapped to the ramps (section 5), a ground is a few exact palette colours,
  so matching it is a colour swap: the cove's and cave mouth's moss greens
  become grassland's greens, the wreck's water becomes the shallows'.
- **Keep shallow water calm**: the brightest sea step as the base of the
  shallows was loud next to the deep sea; the base is the third step, with
  the second only for highlights, in the shallows and in every place on them.
- Terrains meet with hard square edges, as on the original world map.

**Rooms (November 2026).** Walls and floor spaces are textures too: walls
"seen straight from the front, flat, no perspective", floors "seen straight
from above", made seamless and shrunk with `scripts/terrain64.py`. Doors, the
store counter, the lava vent, gangplank and trapdoor are single squares
(`--kind place`); cropping to 0.8 removes the white card frame some seeds
draw. As with terrain, sparse features (sand, mud, cave rock) vanish under
quilting unless the prompt says "covered all over". The one room floor is
kept nearly plain on purpose: it shows in every room. Doors carry their own
wall, which rarely matches the room's wall picture, and no single wall suits
every room. So a door is an overlay: Qwen-Image-Edit 2511 replaces the wall
around it with flat magenta ("keep only the door itself and its frame or
arch exactly as they are"), which kept the door untouched on every try. The
cut-out is shrunk with the same crop as the pick, snapped, and laid over
each wall picture, one door square per wall. Two more floor fixes are
colour swaps too: the scorched floor goes from near-black to ash grey (dark
enough to read as burnt, apart from the lava vent), and the hull wall goes
one wood step darker so it stands apart from the plank floor.
