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
| Proportions | About 4.5 heads tall; slightly large head and hands. No chibi, no realistic anatomy. |
| Height | Every adult human the same height; the boss and big creatures may exceed it. |
| Pose | Standing, front-facing (a slight three-quarter turn is fine), full body, feet on the baseline. |
| Face | Always drawn: small dark eyes (dots or short ovals), visible brows, nose and mouth. No anime eyes, no blank faces, no realistic skin. |
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

**Character prefix:**

> Hand-painted storybook adventure game character, cel shading with two or
> three flat tones, cool shadows and warm highlights, light from the top
> left, clean dark brown outline. Full body, standing, facing the viewer,
> about four and a half heads tall, slightly large head and hands, simple
> friendly face with small dark dot eyes, eyebrows, nose and mouth. Golden
> Age of piracy, around 1710.

**Character suffix:**

> Feet on the ground at the bottom of the figure, whole figure visible with
> margin, isolated on a plain flat solid magenta background, nothing else in
> the image.

Never name pirate games, films or their characters in a prompt.

**What the first character batch taught** (6 roles × 8 seeds):

- The template holds proportions, outline, light and faces well across seeds;
  the figures read at 64 px.
- **The model defaults to light skin and one look per role.** Every crew
  member came out light-skinned, and every navigator bald, heavy and in a
  grass skirt. State skin tone, age, build and hair in each subject line,
  and vary them across the kit.
- **Unspecified captains drift to famous film looks** (braided hair, beard,
  red bandana; the skeleton picked them up too). Describe hair, hat and
  face explicitly so the captain is ours.
- Some seeds frame the figure as a card with a white border; drop those.

## 8. Consistency

1. Pick one reference figure from a character batch: the model sheet.
2. Make other characters from it with Qwen-Image-Edit 2511 ("same style and
   proportions, now a navy marine with a musket"), not from new seeds alone.
3. Hand-finish about 30 tiles to this sheet, then train the kit LoRA on only
   those (design, art pipeline step 3).
4. Check each batch against the sheet: same height, outline colour,
   light direction and face style.
