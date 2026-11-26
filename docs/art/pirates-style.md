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

56 colours in hue-shifted ramps, light to dark. Mid-tones saturated, darks
muted and cooler, lights warmer: bright without looking like candy. The hex
values are in [pirates-palette.json](pirates-palette.json); they may still
move once tiles are hand-finished.

| Ramp | Steps | Use |
| --- | --- | --- |
| Outline | 2 | Dark warm brown; near-black for black cloth, flags, cave mouths |
| Sea | 9 | Foam to deep night blue, plus a muted teal for blue cloth |
| Sand | 3 | Sand, canvas, light skin |
| Skin | 5 | Skin tones, pale to deep |
| Wood | 4 | Sun-bleached to dark tarred wood |
| Leather | 3 | Red-brown leather, book covers, jerkins |
| Rust red | 5 | Coral orange to crimson: coats, bandanas, marines, lava |
| Brass and gold | 4 | Lemon yellow cloth, coins, hilts, buttons |
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
pictures belong to that adventure alone. About 138 pictures in all, over
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
| Documents | 8 | 2 | 6 | Kit: scroll, book. KP: letter of passage, ledger, governor's seal, the *Constance*'s sea log, pardon receipt, Kesi's pilot chart (the four that open gates must look distinct) |
| Weapons | 11 | 11 | | Cutlass, flintlock pistol, blunderbuss, musket with bayonet, officer's sabre, boarding axe, gutting knife, throwing knives, harpoon, gaff hook, halberd |
| Armor | 4 | 4 | | Buff coat, oilskin, leather jerkin, navy coat |
| Magic and tools | 13 | 9 | 4 | Kit: powder keg, lantern, spyglass, ship's bell, rum flask, salve jar, charm, diving helm, small boat. KP: the flat-bottomed punt, the shark-tooth charm's own look if the kit charm doesn't fit, Duroc's signal pistol, Abel's key |
| Treasures | 8 | 3 | 5 | Kit: gold pile, coin chest, crate of goods. KP: Harrow's strongbox, Quill plate, fort pay chest, drowned gold, Reef Pearl |
| Creatures, generic | 22 | 22 | | Deckhand, gunner, boarder, rifleman, smuggler, slinker thief, navy marine, sergeant, house guard, diver, fisher, trader, tavern keeper, skeleton or drowned crew, giant crab, kraken arm, reef shark, sea serpent, leech, gull, ship's cat, parrot |
| Creatures, named | 9 | | 9 | Meg Harrow, Jory Fenn, Abel and Cass Quill, Sabine Duroc, Hob Varrick, Governor Vail (twice: the mansion and the *Sovereign*), Warden Kesi |
| **Total** | **138** | **109** | **29** | |

Outside the picture list: title art, the *Magpie* as the player's world-map
marker (a player picture), the UI font, and later, if Extended adds scene
pictures, an illustration for the vault reveal.

Creature rules for the named characters: each needs a silhouette no generic
role shares (section 3). Vail's house guards must read apart from Duroc's
deserted navy marines, since the story turns on who serves Vail.

**The named characters (re-briefed November 2026).** The roles come from the
adventure's story design (the maintainer's working doc, kept in its own private project,
content.md 6:
premise, rivals, finale and script). The first briefs, from 12 November,
guessed roles from the asset list without it and got every role wrong; five
of the eight pictures have to be made again, and three can be edited. These
briefs replace them; what can be kept is in the last column.

| Character | Role in the story | Look | Silhouette | From the first round |
| --- | --- | --- | --- | --- |
| "Black" Meg Harrow | First rival, Gullwreck Cay. Loud, fearless gunner, a former privateer whose letter of marque Vail revoked; raids his shipping | Woman, mid-fifties, weathered tan skin, long grey braid, scorched buff coat, powder-blackened hands and cheek, wide hat with one feather | Wide hat over a buff coat; a blunderbuss held across her body | Face, hat and braid: an edit (coat, held item) |
| Jory Fenn | Second rival, the Saltmarsh. Genial smuggler who calls everyone "friend", a slinker thief, runs Vail's contraband and keeps good books | Man, thirties, light brown skin, curly hair under a frayed straw hat, open waistcoat over a loose shirt, rolled trousers, bare feet, a wide friendly grin | Straw hat; a stuffed sack over one shoulder and a small account book in the other hand | Face and hat: an edit (older, sack and book for the pole) |
| Abel Quill | Third rival, Twin Hulls; the twin who schemes. Answered Vail's notice and kept the hoard | Man, forties, dark brown skin, neat short beard, narrow and upright, a good blue captain's coat, brass buttons, a tricorn | Lean, upright; a folded paper (the old pardon notice) and a pistol held low | New: same face as Cass, built as his opposite |
| Cass Quill | Abel's twin, the muscle. Tired of running; surrenders if spared | Man, forties, dark brown skin, the same face as Abel, a full black beard, very broad, sleeveless leather jerkin, a knotted red-brown head cloth, bare arms | Broad shoulders; a boarding axe on one shoulder | New. The twins must read as twins: same face and skin, opposite build and dress |
| Captain Sabine Duroc | Fourth rival, Fort Teal. Former navy lieutenant, disciplined and dry, deserted with the marines who survived Vail's hangings | Woman, forties, pale skin, short black hair, a navy officer's coat faded from blue to grey-blue, patched at the elbows, a black bicorne worn sideways | The only bicorne; officer's sabre and spyglass | Face and bicorne: an edit (coat to faded navy, rapier to sabre) |
| Old Hob Varrick | Rival in the Hollow. Oldest pirate in these waters, half mad and oddly kind; doesn't fight, warns | Man, seventies, olive skin gone leathery, wild white hair and beard, thin and bent, a sea coat thirty years out of fashion gone green with damp, barefoot | Stooped; a lit lantern held high and a boarding axe used as a walking stick | New. Must read apart from the drowned crew and the ghosts: living colour, warm lantern light (emissive) |
| Governor Edmund Vail | The villain: Hob's old quartermaster Ned Vail, now governor and pirate lord of these waters. Warm and generous until the vault | Man, about sixty, pale skin, grey hair tied back (no wig), clean-shaven, a plain good dark-grey coat, white stock, no embroidery | Empty hands in Act 1: the only named character without a weapon, holding a quill pen | New: plain, not grand |
| Vail, the *Sovereign* | Act 3: the same man on his own quarterdeck, a duellist | As above, the same coat buttoned up, a sword cane drawn | The same silhouette with the blade out: one change that says who he is now | New (a second picture for his second creature record) |
| Warden Kesi | Speaks for the islanders of Ama Reef, the divers and salvagers Vail has taxed for a generation. Practical, guarded | Woman, sixties, deep brown skin, grey hair cropped short, short and stout, a sun-faded indigo wrap shirt and knee-length skirt, a diver's knife at her belt, calloused bare feet | A tall carved driftwood staff; a rolled pilot chart under one arm | New. A person in working clothes: no costume shorthand, nothing the generic diver and fisher don't also wear |

Meg is drawn apart from the generic gunner by her hat, braid and
blunderbuss; Cass apart from the boarder (also big) by his twin's face, head
cloth and jerkin; Kesi apart from the generic diver and fisher by age,
staff and chart. Keep coats off burgundy and purple: they cut away with the
magenta background.

**KP pictures the story adds or changes** (re-briefed November 2026):

| Picture | Group | Why |
| --- | --- | --- |
| Vail's portrait | Set dressing, redo | Must show the **young** Ned Vail in a tricorn and a sea coat, painted badly in a heavy gilt frame, the brass plate under it gone and four screw holes left. Hob names him from it, so it must not look like the governor at a glance |
| Kesi's pilot chart | Documents, new | Opens the reef channel to its owner; must look unlike the kit's treasure map (a long narrow chart, soundings and a coastline, no X) |
| Duroc's signal pistol | Magic and tools, new | Fired for a marine; must look unlike the flintlock (brass, short, a wide bell mouth) |
| Abel's key | Magic and tools, new | Opens Cass's cabin; a ship's brass cabin key on a cord |
| Vail, the *Sovereign* | Creatures, named, new | In the table above |

**Added with the story's feature pass (17 November 2026).** The story now
uses every Classic rule the sample should show, which adds five KP pictures
and reuses kit ones for the rest:

| Picture | Group | Why |
| --- | --- | --- |
| Hob's whistle | Magic and tools, new | Clears the drowned from a room; a small carved bone whistle on a frayed cord, yellowed, unlike any kit item |
| Bottle of rum | Treasures, new | Paid to a tavern regular, who keeps it; a squat dark-green glass bottle with a cork, so it reads apart from the kit's rum flask (which heals) |
| Kraken-ink pool | Floor spaces, new | Lowers magic defense in the Hollow; a black pool with a purple-black sheen and a few ink tendrils, flat on the floor like the kit's lava vent |
| Meg's key | Magic and tools, recolour | Opens the parrot's cage; Abel's key recoloured to blackened iron (section 17) |
| Guardroom key | Magic and tools, recolour | Opens the diver's cell in Pardon Cove; Abel's key recoloured to dull steel on a ring (section 17) |

Reused, no new picture: Meg's parrot as a carried thing uses the kit's
parrot; Kesi's freed diver the kit diver; the healing chant the second
round's conch shell; the Constance's random creature the second round's
moray eel; the sea encounters the house guard (revenue man, tithe
collector), a crew captain (privateer), the navy marine (lieutenant), the
smuggler and the drowned crew; Vail's strongbox the kit chest; the bar
stool and the firing step the kit's boardwalk and flagstone spaces. The
three keys must still read apart at a glance, which the recolours do.

The three gate documents are now the letter of passage (the mansion; also
what Fort Teal and Ama Reef turn away), the governor's seal (the vault) and
the ledger (both endings), and with the pilot chart all four must read apart
at a glance. The pardon receipt is in no gate: the clerk's message mentions
one, so it can be the thing handed over with the reward purse, or be dropped.
The KP count goes from 25 to 29 (134 to 138 in all), and the feature pass
above takes it to 34 (143). With the four second-round kit pictures the
adventure uses (conch shell, moray eel, both ships) it needs 147, still
under the declared 160.

**Re-brief round made (17 November 2026)**, research `kp2/` (`_scripts/kp2.ps1`,
LoRA v5-s2200, three seeds each; edits with `qedit.ps1`, two seeds each).
Abel, Cass, Hob, Vail, Kesi, the key and the portrait came out right first
time. Meg, Jory and Duroc are edits of their first-round pictures, and Vail on
the *Sovereign* is an edit of the new Vail, so the two match. Picks are in
`kp2/picks`, snapped and tidied in `kp2/snap`, finished in `kp2/faces` (section
9), and in `final/` through `final-replace.json`, which drops the nine
first-round tiles they replace. What it took:

- **"Signal flare pistol"** drew a modern gold automatic; "an antique 1700s
  flintlock signal pistol ... wood and brass like an old musket" fixed it.
- **The pilot chart** first came out as a pale scroll, too close to the kit's
  scroll; naming the colours (tan coast, green jungle, blue sea, a red dotted
  course) and "wider than it is tall" made it read as a chart.
- **The portrait** kept its brass plate in every seed whatever the prompt
  said; a Qwen edit took it off.
- **Qwen edits:** "scorch marks" on Meg's coat came out as camouflage
  blotches and a powder smudge as a black hole in her face (a plain coat in
  one colour fixed it); "officer sabre" came out as a scythe until its length
  was given ("as long as her arm").
- **Snapping:** Abel's dark face and beard went to `outline0`, a black block,
  and Duroc's pale face to the kraken pink (section 5). Every face needs the
  section 9 pass, and the portrait a crimson fringe along its bottom edge
  removed.

**Kit, second round (planned November 2026).** The first 109 cover the list
above but leave gaps against what Kit 4 in [design](../design.md) promises
(ships as portals, a cursed compass, a mermaid's charm, a ghost crew) and
against the stories section 1 says the kit must serve, such as a ghost-ship
horror. Generate these after the hand-finishing pass and LoRA v4, except the
ship, which *The King's Pardon* needs sooner. About 40 pictures, so a pirate
adventure that uses the whole kit needs an Extended picture limit near 180.
The two ships are done: the anchorage square is the intact ship from the
first wreck attempt (section 11), and the portal ship asked for "a small
patch of blue water" (seeds that ran the sea to the edges cut out badly).

**Second round made (November 2026)** with LoRA v4-final (section 10), three
seeds each: the 38 others exist as raw picks, snapped and tidied. v4 held
the style across the set, with people full-length and a flat magenta
background throughout. Only the two ghosts needed a new prompt: "a
translucent pale blue-green man" came out as a faceless pale blob, fixed by
"drawn with a dark teal outline ... a clear face, two glowing white eyes"
and naming the clothing details. Faces (section 9) were placed on the 11
people who needed them (nine crew, the drowned captain, the mermaid); v4
already drew some eyes, so half the edits only thinned doubled or stray
eye pixels. The ghostly lookout was made and then dropped, keeping
creatures at 46.

**Re-rolls (November 2026)**, LoRA v4-final, four seeds each. The black spot
was a black disc on white and read as a hole; "a small torn scrap of
yellowed parchment ... a smudged black ink circle ... scrawled handwriting
underneath, lying tilted" reads as paper. Saying "playing card sized" drew
aces. The rigging was thin and wide at 64 px; "a compact square section ...
a thick tarred rope net of large square meshes ... as wide as it is tall"
fixed it. Qwen-Image-Edit fixed the other two: the double door filled its
whole square, so no wall showed ("make the pair of doors and their frame
narrower ... plain flat magenta on both sides"), and the anchorage had
island grass in its corners ("replace the green grass and sand of the
island corners with the same calm turquoise sea water").

**Outside the list, first drafts (November 2026).** The *Magpie* (player
picture): a single-masted sloop, black hull with a white band and a
black-and-white striped sail, so it reads apart from the portal ship. Its
patch of water is cut away, since a map marker sits on any terrain. Title
art: the sloop sailing toward an island with a fort and a harbour town, at
1344 by 768, with no lettering (the engine draws the name, which isn't
settled yet), and open sky in the upper third for it. Two of four seeds came
out photographic. The title is not snapped to the ramps: they have no sky
blue, and snapping turned the sky to grey speckle. Instead it is shrunk to
336 by 192 and posterized to its own 32 colours with no dithering, which
gives flat bands like the kit. 16 and 24 colours lost the island's green.
The UI font is in section 12.

**The title's sloop is the *Magpie* (November 2026).** The draft's boat had
a cream sail and a brown hull, so title and map marker showed two ships.
Qwen-Image-Edit 2511 repainted it ("change only the small sailing boat: a
black hull with a white band along its side, a black and white striped sail
in horizontal bands, a small teal pennant ... keep everything else exactly
the same"); all four seeds striped the sail, two striped the hull as well.
The edit also redrew the rest of the picture slightly (thousands of pixels
moved), so only the boat is kept: pasted into the original through a soft
mask of what changed in the boat's box. At 336 px two hand fixes: the hull
rows around the white band to one black, so one band shows, and the cream
wash under the hull (the old sail's reflection) to light sea, since under a
black hull it read as a second band. Shrink and posterize are as before (a
box filter, median cut, no dither).

| Group | Pictures | Contents | Why |
| --- | --- | --- | --- |
| Ships | 2 | A ship at anchor (map place square), a ship as a portal thing | Ships are the portals between islands |
| Documents | 1 | Treasure map | The classic quest object; the kit has only a map table |
| Magic items | 8 | Cursed compass, conch shell, voodoo doll, tiki idol, the black spot, ghost lantern, crystal skull, mermaid's comb | Spells hang on things (364 magic spells and 134 magic items in the corpus); these carry any of the 15 actions without leaving the setting |
| Treasures | 5 | Loose gems, jewelled idol, jewelled crown, pearl necklace, gold chalice | A quest goal must look unlike every other treasure |
| Creatures, supernatural | 4 | Ghost pirate (pale, glowing eyes on the emissive layer), drowned captain (boss), skeleton musketeer, skeleton captain | The supernatural ramp (section 2) has one skeleton so far |
| Creatures, sea | 5 | Kraken head (boss), giant octopus, jellyfish, moray eel, mermaid | Mermaid written as a person with her own aims, like the island peoples |
| Creatures, land and cave | 6 | Caiman, giant bat, monkey (a thief), wild boar, giant spider, island jaguar | Swamp, cave and jungle have no creatures of their own |
| Crew | 9 | Captain (man and woman), quartermaster, cook, surgeon, navigator, cabin boy, powder monkey, bosun | The roles players most want to be, and the obvious bosses |
| **Total** | **40** | | Creatures reach Classic's 46 (a ghost ship's lookout was made and dropped to fit) |

Left out on purpose: more terrain (16 is the Classic limit) and food or
mundane items (no ACS rule uses them).

**KP objects (November 2026):** the 17 non-character KP pictures exist as
raw picks, made like the kit (LoRA v3, three seeds each). Two needed a new
prompt: a pink pearl in a pink shell vanished into the magenta background
(keep subjects off pink and purple, or they go with the background when
cut out), and a "brown sailcloth" log looked like the kit's book (a KP
object must differ from every kit picture in colour as well as shape). The
portrait is a Qwen-Image-Edit of Vail's own picture ("a head and shoulders
portrait of this same man inside a gilded frame, same flat cel-shaded
style"), so the two match.

**The named eight (November 2026)** were made from the first briefs (since
replaced, above), three seeds each. Five came out right first time. LoRA v3 puts a tricorn on
anyone called a pirate, whatever the prompt asks; Harrow's wide hat and
Duroc's bicorne came from a one-property Qwen-Image-Edit ("replace only her
hat ... keep everything else exactly the same"). Cass's yellow headwrap
read as blonde hair until the prompt said "all her hair wrapped up and
hidden in a bright yellow cloth headwrap tied in a knot on top", and then
snapped to lime until the gold ramp got its lemon-yellow step.

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

**Purple subjects.** The cutter also drops magenta anti-aliasing. It once
did that everywhere in the picture and so cut the kraken arm's purple body
away, leaving outline and suckers; the snap hid it. Now magenta fringe is
dropped only within 3 px of the background, and anywhere else only true
magenta pockets (red about equal to blue, blue at least 0.4 of red: the
background caught between ropes or a pole and a leg). Purple is bluer and
crimson redder, so both stay.

**Tidy before hand-finishing.** `scripts/tidy.py PALETTE.json OUTDIR
IMG...` does the part of finishing a rule can: removes lone specks, gives a
stray pixel (no neighbour shares its colour, three of four share another)
its neighbours' colour, and adds the 1 px `outline0` outside every edge that
is not already dark, so figures keep their size. Faces, hands and held items
still need a person.

**Faces at 64 px (first pass, November 2026, 16 people).** A head is about
6 px wide and 6 tall; place pixels by coordinate on a zoomed grid, not by
eye on the full tile.

- **Eyes:** two `outline0` pixels on one row, with one or two skin columns
  between them for the nose. Raw tiles often have a dark band there instead
  (hat shadow, a brow line); thin it to two pixels or it reads as
  sunglasses.
- **Brows:** one pixel above each eye, one skin step darker, or grey for
  grey hair. Leave them out under a hat brim; on a 5 px face they make
  clutter.
- **Mouth:** one or two pixels, two skin steps darker, two rows under the
  eyes. Hidden in a beard, it can be left out.
- **Nose:** one pixel one skin step darker between and under the eyes.
- **Dark skin** needs nothing extra: `outline0` eyes read on the deepest
  skin steps, and a lighter step around them is not needed.
- **Pale skin snapped to the kraken pink** before the skin ramp had a pale
  step (section 5). Check every pale face for kraken pixels and swap them
  to the pale skin step, and the rest of the figure too: arms carried
  kraken-pink shading, which becomes the next skin step. No person uses
  the kraken ramp.

**Faces, last six (November 2026).** Two needed more than the recipe. Jory
Fenn's light brown skin had snapped to the leather ramp, the same as his
trousers, so skin and trousers merged; his skin is now the skin ramp.
Abel Quill's raw head was a patchwork of wood, leather and white that read
as a mask, so it was repainted pixel by pixel: dark brown skin, white hair
at the sides, a light stone pixel over each eye for the spectacles. Also
watch for:

- **A dark band under a hat brim** reads as sunglasses even with eyes
  below it; make it a skin-shadow step.
- **Red pixels on a bare scalp** read as blood; brows on a shaved head go
  one skin step darker, not the beard's red.
- **A white cravat right under the chin** reads as a goatee; give the neck
  a row of skin first.

**Faces and hands, fourth pass (November 2026, all 36 people).** The
first three passes placed eyes, but skin was still painted in other ramps:
whole faces in leather (the boarder, diver, house guard, bosun, cabin boy,
mermaid) or sand and wood (the captain, navigator, quartermaster),
rust-orange mouths, chins and ears that read as lipstick or blood, and
15 people's forearms and hands in flat bright rust. `skinaudit.py`
(research `_scripts/`) lists rust, leather and kraken pixels that touch skin;
bright rust below the head is almost always a hand. The rules now:

- **Skin is the skin ramp only.** Pick a base step for the person and remap
  the other ramp onto it step for step (leather 0, 1, 2 to skin 2, 3, 4 for
  brown skin), inside a box: the mermaid's rock is leather too.
- **Shade the side away from the light:** one step darker down the right
  edge of the face and of each forearm; a highlight pixel on a bald head.
- **Mouth** two steps darker than the base, one or two pixels, centred
  under the eyes; **nose** one step darker or none. No rust on a face.
  Undead keep their own ramp (moss for the drowned captain, bone for
  skeletons, whose sockets stay two pixels apart, not a band).
- **Under a hat brim,** the shadow row is one skin step darker, never
  leather or stone.

Edits are `handfinish4/_faces.json`, applied by `faces.py`; `assemble.py`
lays `handfinish4/done` over the earlier passes.

**Where the finishing work is.** In the first set of 35, nearly all of it
was on the 16 people: faces, skin colour, a speckled waistcoat, a held key
that read as a goblet. After `tidy.py`, 18 of the 19 creatures, objects and
weapons needed nothing (the crab got two eye-shine pixels). Budget hand time
for people.

**Non-people of the later rounds (fifth pass, November 2026).** All 41
checked at 4x: the 24 kit 2 objects and creatures and the 17 KP objects.
37 needed nothing. The four that did were faces again: Vail's portrait
(one eye a grey smudge, a leather stroke that read as a scar; redone to his
tile's face), the bat (eyes a dark band on its darkest leather, so a mask;
the face one step lighter with one eye pixel each side), the jaguar (gold
irises in gold fur; the brightest gold beside each pupil) and the boar (a
bright rust snout that read as a clown nose; now leather). An animal's eyes
need a value step against the fur around them, as a person's do.

**Faces of the re-briefed cast (sixth pass, 17 November 2026).** The nine
people of the re-brief round (section 8), with the same rules. Edits are
research `kp2/_faces.json` (`faces.py kp2/_faces.json kp2/snap kp2/faces`);
the 13 finished tiles are in `kp2/faces` and in `final/`. Hob's raised fist had
snapped to sand; it is on the skin ramp now.

- **Dark skin from a light raw:** Abel, Cass and Kesi came out light brown.
  The whole tile's skin moves two steps down the ramp (k2 to k4, then k1 to
  k3, then k0 to k2, in that order so nothing moves twice), and faces painted
  in wood go to k3. On a k3 face the `outline0` eyes read, but the face goes
  flat: one k2 pixel or two on the lit left cheek gives it shape.
- **The twins' beards** are `outline1`, the same as Abel's hat, with a row of
  cheek skin above Abel's so his beard reads as neat and Cass's as full.
- **Duroc** had a kraken-pink face (q0 to k0 over the whole tile) and a rust
  collar under the chin, now a row of neck skin above the white cravat.
- **Brim bands:** Jory's dark band under his straw hat and Meg's under her
  wide hat are now one skin step darker, so they read as shadow, not
  sunglasses.
- **Vail on the *Sovereign*** came out tan from the Qwen edit; his face is
  remapped to Vail's pale k0 and given the same features, so the two match.
- **The portrait** had a crimson cut-out fringe along its bottom and right
  edge: the edge rows are now `outline0`, and the rows under the frame are
  cleared.

**Faces and held items, seventh pass (20 November 2026).** After the First
Voyage playtest, all 28 kit people checked again, heads at 12x; 19 changed
(the 7 player pictures among them), and the three variants that follow
them (night deckhand, blue marine, cursed skeleton) were rebuilt. The
faults and fixes per tile are in research `handfinish6/NOTES.md`; the edits
are text patches there (`patches/`, rows of ramp codes at a corner, made
into `_faces.json` by `_patch.py`). What came up again:

- **Eyes need two skin columns between them** on a face 6 or more wide,
  and a skin pixel between an eye and the hair: an eye beside black hair
  vanishes into it. On a 5 px face, one column is the most there is.
- **Bands still read as masks:** a grey beard drawn dark across the lower
  face (the man captain), a black helmet rim or a fringe of hair over the
  eyes (house guard, thief), skull sockets run together. A beard is light
  with a mouth in it and shading on the right; sockets are two blocks.
- **Red at the chin or round a face still reads as blood:** a surgeon's
  rust stock, coral neckerchiefs, crimson hair round a pale face (now
  auburn, on the leather ramp).
- **Held items, drawn by hand:** at 64 px a weapon reads by a light upper
  edge, a dark lower one, an `outline0` round both and a gold guard or
  lock at the fist; a lump without those reads as a club. A pistol reads
  only as an L (barrel level, grip down from the fist); pointed down it is
  a stick. The figure holds its creature's weapon where the hands were
  empty or holding a lump (sabre, boarding axe, pistol, cutlass, dagger,
  a gaff's hook on the fisher's pole); role props that read (a cook's pot,
  a surgeon's bag, a trader's scales) stay.
- **Small contact sheets lie about faces:** a 1 px eye can vanish when a
  sheet is shrunk for viewing. Check a face by its pixel values or at 12x
  before calling it missing.
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

**v4 (November 2026), trained on finished pixel art.** The 35 hand-finished
64 px tiles (section 9), scaled up 16x with nearest neighbour onto flat
`#FF00FF`, so the LoRA learns finished pixel art rather than raw
generations. Captions as before, plus the full-body line on people, and they
describe the finished tile where it differs from the prompt (the cutlass,
Kesi's key); story names are left out. Same settings as v3, 70 epochs, 2450
steps, about 45 minutes. Installed as `acspirates/acspirates-v4-final` (and
`-s1400`).

Test (v3's 6 subjects, seeds 401 and 402, FP8 Turbo, short trigger prompt):

- **It draws pixel art at 1024**: hard-edged blocks about 11 px (not the
  16 px it was trained on, and not on an exact grid, so shrink with
  `sprite64.py` as before, not plain nearest neighbour).
- **Plants fixed**: the palm is flat pixel art, not painterly.
- **Flat background**: the final save gives near-pure `#FF00FF`; the
  1400-step save still varies the pink.
- **Closer to finished at 64 px**: on the 12 test tiles `tidy.py` cleaned
  141 strays and added 398 outline pixels, against 358 and 591 for v3.
- **Still drawn in**: a tricorn on anyone called a pirate, and faces that
  need the section 9 pass.

**v5 (November 2026), trained after the fourth face pass.** v4's 35 tiles
taken fresh from `final/` (16 of them people with new faces) plus the 20
people v4 never had (the kit 2 crew, the named characters, the undead), 55
images in all (`_scripts/dataset5.py`, research folder). Captions describe
the finished tile, not the old prompt: the smuggler carries rope, not a
sack; the surgeon has no spectacles left; the drowned captain's skin is
green. Same settings as v4, 50 epochs, 2750 steps, 37 minutes. OneTrainer
needs the preset under the config: `scripts/train.py --preset-path
"training_presets/Z-Image/#z-image DeTurbo LoRA 16GB.json" --config-path
lora/v5/train.json` (the config alone fails in a migration step). Installed
as `acspirates/acspirates-v5-final` and `-s2200`.

Test (v3's 6 subjects, seeds 401 and 402, FP8 Turbo, one LoRA per run):

- **Faces come out on the skin ramp.** With v4 both people carry
  rust-orange patches on cheek and mouth at 64 px (the habit section 9
  fixed by hand); with v5 s2200 both faces are clean skin with dark eyes and
  mouth. v5-final is close, but one officer still has an orange cheek.
- **Objects are unchanged:** boat, palm, turtle and pistols match v4 in
  style and background.
- SwarmUI's ComfyUI backend sees a new LoRA file only after
  `RestartBackends`; until then a request fails with "the backend does not
  have that LoRA". Pass one LoRA per `loratest.ps1` call: a comma list
  through `powershell -File` arrives as one string, stacking them.

Use v5-s2200 for new kit pictures.

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
each wall picture, one door square per wall. A door that is wide for its
height (the double door) can't fill the square, or no wall shows: it is
scaled by its own outline to 48 px tall and stood on the bottom edge
instead of centre-cropped. Two more floor fixes are
colour swaps too: the scorched floor goes from near-black to ash grey (dark
enough to read as burnt, apart from the lava vent), and the hull wall goes
one wood step darker so it stands apart from the plank floor.

## 12. UI font

**First version (November 2026).** An original pixel font, drawn glyph by
glyph for this kit and copied from no existing font or machine character
set. The source is [pirates-font.txt](pirates-font.txt), one block of `.`
and `#` per character, so a glyph is edited as text.
`scripts/font.py pirates-font.txt OUTDIR` builds an atlas (`font.png`), an
outlined atlas for text over pictures (`font-outline.png`, 1 px `outline0`),
`font.json` (glyph boxes, baseline, line height), a BDF and a preview.

- **Metrics:** cells 11 px tall: row 0 for accents on capitals, 8 rows
  above the baseline, 2 for descenders. Caps and ascenders 7 px, x-height
  5 px, line height 12. Proportional, 1 to 5 px wide plus 1 px between
  glyphs.
- **Characters:** printable ASCII and all of Latin-1 (191 glyphs), so
  French, Spanish, Portuguese, German and the Nordic languages can be
  written. Accented letters were composed once from base letter and mark,
  then written into the source, so each can be retouched alone. Accents on
  capitals sit in row 0 and touch the letter; lowercase accents leave a
  row free.
- **Look:** plain and readable first, since the log and menus are read for
  hours. Rounded capitals, a footed `l` and a short `t` keep it from
  reading as a terminal font. Digits are 4 px wide, so `0` stays apart
  from `O` (5 px).
- **Bold (display weight):** built, not drawn. Each glyph is laid over
  itself 1 px to the right, one px wider (`font-bold.png`). Used at 2x or 3x
  in `gold1` with an `outline1` outline, it is the title lettering.
- **Scaling:** integer only, like the tiles. At 64 px tiles, 2x gives 24 px
  lines.
- **Ink:** `outline0` on parchment (`sand0`), or `sand0` outlined in
  `outline1` on dark sea.
- **In the viewer (November 2026):** `--ttf-only` writes the font and its
  bold as outline TTFs, each pixel a square (`crates/acs-viewer/assets`),
  which the viewer draws at whole multiples of its pixel (client.md 7.1).

## 13. Mood ramps

**First version (November 2026).** Sections 13 to 16 are art made ahead
of the client: nothing in `crates/` reads moods, emissive layers, normal
maps or animation frames yet, and the bundle (content.md 2) carries only
`art/pictures.png`. "The engine draws" below says what a client would do.
Section 1 says darkness comes from the engine. A mood is a colour table, one entry for each of the 56 kit colours
([pirates-moods.json](pirates-moods.json)), so recolouring a region is a
lookup, with no filter and no new art. `scripts/mood.py` computes the tables
in OKLab from a few parameters per mood (lightness scale, contrast,
chroma, a tint, how much red survives, and a colour to mix toward), and
`--preview` shows pictures in every mood.

| Mood | What it does |
| --- | --- |
| Night | Half the lightness, cool blue tint, chroma down, a little night blue mixed in. Sand reads as moonlit grey, the sea near black. |
| Fog | Lights and darks pulled toward a pale grey, chroma halved: low contrast, nothing lost. |
| Cursed | Desaturated, green-yellow tint, reds cut, contrast up: sickly, still readable. |
| Underwater | Reds absorbed first (red scale 0.3), teal tint, a quarter of deep water mixed in. |

Glow is not part of a mood: lanterns, eyes and lava keep their colour from
the emissive layer (section 14). Change the parameters in `mood.py` and
rebuild the JSON; never edit the tables by hand.

## 14. Emissive layer

**First version (November 2026).** For each tile that gives light, its
glowing pixels as an image of their own: glow colour and strength, all else
clear. The engine draws the mood-recoloured tile, then adds this layer at
full brightness with a soft halo.
[pirates-emissive.json](pirates-emissive.json) lists the pixels: by kit
colour, optionally inside a box (only the flame, not the brass of a lantern
of the same gold), or one by one with their own colour. `scripts/emissive.py`
writes `<tile>.emit.png`, and with `--preview` shows day, the mood, and the
mood with glow.

Thirteen tiles glow: lantern, ghost lantern, the skeleton captain's lantern,
Hob Varrick's lantern (added with the re-brief round, with a flicker in
[pirates-anim.json](pirates-anim.json)),
the smoke rack's fire, lava vent, volcanic cracks, crystal skull, jellyfish,
compass needle, kraken eyes, the drowned captain's eyes and the ghost's
eyes. The ghost had no eyes at all; it got two dark pixels that read by day
and glow white at night. Glowing eyes are why an emissive pixel can have a
colour of its own.

The drowned captain's entry first lit his white cravat (`sand0` in a box
below the face), not his eyes; it now names the two eye pixels, as the
ghost's does.

## 15. Normal maps

**First version (November 2026).** `scripts/normals.py` makes a normal map
for every tile, with no painting. Height comes from lightness (the kit is
lit from the top left, so light faces are the raised ones) plus a 3 px
bevel inside a figure's silhouette, smoothed 1 px. The normals come from
its slope. Terrain and walls get lightness only. Under a torch at night,
flagstone gutters, barrel staves and coat folds catch the light, which is
all a 64 px tile needs. A tile that lights wrongly can be painted by hand
later: the engine only reads the image.

## 16. Animation

**First version (November 2026).** Made from the finished tiles by
`scripts/animate.py` ([pirates-anim.json](pirates-anim.json)), no new
drawing:

- **Water** (deep sea, shallows, the water floor): rows shift 1 px by a
  sine and wrap round. The period divides 64, so the tiles stay seamless
  on the map. 8 frames.
- **Fire and glow** (lava vent, volcanic cracks, lanterns, the smoke-rack
  fire): colour cycling on exact kit colours inside a box. 3 or 4 frames.
- **Flag:** columns of the flag move up and down, from still at the pole
  to 1.6 px at the free end. 6 frames.
- **Ships** (the portal ship, the *Magpie*): a 1 px bob. Full sail
  animation is left for hand drawing; at 64 px a bob reads as sailing.

Each animation is frames `<tile>-<t>.png` plus `<tile>.json` with the time
per frame. Frame 0 is the still tile, so an engine without animation shows
the same picture.

## 17. Recolour variants

**First version (November 2026).** A variant is a kit creature with one ramp
moved onto another ([pirates-variants.json](pirates-variants.json),
`scripts/variants.py`): every step of the source ramp lands on a run of
steps of the target, on exact kit colours, so the variant stays on the
palette and needs no drawing. A variant can also apply a mood to one tile:
a cursed skeleton is the skeleton in the cursed table. Each still costs a
picture slot.

| Variant | From | Swap |
| --- | --- | --- |
| Blue marine | Navy marine | Rust coat to sea blue: a rival navy |
| Blue parrot | Parrot | Rust to sea: a blue-and-gold macaw |
| Blue crab | Giant crab | Rust to sea |
| Fire serpent | Sea serpent | Sea to rust; its water becomes a lava pool |
| Rose octopus | Giant octopus | Rust to the kraken's rose steps |
| Cursed skeleton | Skeleton | The cursed mood |
| Night deckhand | Deckhand | The night mood, as a check that creatures keep their read at night |

The first octopus swap used the kraken ramp from its lightest step and came
out pale pink; starting one step darker gives a rose that reads apart from
the orange original.

## 18. Checks and packing

**Checks** (`scripts/kitcheck.py`, design art pipeline step 8): every tile
at 64 px and at 32 px, in greyscale, and through the three colour-blind
simulations (Machado 2009), plus a silhouette test. The silhouette test
compares solid silhouettes, feet on the baseline, by overlap (IoU), and
only for standing figures. Boxes, barrels and books share shapes by nature
and are told apart by colour.

First run (November 2026), 175 tiles: nothing to fix. Values separate
figure from ground in grey. Under deuteranopia, beach and grassland both
turn pale yellow but keep their lightness and pattern; deep sea and
shallows, reef and channel, and the three gate documents stay distinct in
every view. The closest people are the woman captain and the drowned
captain (IoU 0.86, both captains in long coats), told apart by the drowned
captain's grey-green. Whole-figure IoU sits around 0.8 for any two adults
of one height, so the test flags outliers, not the norm.

**Packing** (`scripts/pack.py`, step 9): one atlas of every 64 px tile
(PNG and lossless WebP, 88 KB for 220 tiles), emissive and normal atlases at
the same boxes, and a manifest. For each tile the manifest gives its atlas
box, source file, hand-edit file, layers and licence (CC BY-SA 4.0, as for
all art), and the generation chain: model, LoRA, seed and prompt, followed
back through Qwen-Image-Edit sources to the first generation. An exact pick
(`picks.json` keyed by the tile's own stem, as `kp2/picks.json` is for the
renamed re-brief edits) wins over a pick by picture name. Terrain and
room picks were made by scripts that kept no log; those point to their
pick and the generating script. Door squares point to their door and wall.

## 19. Hero idle loops

**Made November 2026 with Wan 2.2.** Design (sprite tricks) plans a few real
frames for hero creatures. `_scripts/hero.ps1` (research folder) runs Wan
2.2 I2V A14B (the official fp8 high- and low-noise pair) through SwarmUI.
The start image is the hero's raw pick, unchanged; boomerang makes the clip
loop. `scripts/frames64.py` then cuts every frame by one shared box and one
shrink factor, so the creature keeps its place and size, and snaps and
tidies each frame like the stills. 8 frames are kept from the loop.

What it took on a 16 GB card:

- **Small video, full steps, no step LoRAs.** With the lightx2v 4-step
  LoRAs it ran out of memory at every size, even 320 px with 9 frames.
  The traceback showed why: ComfyUI bakes a LoRA into fp8 weights on the
  GPU before sampling. Without them (20 steps, CFG 3.5) it runs.
  320 px is plenty when the frames end at 60 px.
- **Not the GGUF builds for now.** Wan 2.1 I2V 480p as GGUF fits, but the
  auto-updated ComfyUI's fused linear path skips GGUF dequantization (a
  Q5-packed 8640 against the real 13824 in the feed-forward layer). Fixing
  that means patching the backend.
- **Not Qwen-Image-Edit keyframes.** Each edit moved shading or broke
  shapes, and all of them shrank the head, so a loop of them flickers. Wan
  keeps colour and outline steady from frame to frame.

**Ask for big motion, and pin the colours.** At 60 px a sway of a few
video pixels is under one tile pixel: the motion vanishes and only shading
noise survives, which reads as pixels shifting, not as movement. The first
captain loop asked him to "stand still, sway very slightly" over 9 frames,
and that is what it looked like. The rerun names a large action (a heavy
breath, a side-to-side sway, coat tails billowing out and settling), drops
"stays still", and asks for 25 frames. The bigger motion made Wan lose
track of the character: on one seed his green hat turned black halfway
through. Naming the hat's colour in the prompt ("stays green the whole
time; his colours never change") held it on all four rerolls. Check each
take before cutting it: compare the mean colour of the top of the figure in
every frame with frame 0 (the black hat drifted by 56, steady takes by 2
or 3). Then pick the take by eye at 64 px; the take with the most motion
broke its coat into fragments.

**True loops, not boomerang.** Boomerang plays the clip forward and back,
which reads as mechanical. `hero.ps1 -Loop` gives Wan the start image as
the end frame too (SwarmUI's `videoendimage`, Wan first-last-frame), so the
motion goes out and comes home without reversing. Use 25 frames and
`frames64.py --drop-last`: the last frame returns close to the first but
not exactly. A looped clip takes about 30 minutes at 320 px on a 16 GB
card, so `hero.ps1` waits up to an hour; a request that times out still
finishes, in SwarmUI's `Output/local/raw/<date>/`.

Big motion breaks things, so check each take at 64 px:

- **Name what must not change,** colours and size ("stays the same size,
  its colours never change"), and ask for no splashes: a kraken asked to
  "swell and splash" shot a column of water through its own body.
- **The shared box shrinks the hero** when any frame reaches wide (the
  kraken's tentacles to the image edge): pick takes whose motion stays
  inside the still's footprint.
- **Count the eyes.** One kraken take kept only one eye at 64 px; the
  chosen one lost an eye in one frame, put back by hand.

Three heroes have true loops (November 2026): the drowned captain breathes
and swings his arms and coat wide (230 to 670 pixels change per frame, seed
811; boomerang loops were 130 to 240); the mermaid tosses her head, her hair
swings out and her tail sweeps (150 to 890, seed 811; was 10 to 50); the
kraken curls its tentacles in a steady breath (200 to 420, seed 811). The
kraken's boomerang loop moved more (400 to 900), but the bigger looped
takes broke it. The old loops are kept as `hero/_*-boomerang`. The shared
box must hold the widest frame, so a hero comes out a little smaller than
its still tile; frame 0 is not the still.

**Faces in the loops.** Wan starts from the raw pick, so a loop's frames
never get the still's hand-placed face: the mermaid's loop had no eyes at
all. `_scripts/facepaste.py` (research folder) finds the head in each frame
by the offset that best matches the finished still's skin pixels (skin and
moss ramps, after the remap, so swinging hair doesn't pull the match), then
copies the still's face box into it, outline pixels excepted unless named
as eyes. It can repeat a ramp remap above a given row. A frame that matches
badly (the head turned or tipped back) keeps its own face, and its eyes and
mouth are placed by hand in `hero/_loopfaces-<hero><seed>.json`.
`_scripts/loopcut.sh HERO SEED` runs the cut, the paste and the hand edits
in one go; run it whenever a hero's still face changes.
