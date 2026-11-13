# Rules: creatures and turn order

Behaviour spec for creatures outside combat: when they act, how a turn goes
round, random room creatures, and the first slice of the AI (goals, paths,
doors, picking up gold and treasure, the overburdened drop, readying
weapons). Licensed CC BY 4.0. The engine implements it in
`crates/acs-engine` (`creatures.rs`, `ai.rs`).

Tags as in [movement](movement.md): **[manual]** paraphrased from the
manual, **[data]** from the stored format or corpus counts, **[designer]**
from Baber's Jadeworld notes, **[unknown]** with the engine's current choice
as **Engine:**. Counts cover all 15 corpus adventures.

Combat is not in this slice. Where a creature would attack, the engine only
reports it (`CreatureConfronts`) and the creature's turn ends.

## 1. Who is in a region

1. A region has 16 resident creature slots, each a full creature record that
   names its room and square. [data] No more than 16 creatures occupy a
   region at once; past that no random creature appears and no summons are
   answered. [manual] **Engine:** the 16 slots *are* the region's
   creatures: a random creature that appears takes a free slot, one that is
   removed frees it, so the limit is the slot count. Extended adventures
   may have more slots (`limits.residents_per_region`) and add a per-room
   cap (`limits.creatures_per_room`); Classic is unchanged.
2. A creature stays in its region: it takes only doors that lead somewhere in
   the same region (section 5). [manual]
3. Creatures on the world map exist only as random encounters, which still
   wait for encounter areas (movement 7.1). [unknown]

## 2. Turns

1. Residents get no turn until a player enters their room. The player moves
   first; the creature then gets its turn even if the player has left the
   room meanwhile, which is how creatures follow players through doors.
   [manual]
2. Turns cycle through every player and every creature in the room.
   [manual] On the C64 the program schedules by **region** (the world map
   is region 0):
   1. A **cycle** marks every region where a living player stands, then
      visits the marked regions, lowest number first, so the world map
      first. A region a player reaches during the cycle waits for the next
      one.
   2. A **visit** is at most 10 **rounds**. It ends sooner after a round in
      which no living player is left in the region, or in which the number
      of living players went down.
   3. A **round's turn list** is built at its start. On the map: every
      living player there, in joining order. In a room region: the rooms
      that hold a living player (at most 4, in player order), and in each,
      its players in joining order, then its creatures in slot order.
      Turns go in list order; the dead are skipped and nobody is added
      during the round. So a creature takes one turn a round, after the
      players of its room, and a room counts only if a player stands in it
      when the round begins; a creature already on the list takes its turn
      even if the player has left. Anything asked for during the round
      (adding or retiring a player, saving) happens after it.
   [research: turn-order first pass, 2026-11-12, `docs/research/turn-order.md`] Confirmed: with T in a room with three creatures and U
   on the map, the scheduler's region and round counter, read at every
   turn's start, showed U alone for rounds 21 to 39 (the rest of one map
   visit and a whole one: 19 turns), then T, AA, BB, CC for rounds 40 to
   49, U for 50 to 59, and the room again from round 60, in both runs.
   [capture: turns --peek, capture.md 10.13] And by name strip: T, then
   slots 0, 1, 2 (in slot order, not by Speed, distance or name), then T,
   in 17 phases of 6 runs and 9 of 3 with the slots swapped; T, U, then
   the creatures in 21 phases of 7 runs, never after T alone; and the
   first creature turns after both players had walked in only once both
   stood in the room at a round's start. [capture: turns, capture.md
   10.10] **Engine:** the rule above. A player who dies mid-round ends
   the visit at the round's end; the "restore the last saved game" screen
   the original then shows is the front end's business.
   Two players in different rooms of one region: T in the side room, U
   and two creatures in room 0; every round from the region's second on
   went T, then U and the creatures, in 3 runs. [capture: turns, case
   rooms2, capture.md 10.10.4] But in the region's first round T had no
   turn: U and the creatures only. T had come in through the map door in
   the map visit's last round (with or without going on through the
   portal), U a round earlier; taking the portal later, inside the
   region's visit, cost T no round (3 and 3 runs). [capture] [unknown]
   **Engine:** lists T as usual. Not settled: that missed round (the
   turn-order brief), the individuals the researcher saw numbered 5 to
   15 on room lists.
3. So creatures act only while a player stands in their room at a round's
   start, and move once a round however many players share the room. Two
   players in different places do not alternate: each gets its region's
   visit of up to 10 rounds in turn. [research, capture]

4. **Random creatures fall away.** Random creatures in rooms no player
   occupies are removed now and then. [manual] On the C64, after each round
   in a room region one roll of 0 to 15 picks a slot; if it holds a
   creature, one percentile roll follows, and under 20 every random
   creature of the region (resident ones never) in a room with no player
   in it goes. So a left-behind random creature goes with chance (slots in
   use / 16) x 20% a round. [research: random-rolls third pass,
   2026-11-12, `docs/research/random-rolls.md`] The researcher read the
   percentile as one per random creature whose room was not on the
   round's list. Captured with every draw logged: the percentile is
   drawn once whenever the slot is in use, with no random creature left
   too, and a random creature goes at the end of the very round in which
   the last player left its room, though that room was on the round's
   list. So what counts is the room being empty at the round's end.
   In 256 runs from one snapshot,
   each with its own random state, the rule predicted all 1,280 round
   ends' draws and the 114 removals, 27 of them at the end of the round
   the player left in; the researcher's reading got 154 runs wrong. [capture: mem --draws, case fall,
   capture.md 10.14.3] Not captured: two random creatures at once (one
   roll or one each); a player walking into a room mid-round (by the rule
   above it protects the room's random creatures). **Engine:** the rule
   above, at the end of each round of a room region's visit, the removed
   creature taking what it carries (`CreatureRemoved`, reason
   `left_behind`); what becomes of its things in the original is not
   known.
5. **The movement bar.** A creature's turn has a bar like a player's: its
   Speed after the weight slowdown ([inventory](inventory.md) 3.3), using
   the creature's own Strength, Constitution, Life Force and load. On the
   C64 it shows on the left as a player's does, 8 pixels a point of Speed
   (24, 40 and 32 pixels for Speeds 3, 5 and 4, carrying nothing), and
   drains as the creature walks. [capture: turns] The slowdown part is
   [unknown].
   **Engine:** so. A creature with Speed 0 does not move; the manual only
   adds that it does not show its name on its turn. [manual]
6. **Standing next to an aggressive, unfriendly individual** shortens a
   player's bar. [manual] By how much is not given. On the C64 the bar is
   halved, and only for a creature on one of the 4 squares beside the
   player (north, south, east or west), not diagonally: a Speed 12 player
   (96 pixels) on the room's entry square showed 48 at the top of each of
   its next turns with an aggressive enemy or an aggressive neutral (Speed
   0) on the square east of it, and 96 with the same enemy diagonally next
   to it or two squares east, with an aggressive friend or a peaceful
   enemy east of it. [capture: turns, the adj- cases, 2 runs each,
   capture.md 10.10.2] **Engine:** at the start of a player's turn, if any
   creature on the 4 squares beside the player is Aggressive and not a
   Friend, the bar is halved (after the weight slowdown, rounding down,
   at least 1; Speed 0 stays 0). The `TurnStarted` event says so
   (`hindered`). Applied at turn start only; whether walking up to such a
   creature mid-turn shortens the rest of the bar is not captured, nor
   how odd Speeds round ([unknown]).

## 3. Random room creatures

1. On entering a room there is one roll against the room's chance for its
   random creature (movement 7.2). [manual] On the C64 the chance is 5% a
   point of the room's 0..15 setting, and the roll comes once a player has
   walked in: through the map door the creature showed as the room was
   drawn, through a portal into another room (which ends the turn,
   [movement](movement.md) 2.3) as the next round began. With the chance
   at 15 (75%) one appeared in 8 of 10 walks in from the map and 6 of 10
   through a portal. [research: random-rolls third pass, 2026-11-12, `docs/research/random-rolls.md`] [capture: turns, cases rc-map and
   rc-side, capture.md 10.14] The researcher read the roll as taken each
   round as the list is built and counting only while every player in the
   room has just arrived; the map door case shows it is not lost when the
   arriving player's turn goes on. **Engine:** one percentile roll against
   the chance when a player enters a room.
2. It appears only on a square with no thing on it. [manual] On the C64
   up to 5 tries: a random column and a random row of the room's inside;
   a square with an individual or a thing is rejected; after 5 rejections
   nothing appears. [research] The squares seen were spread over the
   room's inside. [capture] **Engine:** the same.
3. It is a copy of the region's random creature record named by the room
   (one of 8), with the random mark, put in the lowest free slot, and it
   takes its turns in the room's list like any creature. [research;
   capture: slot 0 with the mark, 16 of 16 appearances] **Engine:** event
   `RoomEncounter` with the slot and square.
4. The roll is skipped when all 16 slots are in use (1.1).

## 4. Personality

1. The strategy line has three settings: Aggressive or Peaceful, Brave or
   Cautious, and Friend, Enemy, Thief or Neutral. [manual] All three are in
   the record. [data]
2. **Fighter or slinker.** A creature that is Aggressive and either Brave or
   unwounded (Life Force at least Constitution) behaves as a fighter. One
   that is Peaceful, or Cautious and wounded, behaves as a slinker. Slinkers
   never attack or cast. [manual] Worked out afresh at the start of each of
   its turns. [unknown]
3. Placed residents by type at the start (all 15 adventures, 3,248): 1,985
   fighter enemies, 632 fighter friends, 494 fighter neutrals, 105 fighter
   thieves, 16 slinker friends, 9 slinker thieves, 7 slinker neutrals and no
   slinker enemies. 6 have Speed 0. [data]
4. **Smart or dumb** (Wisdom above 14, below 6, chance in between) matters
   for casting and for a hurt neutral turning enemy: the magic and combat
   rules. Not used here.

## 5. Where a creature may step

1. Creatures never move onto a space or an obstacle, custom ones included.
   [manual] **Engine:** nor onto walls, stores, or another individual.
2. **Things it won't pick up.** Creatures never pick up magic items,
   weapons or armor; only thieves pick up treasure; all creatures except
   friends and slinker neutrals pick up gold. [manual] An AI will not walk
   onto a magic item. [designer] **Engine:** a square whose top thing is
   carriable but not one this creature picks up (spells included) is closed
   to it, and so is one it picks up but is at the carry limit for. A square
   whose top is a floor or nothing is open. Things lower in a stack do not
   matter (movement 5.3).
3. **Doors.** Creatures move onto a door only if it has no do-all, rid-room
   or add-to-room spell and it leads somewhere in the same region. [manual]
   **Engine:** a portal whose destination is a room of the same region (a
   deleted one included, see 5.5); its access applies to the creature as to
   a player (owners of / non-owners of, with the needed thing used up if the
   door destroys it); an "invoke spell" door is closed to creatures when the
   spell is Activate All, Rid Room or Add to Room. Other spell doors let them
   pass and the spell goes off for them ([magic](magic.md) 6.6). [unknown]
4. **Taking a door** costs one step and ends the creature's turn. It arrives
   on the door's destination square. Baber notes that a creature following a player can share
   the player's square for a moment. [designer] On the C64, a friend of
   ours (Speed 6) that followed the player through a portal into another
   room vanished from the first room on the step onto the portal, its turn
   ending there, and showed in the new room on its next turn on the
   portal's destination square, on top of the player standing there; it
   then stepped off. [capture: turns, case follow, 2 runs, capture.md
   10.10.3] With a creature of ours (Speed 0) standing on the destination,
   the player landed on it too, and the friend that followed landed there
   as well, three on one square, then stepped off (6 runs, the friend
   following in 5). [capture: turns, case held, capture.md 10.10.5]
   **Engine:** taking a door costs the step and ends the creature's turn;
   it arrives on the destination square whoever stands there, sharing it
   until they move. The original keeps the creature out of sight until
   its next turn; the engine places it at once.
5. A creature sent through a door into a deleted room is banished.
   [manual] **Engine:** removed (`CreatureRemoved`, reason `banished`) with
   what it carries.
6. Moves are one square to one of the 4 squares beside it, never
   diagonally (6.5). [research, capture]

## 6. Goals

1. On its turn each type looks for its targets in a fixed order: [manual]

   | Type | Seeks, in order |
   | --- | --- |
   | Fighter enemy | player; aggressive friend; peaceful friend; exit; aggressive neutral or thief; peaceful neutral or thief; gold |
   | Slinker enemy | peaceful enemy; aggressive enemy; exit; gold |
   | Fighter friend | aggressive enemy; aggressive neutral; peaceful enemy; aggressive thief; player; exit |
   | Slinker friend | player; exit |
   | Fighter neutral | aggressive friend or enemy; aggressive thief; peaceful friend or enemy; peaceful thief or aggressive neutral; peaceful neutral; gold; exit |
   | Slinker neutral | exit |
   | Fighter thief | treasure or gold; exit |
   | Slinker thief | treasure or gold; exit |

2. **Nearer beats better** and a wounded slinker seeks an exit. [manual]
   On the C64 a creature chooses **one goal at the top of its turn** (after
   the healing die) and keeps it all turn. Every candidate scores its
   **rank plus the steps** from the creature's square (columns apart plus
   rows apart, walls ignored); the lowest score wins, a tie going to the
   one looked at first (individuals by number, players first, then things
   in the room's order); nothing under 200 means no goal. Candidates are
   the other individuals standing in its room, and exits, gold and other
   treasure on its squares. A player who has left the room is not one, so
   a friend left behind goes for an exit (often the same door). The
   wounded slinker's exit is just its type's ranks: a cautious creature
   that is wounded becomes a slinker. [research: random-rolls third pass, 2026-11-12, `docs/research/random-rolls.md`] Confirmed: with each
   draw logged, the goal and step rules predicted every step of a
   peaceful friend (the player against a portal) and a peaceful neutral
   (the portal, or no goal) in all 64 runs from each of two rooms, 1,367
   steps. [capture: mem --draws, capture.md 10.14]
3. **Ranks of individuals** by the creature's type (rows) and the other's
   alignment and setting (columns; A aggressive, P peaceful); lower is more
   wanted, "-" never: [research]

   | Type | Player | A friend | P friend | A enemy | P enemy | A neutral | P neutral | A thief | P thief |
   | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
   | Fighter enemy | 0 | 4 | 6 | - | - | 80 | 84 | 80 | 84 |
   | Fighter friend | 32 | - | - | 0 | 4 | 3 | - | 5 | - |
   | Fighter neutral | 1 | 0 | 4 | 0 | 4 | 6 | 8 | 2 | 6 |
   | Fighter thief | - | - | - | - | - | - | - | - | - |
   | Slinker enemy | - | - | - | 29 | 25 | - | - | - | - |
   | Slinker friend | 26 | - | - | - | - | - | - | - | - |
   | Slinker neutral or thief | - | - | - | - | - | - | - | - | - |

4. **Ranks of things** (an exit: a door it may take, 5.3; gold and other
   treasure it would pick up, 5.2): [research]

   | Type | Exit | Gold | Other treasure |
   | --- | --- | --- | --- |
   | Fighter enemy | 96 | 128 | - |
   | Fighter neutral | 96 | 96 | - |
   | Fighter friend | 96 | - | - |
   | Fighter or slinker thief | 96 | 32 | 32 |
   | Slinker enemy | 32 | 96 | - |
   | Slinker friend or neutral | 32 | - | - |

   So a slinker friend takes an exit over the player only when the exit is
   at least 7 steps nearer; a fighter enemy an aggressive friend over the
   player at 5 steps nearer. Where these differ from the manual's lists
   (a fighter enemy puts neutrals and thieves above an exit; a fighter
   neutral seeks players), the numbers are what the C64 does.
5. **Each step** looks at the 4 squares beside the creature, in the order
   south, north, east, west, and takes a roll of 0 to 2 for each open one
   (5.1, 5.2). It steps to the open square with the lowest **3 x the steps
   from it to the goal + 5 x its cost + 4 x its roll** (a tie to the first
   looked at). With no goal it steers by the square it started the turn
   on, so it wanders about there. The cost: 1 for the goal's own square;
   an exit 1 for a slinker, 3 for a fighter; gold or a treasure it would
   take, its thing rank / 32; 4 for a square it has stood on this turn
   (its starting square too); 2 for any other. One step nearer saves 6
   and the roll adds 0, 4 or 8, so it often steps sideways or back: that
   is why paths differ from run to run. It steps every move of its bar,
   **even beside its goal**, unless an action ends its turn (7) or no
   square beside it is open, when it stands and waits the bar out (each
   idle move taking magic 5.3's healing roll). [research, capture]
   When the square it picks holds gold or a treasure it takes, it takes
   the thing from where it stands, without stepping onto it, and its turn
   ends (7.5). [capture] Confirmed with things: a peaceful thief and an
   aggressive thief going for treasure and gold, a peaceful enemy for
   gold, two treasures at the same score going to the first in the room's
   order, every step and take of 64 runs, 598 steps. [capture: mem
   --draws, case thief, capture.md 10.14.4]
   Not settled: on a turn's first step some squares holding a thing cost
   4; one more case in which the step choice gives up. [unknown]
6. **Engine:** 6.2 to 6.5 as written. Before the goal, a smartness roll
   decides whether a creature acts smart (magic 6.1). On the C64 that roll
   is taken at the top of the turn by friends, enemies and every fighter
   (4.2; a fighter thief too, [capture] 10.14.4),
   whether or not it has anything to cast; the engine rolls it only when
   it has.

## 7. Reaching a goal

1. **A foe.** A fighter loves to attack certain types and may attack others
   it meets on the way. [manual] How the C64 decides is for the combat
   pass. An aggressive, brave enemy of ours with no weapon never attacked
   the player beside it: it kept stepping round it (6.5) in 32 of 32 runs,
   its turn taking one more roll than a slinker's. [capture: fight,
   capture.md 10.14.6] The same enemy holding a melee weapon of ours
   attacked, several times a turn ("EE misses T", a message each), in
   every run. [capture: armed, 10.14.7] **Engine:** a fighter holding a weapon, beside (one
   of the 4 squares) the individual it chose, emits `CreatureConfronts`
   when that individual is a foe (7.4), and its turn ends; combat will
   turn this into an attack. Without a weapon it steps on as 7.2.
   [unknown] past the capture.
2. **Anyone else** (a slinker friend beside a player, a slinker enemy
   beside a fellow enemy): the creature keeps stepping round it until its
   bar runs out (6.5). [research, capture]
3. **A friend next to the player it sought** drops a magic item if it
   carries one. [manual] On the C64 a peaceful friend of ours (Strength
   20, a treasure of ours and 100 gold, no magic item) dropped the
   treasure on its turn beside the player and kept its gold, in 16 of 16
   runs; where it landed took rolls (as 8.1). [capture: burden2,
   capture.md 10.14.5] **Engine:** its lowest-numbered magic item, else
   its lowest-numbered treasure (gold never), onto the first
   square around the player (direction order from N) that is an inside
   square accepting drops (inventory 5.4) with no individual on it, else
   onto the friend's own square. A drop spell then acts on the friend
   itself, since it lands on no creature ([magic](magic.md) 2.3). [unknown]
4. **Foes**, for readying and for 7.1: [manual] **Engine:**
   - an enemy's foes are players and friends;
   - a friend's foes are enemies, aggressive neutrals and aggressive thieves;
   - neutrals and thieves treat every other individual as a foe (a fighter
     neutral is not averse to attacking anybody; thieves show no loyalty to
     other thieves). [manual]
5. **Gold and treasure.** A creature picks up gold or a treasure as
   players do ([inventory](inventory.md) 2): gold adds the pile to the
   creature's wealth (up to 25,000, the rest stays), a treasure goes into
   its possessions within the carry limits. [manual] On the C64 it takes
   the thing from the square beside it, the step it picked (6.5) not
   taken, and its turn ends; the program shows "(name) takes (thing)" or
   "(name) finds gold worth N and now has M" and waits for the button.
   [capture: thief, 64 runs] (The amounts there, 512 and 1,361, came
   from our disks lacking a pile's size record: inventory 2.5.)
   **Engine:** takes it from beside, and the turn ends.
6. **An exit**: the creature takes the door (5.4).

## 8. Things creatures do with what they carry

1. **Overburdened creatures** drop a magic item, a treasure or gold.
   [manual] At the start, 3 master creatures and 5 residents carry more than
   their capacity. [data] On the C64 (Strength 1 creatures of ours, Speed
   2, 16 runs each, all alike): one with 300 gold and treasures of weight
   400 (lower number) and 1 dropped **all** its gold on its first turn and
   the weight 400 one on its second; one with 2,000 gold dropped all
   2,000 at once; one with treasures of weight 1 (lower number) and 400
   dropped the weight 1 one first, then the other. So all the gold first,
   then the lowest-numbered treasure, whatever it weighs; one drop a
   turn, and a turn with a drop had no steps (they walked again once
   under capacity). The gold stayed on the creature's square; a treasure
   landed on a square beside it, chosen with one to five pairs of rolls
   (a try per pair, seemingly). [capture: burden, burden2, capture.md
   10.14.5] **Engine:** at the start of its turn, a creature whose load is
   above its capacity drops all its gold on its own square (one pile),
   else its lowest-numbered treasure or magic item there; one
   drop a turn, after which its moves are idled. A treasure that
   disappears when dropped vanishes; things that can't be dropped
   (owner-only) are never chosen. While over capacity it picks nothing
   up. [unknown] Not settled: the treasure's square (the rolls: a
   research question), magic items' place in the order, whether the
   steps are lost to the drop or to the weight slowdown (inventory 3.3),
   and how gold counts toward the load.
2. **Readying.** A creature readies an appropriate weapon: when not next to
   a foe, a missile weapon if it has one, otherwise a melee weapon if it has
   one; with several, the lowest item number. [manual] **Engine:** checked
   at the start of each of its turns, free of cost; a change emits
   `CreatureReadied`. A creature already holding the right kind keeps it
   only if it is the lowest-numbered of that kind. [unknown]
3. A safe fighter with a readied missile weapon fires at a foe goal.
   [manual] Combat.
4. Casting and using magic items: [magic](magic.md) section 6.

## 9. Corpus check

The corpus test (`crates/acs-engine/tests/corpus.rs`) puts a player in each
of the 996 live rooms that hold residents or a random creature and plays a
dozen turns of waiting and wandering. With the region-visit scheduler
(2.2), a room door into another room ending the turn (movement 2.3) and
the healing and reversal rules of magic 5 (November 2026), the
creatures' own goals and steps of section 6 and the falling-away roll as
captured (2.4, November 2026), and the take from beside and the
overburdened drop as captured (7.5, 8.1), no confronting without a
weapon (7.1), gold piles' sizes (inventory 2.5) and arrivals on a held
destination (5.4, November 2026), creatures take 9,340 turns, make
17,646 steps, go through 171 doors, square up to someone 4,331 times,
ready 1,897 weapons, pick up gold 17 times and treasure 12 times, and
make 40 drops (overburdened, and friends' gifts);
7 random creatures appear and 2 fall away. No
creature leaves its region, stands on a wall after moving or breaks a
carry limit; none shares a square with another individual except on
arriving through a door (5.4). The random walks with
creatures replay exactly.

## 10. Left for later

- **Combat:** attacks on `CreatureConfronts`, attacking those met on the way,
  missile fire, creatures turning hostile when hurt.
- **World map:** encounter areas and the creatures in them.
- **Mimics** copy the opponent they meet: combat.

## Captures needed

1. Turn order: captured and researched (2.2, 2.3), players in different
   places and two players in different rooms of one region included.
   Still open: the round a player missed after coming in at the end of
   the map's visit (2.2); what happens to the visit when a player dies.
2. Random creatures: appearing and falling away captured and researched
   (3, 2.4). Still open: two random creatures left at once.
3. The adjacency slowdown: captured (2.6: halved, 4 squares, aggressive
   and not a friend). Still open: mid-turn, odd Speeds.
4. Goal choice: researched and captured for slinker friends and neutrals
   (6.2-6.4), and for thieves and a slinker enemy going for gold and
   treasure (6.5, 7.5). Still open: fighters' goals among individuals
   (combat).
5. Paths: researched and captured (6.5: 4 squares, scored with a roll).
   Still open: the first step's cost of some things.
6. A creature following through a door: captured (5.4: the step ends its
   turn; it lands on the destination square, sharing it with a player or a
   creature standing there). Seen in the same runs:
   creatures' paths and goals varied from run to run (a friend sometimes
   took the portal with the player still in the room), so goal choice
   takes a roll (items 4 and 5; the random-rolls brief, item 12).
7. The overburdened drop: captured (8.1: all the gold, then the
   lowest-numbered treasure, one a turn). Still open: the square a
   treasure lands on (rolls), magic items, gold's weight.
