// SPDX-License-Identifier: GPL-3.0-or-later

//! Movement and inventory smoke tests on the private test corpus.
//!
//! Set `ACSVM_CORPUS` to the corpus folder (`c64/` with `.d64` disks, `pc/`
//! with `.fpy`/`.hrd`); without it these tests pass with a "skipped" note.
//! Disks named `advcnst*` are EA's program disks and are skipped.

use acs_core::model::{Access, Adventure, ThingKind};
use acs_core::{decode, Image, Platform};
use acs_engine::inventory::{self, GOLD};
use acs_engine::{Command, Dir, Event, Game, Location, PlayerSetup, Rng, Rules};
use std::path::{Path, PathBuf};

fn find(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            find(&p, out);
        } else if Platform::from_path(&p).is_some() {
            let name = p.file_name().unwrap().to_string_lossy().to_ascii_lowercase();
            if !name.starts_with("advcnst") {
                out.push(p);
            }
        }
    }
}

fn corpus() -> Option<Vec<(String, Adventure)>> {
    let dir = PathBuf::from(std::env::var_os("ACSVM_CORPUS")?);
    let mut files = Vec::new();
    find(&dir, &mut files);
    files.sort();
    assert!(!files.is_empty(), "no adventures under {}", dir.display());
    Some(
        files
            .iter()
            .map(|p| {
                let image = Image::open(p, None).expect("opens");
                let name = p.file_name().unwrap().to_string_lossy().into_owned();
                (name, decode(&image).expect("decodes").adventure)
            })
            .collect(),
    )
}

fn game(a: &Adventure) -> Game {
    Game::new(
        a.clone(),
        1,
        Rules::default(),
        PlayerSetup {
            speed: 15,
            ..Default::default()
        },
    )
    .0
}

/// A place a character may legally stand after a jump: an inside square of a
/// live room, or a map cell.
fn valid_landing(a: &Adventure, at: Location) -> bool {
    match at {
        Location::World { x, y } => x < 40 && y < 40,
        Location::Room { region, room, x, y } => a
            .regions
            .iter()
            .find(|r| r.number == region)
            .and_then(|r| r.rooms.get(room as usize))
            .is_some_and(|r| !r.deleted && x > 0 && y > 0 && x + 1 < r.width && y + 1 < r.height),
    }
}

#[derive(Default, Debug)]
struct Tally {
    taken: usize,
    to_start: usize,
    covered: usize,
    guarded: usize,
}

#[test]
fn every_door_can_be_taken_and_lands_somewhere_real() {
    let Some(advs) = corpus() else {
        eprintln!("skipped: ACSVM_CORPUS not set");
        return;
    };
    let mut total = Tally::default();
    let mut failures = Vec::new();
    for (file, a) in &advs {
        let mut t = Tally::default();
        // Room doors.
        for reg in &a.regions {
            for room in reg.rooms.iter().filter(|r| !r.deleted) {
                for (i, item) in room.items.iter().enumerate() {
                    let Some(thing) = a.things.get(item.thing as usize - 1) else {
                        continue;
                    };
                    let ThingKind::Portal { access, .. } = &thing.kind else {
                        continue;
                    };
                    // Only the top of a stack acts (a door under a cover is
                    // reached once the cover is gone).
                    if room.items[..i].iter().any(|o| (o.x, o.y) == (item.x, item.y)) {
                        t.covered += 1;
                        continue;
                    }
                    // Stand next to the door, inside the room, and step onto it
                    // (straight: the default rules refuse diagonal steps).
                    let Some((sx, sy, dir)) = Dir::ALL.iter().filter(|d| !d.is_diagonal()).find_map(|d| {
                        let (dx, dy) = d.delta();
                        let (sx, sy) = (item.x as i16 - dx as i16, item.y as i16 - dy as i16);
                        let inside =
                            sx > 0 && sy > 0 && sx + 1 < room.width as i16 && sy + 1 < room.height as i16;
                        inside.then_some((sx as u8, sy as u8, *d))
                    }) else {
                        failures.push(format!(
                            "{file}: region {} room {} door at {},{} has no inside neighbour",
                            reg.number, room.number, item.x, item.y
                        ));
                        continue;
                    };
                    let mut g = game(a);
                    g.place(
                        0,
                        Location::Room {
                            region: reg.number,
                            room: room.number,
                            x: sx,
                            y: sy,
                        },
                    );
                    if let Access::OwnersOf { thing } = access {
                        g.give(0, *thing);
                    }
                    let ev = g.apply(Command::Move { dir });
                    let at = g.current_player().at;
                    if ev.iter().any(|e| matches!(e, Event::BumpedCreature { .. })) {
                        t.guarded += 1;
                    } else if ev.iter().any(|e| matches!(e, Event::SentToStart { .. })) {
                        t.to_start += 1;
                    } else if ev.iter().any(|e| matches!(e, Event::TookDoor { .. })) && valid_landing(a, at) {
                        t.taken += 1;
                    } else {
                        failures.push(format!(
                            "{file}: region {} room {} door at {},{} (param {}): {ev:?}",
                            reg.number, room.number, item.x, item.y, item.parameter
                        ));
                    }
                }
            }
        }
        // Map doors on door terrain.
        for door in &a.world.doors {
            let (x, y) = (door.at.x, door.at.y);
            if a.world.tiles[y as usize][x as usize] < 12 {
                continue;
            }
            let Some((sx, sy, dir)) = Dir::ALL.iter().find_map(|d| {
                let (dx, dy) = d.delta();
                let (sx, sy) = (x as i16 - dx as i16, y as i16 - dy as i16);
                ((0..40).contains(&sx) && (0..40).contains(&sy)).then_some((sx as u8, sy as u8, *d))
            }) else {
                continue;
            };
            let mut g = game(a);
            g.place(0, Location::World { x: sx, y: sy });
            let ev = g.apply(Command::Move { dir });
            let at = g.current_player().at;
            if ev.iter().any(|e| matches!(e, Event::SentToStart { .. })) {
                t.to_start += 1;
            } else if ev
                .iter()
                .any(|e| matches!(e, Event::TookDoor { thing: None, .. }))
                && valid_landing(a, at)
            {
                t.taken += 1;
            } else {
                failures.push(format!("{file}: map door {} at {x},{y}: {ev:?}", door.index));
            }
        }
        println!("{file}: {t:?}");
        total.taken += t.taken;
        total.to_start += t.to_start;
        total.covered += t.covered;
        total.guarded += t.guarded;
    }
    println!("total: {total:?}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(total.taken > 0);
}

#[test]
fn random_walks_stay_on_real_squares_and_replay() {
    let Some(advs) = corpus() else {
        eprintln!("skipped: ACSVM_CORPUS not set");
        return;
    };
    for (file, a) in &advs {
        let setup = PlayerSetup::default();
        let (mut g, _) = Game::new(a.clone(), 99, Rules::default(), setup.clone());
        let mut pick = Rng::new(5);
        let mut rooms = std::collections::BTreeSet::new();
        for _ in 0..3000 {
            let dir = Dir::ALL[pick.below(8) as usize];
            let carried = g.current_player().possessions.clone();
            let any = |pick: &mut Rng| {
                carried
                    .get(pick.below(carried.len().max(1) as u32) as usize)
                    .copied()
            };
            let cmd = match pick.below(40) {
                0..=3 => Command::Wait,
                4 => match any(&mut pick) {
                    Some(thing) => Command::Drop {
                        thing,
                        dir: (pick.below(2) == 0).then_some(dir),
                    },
                    None => Command::Wait,
                },
                5 => match any(&mut pick) {
                    Some(thing) => Command::Ready { thing },
                    None => Command::Wait,
                },
                6 => Command::DropGold {
                    amount: 1,
                    dir: Some(dir),
                },
                _ => Command::Move { dir },
            };
            g.apply(cmd);
            // Inventory invariants (inventory 1.2, 1.3, 4.4).
            let pl = g.current_player();
            for &t in &pl.possessions {
                assert!(
                    pl.count(t) <= inventory::limit(t),
                    "{file}: {} of thing {t}",
                    pl.count(t)
                );
                assert_ne!(t, GOLD, "{file}: gold kept as an item");
            }
            assert!(pl.gold <= inventory::GOLD_LIMIT);
            for t in [pl.weapon, pl.armor].into_iter().flatten() {
                assert!(pl.owns(t), "{file}: readied {t} not carried");
            }
            let at = g.current_player().at;
            match at {
                Location::World { x, y } => assert!(x < 40 && y < 40, "{file}: {at:?}"),
                Location::Room { region, room, x, y } => {
                    let r = &g.adventure().regions[region as usize - 1].rooms[room as usize];
                    assert!(!r.deleted && x < r.width && y < r.height, "{file}: {at:?}");
                    rooms.insert((region, room));
                }
            }
        }
        let r = Game::replay(a.clone(), 99, Rules::default(), setup, g.log());
        assert_eq!(r, g, "{file}: replay differs");
        let pl = g.current_player();
        println!(
            "{file}: random walk visited {} rooms, carries {} things and {} gold, load {}/{}",
            rooms.len(),
            pl.possessions.len(),
            pl.gold,
            pl.load(g.adventure()),
            pl.capacity()
        );
    }
}

/// The counts behind inventory.md sections 1 and 3.
#[test]
fn carriable_things_and_creature_loads_match_the_spec() {
    let Some(advs) = corpus() else {
        eprintln!("skipped: ACSVM_CORPUS not set");
        return;
    };
    let (mut carriable, mut over, mut over_half, mut creatures) = (0, 0, 0, 0);
    for (file, a) in &advs {
        // 1.3: thing 81 is the gold treasure, value 1.
        let gold = &a.things[GOLD as usize - 1];
        assert!(
            matches!(gold.kind, ThingKind::Treasure { carry, .. } if carry.value == 1),
            "{file}: thing 81 is {:?}",
            gold.kind
        );
        // 1.1: every carriable thing is numbered 1..=81.
        for t in &a.things {
            if inventory::carriable(&t.kind) {
                carriable += 1;
                assert!(t.number <= GOLD, "{file}: carriable thing {}", t.number);
            }
        }
        // 1.2 and 3.5: creature possessions within the limits, and loads.
        let all = a
            .creatures
            .iter()
            .chain(a.regions.iter().flat_map(|r| r.residents.iter()));
        for c in all.filter(|c| c.in_use) {
            creatures += 1;
            for p in &c.possessions {
                assert!(
                    p.count as u16 <= inventory::limit(p.thing),
                    "{file}: {} {p:?}",
                    c.name
                );
                assert!(inventory::carriable(&a.things[p.thing as usize - 1].kind));
            }
            let load: i32 = c
                .possessions
                .iter()
                .map(|p| inventory::weight(a, p.thing) * p.count as i32)
                .sum::<i32>()
                + c.wealth as i32 * inventory::weight(a, GOLD);
            let capacity = (c.strength as i32 - (c.constitution as i32 - c.life_force as i32) / 2) * 100;
            if load > capacity {
                over += 1;
            } else if load * 2 > capacity {
                over_half += 1;
            }
        }
    }
    println!(
        "{carriable} carriable things; {creatures} creatures: {over} over capacity, {over_half} over half"
    );
    assert!(carriable > 0);
}

/// Steps onto every carriable thing that is on top of its stack.
#[test]
fn every_carriable_thing_can_be_picked_up() {
    let Some(advs) = corpus() else {
        eprintln!("skipped: ACSVM_CORPUS not set");
        return;
    };
    let (mut items, mut gold, mut pieces, mut readied, mut other) = (0, 0, 0u32, 0, Vec::new());
    for (file, a) in &advs {
        for reg in &a.regions {
            for room in reg.rooms.iter().filter(|r| !r.deleted) {
                for (i, item) in room.items.iter().enumerate() {
                    let Some(thing) = a.things.get(item.thing as usize - 1) else {
                        continue;
                    };
                    if !inventory::carriable(&thing.kind)
                        || room.items[..i].iter().any(|o| (o.x, o.y) == (item.x, item.y))
                    {
                        continue;
                    }
                    let Some((sx, sy, dir)) = Dir::ALL.iter().find_map(|d| {
                        let (dx, dy) = d.delta();
                        let (sx, sy) = (item.x as i16 - dx as i16, item.y as i16 - dy as i16);
                        let inside =
                            sx > 0 && sy > 0 && sx + 1 < room.width as i16 && sy + 1 < room.height as i16;
                        inside.then_some((sx as u8, sy as u8, *d))
                    }) else {
                        continue;
                    };
                    let mut g = game(a);
                    let at = |x, y| Location::Room {
                        region: reg.number,
                        room: room.number,
                        x,
                        y,
                    };
                    g.place(0, at(sx, sy));
                    let ev = g.apply(Command::Move { dir });
                    // The taker stays on its own square (inventory 2.1).
                    let stayed = g.current_player().at == at(sx, sy);
                    if ev.iter().any(|e| matches!(e, Event::BumpedCreature { .. })) {
                        continue;
                    }
                    match ev.first() {
                        Some(Event::PickedUp { thing, .. }) if *thing == item.thing && stayed => {
                            items += 1;
                            readied += ev.iter().filter(|e| matches!(e, Event::Readied { .. })).count();
                        }
                        Some(Event::GoldPickedUp { amount, .. }) if stayed => {
                            gold += 1;
                            pieces += *amount as u32;
                        }
                        _ => other.push(format!(
                            "{file}: region {} room {} {} at {},{}: {ev:?}",
                            reg.number, room.number, thing.name, item.x, item.y
                        )),
                    }
                }
            }
        }
    }
    println!("picked up {items} things ({readied} weapons readied) and {gold} gold piles ({pieces} pieces)");
    assert!(other.is_empty(), "{}", other.join("\n"));
    assert!(items > 0);
}

/// Creatures 1-8 and magic 6-7: a player dropped into every room that has
/// residents or a random creature, then a dozen turns of waiting and
/// wandering. Creatures stay in their region, never share a square, stand
/// only inside live rooms once they move, and keep within the carry limits.
/// Casting creatures may kill the player, which ends that room's tour.
#[test]
fn creatures_in_every_room_follow_the_rules() {
    use acs_engine::creatures::is_fighter;
    use std::collections::BTreeMap;
    let Some(advs) = corpus() else {
        eprintln!("skipped: ACSVM_CORPUS not set");
        return;
    };
    let mut total: BTreeMap<String, usize> = BTreeMap::new();
    let mut types: BTreeMap<String, usize> = BTreeMap::new();
    let mut rooms_toured = 0;
    for (file, a) in &advs {
        for reg in &a.regions {
            for c in reg.residents.iter().filter(|c| c.in_use) {
                let kind = if is_fighter(c) { "fighter" } else { "slinker" };
                *types
                    .entry(format!("{kind} {:?}", c.strategy.alignment))
                    .or_default() += 1;
                if c.speed == 0 {
                    *types.entry("speed 0".into()).or_default() += 1;
                }
            }
        }
        let mut tally: BTreeMap<String, usize> = BTreeMap::new();
        for reg in &a.regions {
            for room in reg.rooms.iter().filter(|r| !r.deleted) {
                let lives = reg.residents.iter().any(|c| c.in_use && c.room == room.number);
                let random = room.random_chance > 0
                    && reg
                        .random_creatures
                        .get(room.random_creature as usize)
                        .is_some_and(|c| c.in_use);
                if !lives && !random {
                    continue;
                }
                let start: Vec<_> = reg
                    .residents
                    .iter()
                    .map(|c| (c.in_use, c.room, c.x, c.y))
                    .collect();
                let free = (1..room.height.saturating_sub(1))
                    .flat_map(|y| (1..room.width.saturating_sub(1)).map(move |x| (x, y)))
                    .find(|&(x, y)| {
                        !room.items.iter().any(|it| it.x == x && it.y == y)
                            && !reg
                                .residents
                                .iter()
                                .any(|c| c.in_use && c.room == room.number && (c.x, c.y) == (x, y))
                    });
                let Some((x, y)) = free else { continue };
                rooms_toured += 1;
                let mut g = game(a);
                g.place(
                    0,
                    Location::Room {
                        region: reg.number,
                        room: room.number,
                        x,
                        y,
                    },
                );
                let mut pick = Rng::new(reg.number as u64 * 100 + room.number as u64);
                let mut arrived = std::collections::HashSet::new();
                for i in 0..12 {
                    let cmd = if i % 2 == 0 {
                        Command::Wait
                    } else {
                        Command::Move {
                            dir: Dir::ALL[pick.below(8) as usize],
                        }
                    };
                    let evs = g.apply(cmd);
                    let last = format!("{evs:?}");
                    // Creatures 5.4: a creature that came through a door
                    // may share the player's square until it moves.
                    for e in &evs {
                        match e {
                            Event::CreatureTookDoor { region, slot, .. } => {
                                arrived.insert((*region, *slot));
                            }
                            Event::CreatureMoved { region, slot, .. } => {
                                arrived.remove(&(*region, *slot));
                            }
                            _ => {}
                        }
                    }
                    for e in evs {
                        let k = format!("{e:?}");
                        let k = k.split([' ', '{']).next().unwrap();
                        let magic = [
                            "SpellCast",
                            "ItemUsed",
                            "SpellTriggered",
                            "Died",
                            "SummonUnanswered",
                            "Music",
                            "LongMessage",
                            "Given",
                        ];
                        if k.starts_with("Creature") || k == "RoomEncounter" || magic.contains(&k) {
                            *tally.entry(k.to_string()).or_default() += 1;
                        }
                        if let Event::CreatureRemoved { reason, .. } = e {
                            *tally.entry(format!("removed {reason:?}")).or_default() += 1;
                        }
                        if let Event::SpellCast { success: true, .. } = e {
                            *tally.entry("cast and worked".into()).or_default() += 1;
                        }
                        if let Event::Died {
                            who: acs_engine::Who::Player { .. },
                            ..
                        } = e
                        {
                            *tally.entry("player died".into()).or_default() += 1;
                        }
                    }
                    let ga = g.adventure();
                    for greg in &ga.regions {
                        let live: Vec<_> = (0..g.slots(greg.number))
                            .filter_map(|s| g.creature(greg.number, s).map(|c| (s, c)))
                            .collect();
                        for &(s, c) in &live {
                            let r = &greg.rooms[c.room as usize];
                            assert!(!r.deleted && c.x < r.width && c.y < r.height, "{file}: {c:?}");
                            let moved = start.get(s).is_some_and(|&st| st != (true, c.room, c.x, c.y))
                                || greg.number != reg.number;
                            if moved && greg.number == reg.number {
                                assert!(
                                    c.x > 0 && c.y > 0 && c.x + 1 < r.width && c.y + 1 < r.height,
                                    "{file}: creature on a wall {c:?}"
                                );
                            }
                            for &(t, o) in &live {
                                let same = t != s && o.room == c.room && (o.x, o.y) == (c.x, c.y);
                                // Except one that just came through a door
                                // (creatures 5.4).
                                let door = arrived.contains(&(greg.number, s))
                                    || arrived.contains(&(greg.number, t));
                                assert!(
                                    !(same && moved && !door),
                                    "{file}: two creatures share {},{}",
                                    c.x,
                                    c.y
                                );
                            }
                            let pl = g.current_player().at;
                            // The dead stand nowhere (magic 7.4).
                            if moved && g.current_player().alive && !arrived.contains(&(greg.number, s)) {
                                assert_ne!(
                                    pl,
                                    Location::Room {
                                        region: greg.number,
                                        room: c.room,
                                        x: c.x,
                                        y: c.y
                                    },
                                    "{file}: creature on the player after {last}"
                                );
                            }
                            assert!(c.wealth <= inventory::GOLD_LIMIT);
                            for p in &c.possessions {
                                assert!(p.count as u16 <= inventory::limit(p.thing), "{file}: {c:?}");
                            }
                        }
                    }
                    if g.is_over() {
                        break;
                    }
                }
            }
        }
        println!("{file}: {tally:?}");
        for (k, v) in tally {
            *total.entry(k).or_default() += v;
        }
    }
    println!("residents by type: {types:?}");
    println!("{rooms_toured} rooms toured; total: {total:?}");
}
