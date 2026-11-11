// SPDX-License-Identifier: GPL-3.0-or-later

//! Movement smoke tests on the private test corpus.
//!
//! Set `ACSVM_CORPUS` to the corpus folder (`c64/` with `.d64` disks, `pc/`
//! with `.fpy`/`.hrd`); without it these tests pass with a "skipped" note.
//! Disks named `advcnst*` are EA's program disks and are skipped.

use acs_core::model::{Access, Adventure, ThingKind};
use acs_core::{decode, Image, Platform};
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
            let cmd = match pick.below(10) {
                0 => Command::Wait,
                _ => Command::Move {
                    dir: Dir::ALL[pick.below(8) as usize],
                },
            };
            g.apply(cmd);
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
        println!("{file}: random walk visited {} rooms", rooms.len());
    }
}
