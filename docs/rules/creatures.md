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
   removed frees it, so the limit is the slot count.
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
   [manual] **Engine:** when a player's turn ends there is a **creature
   phase**: every creature standing in a room that player was in at any
   point during the turn (the room it started in and each room it entered)
   takes one turn, in order of region, then slot. A creature takes at most
   one turn per phase, even if it walks into another visited room. Then the
   next player's turn starts. [unknown]
3. So a creature is awake exactly while a player is, or was this turn, in
   its room. A creature left in a room with no player stops acting. Two
   players in one room give its creatures two phases a round. [unknown]
4. **Random creatures fall away.** Random creatures in rooms no player
   occupies are removed now and then. [manual] How often is not given.
   **Engine:** at the end of every creature phase, each random creature whose
   room holds no player is removed (`CreatureRemoved`, reason `left_behind`),
   with everything it carries. [unknown]
5. **The movement bar.** A creature's turn has a bar like a player's: its
   Speed after the weight slowdown ([inventory](inventory.md) 3.3), using
   the creature's own Strength, Constitution, Life Force and load. [unknown]
   **Engine:** so. A creature with Speed 0 does not move; the manual only
   adds that it does not show its name on its turn. [manual]
6. **Standing next to an aggressive, unfriendly individual** shortens a
   player's bar. [manual] By how much is not given. **Engine:** at the start
   of a player's turn, if any creature on the 8 squares around the player is
   Aggressive and not a Friend, the bar is halved (after the weight slowdown,
   rounding down, at least 1; Speed 0 stays 0). The `TurnStarted` event says
   so (`hindered`). Applied at turn start only. [unknown]

## 3. Random room creatures

1. On entering a room there is one roll against the room's chance for its
   random creature (movement 7.2). [manual]
2. It appears only on a square with no thing on it. [manual] **Engine:** a
   uniformly chosen inside square with no room item, no player and no
   creature, picked with the game's generator; if there is none, nothing
   appears. The roll comes first, so a full room still uses up the roll.
   [unknown]
3. It is a copy of the region's random creature record (traits, possessions,
   wealth, readied things, strategy), put in the lowest free slot.
   [unknown] **Engine:** event `RoomEncounter` with the slot and square.
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
   pass without the spell going off, until the magic rules exist. [unknown]
4. **Taking a door** costs one step and ends the creature's turn. It arrives
   on the door's destination square, or if an individual stands there, on
   the nearest inside square with no individual and no thing (rings of
   growing distance, scanned top row first, left to right); with none free
   it stays where it is and its turn ends. [unknown] Baber notes that a creature following a player can share
   the player's square for a moment. [designer] **Engine:** never shares.
5. A creature sent through a door into a deleted room is banished.
   [manual] **Engine:** removed (`CreatureRemoved`, reason `banished`) with
   what it carries.
6. Moves are one square in any of the 8 directions, like players
   (movement 3.2). [unknown]

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

2. A slinking, wounded creature seeks an exit. [manual] **Engine:** for a
   wounded slinker, exit moves to the front of its list.
3. **Nearer beats better.** Most creatures go for a less favoured goal if it
   is closer than a more desirable one. [manual] **Engine:** every creature
   takes the nearest reachable goal of any category on its list; the list
   order only breaks ties of distance, and after that the scan order
   (players by number, then creatures by slot, then squares top row first).
   One exception: an *exit* counts only when nothing ranked above it on the
   list is reachable. Almost every room has a door, so if exits competed on
   distance, a fighter enemy would usually walk out of the room instead of
   closing in on the player in it, which is not how the game plays: players
   flee through doors and only one or two creatures follow. [designer]
   [unknown]
4. **What counts as each goal**, all inside the creature's room: [unknown]
   **Engine:**
   - *player*, *friend*, *enemy* and so on: another individual in the room,
     by alignment and Aggressive/Peaceful; never the creature itself;
   - a player who left the room this turn by a door this creature may take
     counts as a *player* goal on that door square, so creatures chase the
     player through the door they took (2.1);
   - *exit*: a door square the creature may take (5.3);
   - *gold*: a square whose top is Gold, for creatures that pick it up;
   - *treasure*: a square whose top is a treasure other than Gold, for
     thieves.
5. **Distance** is the number of steps along squares the creature may enter
   (5.1, 5.2), diagonals counting one (a breadth-first search). An
   individual goal is reached on any square next to it (8 neighbours); a
   thing or door goal on its square. Goals with no path are skipped.
   [unknown]
6. The creature steps one square along a shortest path, then chooses again,
   until its bar runs out, it reaches a goal that ends its turn, or no goal
   is reachable (then it stays put). Among shortest first steps the engine
   takes the first in direction order N, NE, E, SE, S, SW, W, NW. [unknown]

## 7. Reaching a goal

1. **A foe.** A fighter loves to attack certain types and may attack others
   it meets on the way. [manual] **Engine:** a fighter next to the
   individual it chose emits `CreatureConfronts` when that individual is a
   foe (7.4), and its turn ends; combat will turn this into an attack.
   Attacks on individuals met on the way come with combat.
2. **Anyone else** (a slinker friend next to a player, a slinker enemy next
   to a fellow enemy): the creature stays there and its turn ends.
   [unknown]
3. **A friend next to the player it sought** drops a magic item if it
   carries one. [manual] **Engine:** the lowest-numbered one, onto the first
   square around the player (direction order from N) that is an inside
   square accepting drops (inventory 5.4) with no individual on it, else
   onto the friend's own square. Its drop spell does not go off yet. [unknown]
4. **Foes**, for readying and for 7.1: [manual] **Engine:**
   - an enemy's foes are players and friends;
   - a friend's foes are enemies, aggressive neutrals and aggressive thieves;
   - neutrals and thieves treat every other individual as a foe (a fighter
     neutral is not averse to attacking anybody; thieves show no loyalty to
     other thieves). [manual]
5. **Gold and treasure.** Stepping onto gold or a treasure it picks up takes
   it, as for players ([inventory](inventory.md) 2): gold adds the pile to
   the creature's wealth (up to 25,000, the rest stays), a treasure goes into
   its possessions within the carry limits. [manual] Its turn goes on.
6. **An exit**: the creature takes the door (5.4).

## 8. Things creatures do with what they carry

1. **Overburdened creatures** drop a magic item, a treasure or gold.
   [manual] At the start, 3 master creatures and 5 residents carry more than
   their capacity. [data] **Engine:** at the start of its turn, a creature
   whose load is above its capacity drops one thing on its own square: the
   lowest-numbered magic item, else the lowest-numbered treasure, else a
   pile of gold just big enough to get back under capacity (at most 255, at
   least 1). Only one drop a turn. A treasure that disappears when dropped
   vanishes; things that can't be dropped (owner-only) are never chosen.
   While over capacity it picks nothing up. [unknown]
2. **Readying.** A creature readies an appropriate weapon: when not next to
   a foe, a missile weapon if it has one, otherwise a melee weapon if it has
   one; with several, the lowest item number. [manual] **Engine:** checked
   at the start of each of its turns, free of cost; a change emits
   `CreatureReadied`. A creature already holding the right kind keeps it
   only if it is the lowest-numbered of that kind. [unknown]
3. A safe fighter with a readied missile weapon fires at a foe goal.
   [manual] Combat.
4. Casting and using magic items: the magic rules.

## 9. Corpus check

The corpus test (`crates/acs-engine/tests/corpus.rs`) puts a player in each
of the 996 live rooms that hold residents or a random creature and plays a
dozen turns of waiting and wandering. Creatures take 18,452 turns, make
6,649 steps, go through 509 doors, square up to someone 16,631 times, ready
2,711 weapons, pick up gold 63 times and treasure 9 times, and make 52
overburdened drops. No creature leaves its region, stands on a wall after
moving, shares a square or breaks a carry limit. The random walks with
creatures replay exactly.

## 10. Left for later

- **Combat:** attacks on `CreatureConfronts`, attacking those met on the way,
  missile fire, creatures turning hostile when hurt.
- **Magic:** creatures casting and using items, door and drop spells set off
  by creatures, summons (which also obey 1.1), Banish.
- **World map:** encounter areas and the creatures in them.
- **Mimics** copy the opponent they meet: combat.

## Captures needed

1. Turn order with several players and creatures in one room.
2. Random creature square choice and how soon left-behind ones go.
3. The adjacency slowdown: bar length next to an aggressive unfriendly.
4. Goal choice when a less favoured goal is closer: how much closer.
5. Path tie-breaks and whether creatures move diagonally.
6. A creature following through a door: where it lands, and whether its
   turn goes on in the new room.
7. The overburdened drop: what, where, how much gold, how often.
