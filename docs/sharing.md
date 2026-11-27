# Sharing old adventures

How an adventure made with *Adventure Construction Set* back then can
be played by people who don't have the disk, and what it takes for
ACSVM to offer one for download. Plan and policy: the remake tool in
section 5 is not built yet. Licensed CC BY 4.0.

## 1. What is in an adventure

An ACS adventure is one file that holds everything it uses: the world
map, regions and rooms, the things and creatures with their names and
ratings, the messages, and every picture. Music is the exception: an
adventure stores only cue numbers, and ACSVM plays its own cues for
them (client.md 6).

The pictures are the catch. ACS came with three builder sets (fantasy,
spy and mystery, science fiction) of stock pictures and stock things
and creatures. An author usually started an adventure from one, and its
pictures were copied into the adventure, where the author could keep
them, change them or draw over them. So most fan adventures are part
the author's work and part EA's: the story, map, rooms and messages are
the author's, and many of the pictures, and some names and ratings, are
EA's stock.

An author can license what they made. They can't license EA's stock
pictures, even though those pictures sit on their disk.

## 2. Playing an adventure you already have

Nothing on this page is needed for that. Open the disk image or file
and ACSVM plays it; the file never leaves your computer. HD packs
(client.md 5) can show new art over the old pictures, matched by a hash
of each picture, so a player can see stock pictures redrawn without
anyone distributing the adventure.

## 3. Three kinds of adventure

| The adventure | Can ACSVM offer it? |
| --- | --- |
| **All the author's own.** Every picture drawn by the author from a blank tile; names, ratings, text and story theirs. | Yes, with the author's permission (section 4), converted to a bundle as it stands. |
| **Built on a builder set.** The author's story, map and text, with some or all stock pictures, things or creatures. | Yes, as a remake (section 5): the author's work kept, EA's stock replaced with our own art and lists. |
| **Built on someone else's world.** Tolkien's Middle-earth, a comic's characters, a film. | No. The author can't license characters and worlds that aren't theirs, and a remake doesn't change that. |

A picture the author started from a stock picture and then changed
counts as stock: it is still derived from EA's art. Only pictures drawn
from scratch count as the author's.

## 4. Permission

An adventure is offered only with its author's written permission,
recorded in `docs/provenance.md` with where and when it was given. We
ask for:

- **A licence.** CC BY-SA 4.0, the licence of our own adventures, so
  anyone may share and change it with credit. An author who wants
  another free licence can say so, and we'll check it fits.
- **Which pictures are theirs.** The author tells us which pictures
  they drew from scratch, and which are stock or started from stock.
  When they don't remember, we treat a picture as stock if it also
  appears in other authors' adventures.
- **Credit.** How they want to be named in the bundle's byline and
  credits (content.md 2).

An author page that already lets anyone redistribute and change the
files counts as permission for the author's own work, as for RoePipi's
*Coco's Adventures* (provenance.md). It never covers the stock pictures
on the same disk.

## 5. Remakes

A remake is the author's adventure as an Extended bundle (`.advx`,
content.md 2), with EA's stock taken out and ours put in:

1. **Read the adventure** to the canonical model (`acs dump`).
2. **Swap the stock pictures.** Each stock picture is replaced by a
   picture from one of our kits that shows the same kind of thing: a
   wall for a wall, a skeleton for a skeleton. The kit's art is drawn
   fresh from a written brief of the subject, never traced from or
   checked against the stock picture (design.md, "Our own spy"). The
   author's own pictures stay as they are.
3. **Swap stock names and ratings.** A thing or creature still carrying
   a builder set's name and ratings takes our kit's, or new ones chosen
   with the author. Ones the author named and rated keep theirs.
4. **Build the bundle** with the author's credits and licence, and our
   kit's credits for the art (`acs bundle`).
5. **Check it.** A remake ships only when no picture in it matches a
   stock picture, and someone has played it to the end.

The list of which stock picture becomes which kit picture is made from
local copies and stays outside the repository, with the corpus
(art/fantasy-creatures.md 1). Only the finished bundle, which holds the
author's work and ours, is published.

**What it needs.** A kit for each builder set's genre. The pirate kit
exists; the fantasy kit is started, with its 46 creatures briefed in
[art/fantasy-creatures.md](art/fantasy-creatures.md) for the first
remakes, RoePipi's two *Coco's Adventures* disks. Steps 2, 3 and 5
need a tool that isn't written yet.

**A remake looks different.** The map, rooms,
story and rules are the author's, but the stock pictures are new
drawings, so a room can look quite unlike the one remembered. The
original file, for anyone who has it, still plays with its own pictures
(section 2).

## 6. Sharing yours

If you made an ACS adventure, or know who did, open an issue or get in
touch. Tell us what it is, which computer it was made on, and as much as
you remember about which pictures you drew yourself. Please don't
attach the disk to a public issue: we'll arrange a private copy, and
nothing is published until the steps above are done.

## 7. Open questions

- **Stock picture hashes.** An HD pack for the builder sets lists a hash
  of each stock picture. A hash names a picture without containing it
  (design.md, "Render modes"), but so far no hash of an EA picture has
  been committed (client.md 10). Whether a builder-set HD pack can be
  published, or ships only as art for players to pair with a local list,
  is the maintainer's call.
- **Builder-set lists.** How close a remake's names and ratings may stay
  to a stock thing's (a "dagger" with ordinary dagger ratings) before it
  counts as copied.
