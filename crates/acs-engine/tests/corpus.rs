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
                    // Stand next to the door, inside the room, and step onto it.
                    let Some((sx, sy, dir)) = Dir::ALL.iter().find_map(|d| {
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
                    let landed = g.current_player().at == at(item.x, item.y);
                    if ev.iter().any(|e| matches!(e, Event::BumpedCreature { .. })) {
                        continue;
                    }
                    match ev.first() {
                        Some(Event::PickedUp { thing, .. }) if *thing == item.thing && landed => {
                            items += 1;
                            readied += ev.iter().filter(|e| matches!(e, Event::Readied { .. })).count();
                        }
                        Some(Event::GoldPickedUp { amount, .. }) if landed => {
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
