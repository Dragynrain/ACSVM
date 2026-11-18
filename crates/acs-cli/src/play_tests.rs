// SPDX-License-Identifier: GPL-3.0-or-later

//! `acs play` on a synthetic adventure built with acs-core's fixture
//! builder. No real adventure data.

use crate::play::play;
use crate::script::parse;
use crate::test_fixture::{creature_record, Fixture, RegionSpec, RoomSpec};
use acs_core::layout::world_map as wm;
use acs_core::model::{
    Access, Alignment, Carry, Destination, MapDoor, MapPoint, RoomItem, SpellAction, SpellActionKind,
    ThingKind, WeaponStats, WorldCreature,
};
use acs_core::{decode, Adventure, Platform};
use acs_engine::{Game, Location, SaveFile, Who};

const KEY: u8 = 1; // treasure
const LOCKED: u8 = 2; // portal, owners of KEY
const DOOR: u8 = 3; // portal, anyone
const PILLAR: u8 = 4; // obstacle
const SWORD: u8 = 5; // melee weapon
const SCROLL: u8 = 6; // a space showing long message 7 (set in the model)

/// One region, three rooms:
///
/// - HALL 8x6: the map door's arrival at (2,2); keys on (3,2) and (3,4);
///   a locked door (owners of KEY) on the east wall (7,2) into VAULT (1,2).
/// - VAULT 6x6: a door back to HALL (0,2); a door on (5,2) into FINAL
///   (1,2); pillars round (3,4), which no one can reach.
/// - FINAL 5x5: a door back to VAULT (0,2).
///
/// World map: open grass, start (20,20), a map door at (20,19) into HALL.
/// Resident slot 0 is a template creature, not in use.
fn adventure() -> Adventure {
    let mut f = Fixture::new(Platform::C64);
    f.header("PLAYTEST", "", "");
    f.thing(KEY as usize, "KEY", [0x01, 1, 1, 3]);
    f.thing(LOCKED as usize, "LOCKED DOOR", [0x07, 5 | 2 << 6, 0, KEY]);
    f.thing(DOOR as usize, "DOOR", [0x27, 5, 0, 0]);
    f.thing(PILLAR as usize, "PILLAR", [0x0A, 8, 0, 0]);
    f.thing(SWORD as usize, "SWORD", [0x04, 5, 10, 0]);
    f.extended(None, 0, [5, 0, 20 | 0x40]);
    f.thing(SCROLL as usize, "SCROLL", [0x01, 1, 1, 3]);
    f.long_message(7, "AN OLD SCROLL");
    let mut template = creature_record(10, 0x60, 70);
    template[0x10] = 0x10;
    f.thing(127, "EMPTY FLOOR", [0x00, 1, 0, 0]);
    f.thing(128, "ROOM BORDER", [0x00, 9, 0, 0]);
    f.region(RegionSpec {
        name: "KEEP",
        rooms: vec![
            RoomSpec {
                name: "HALL",
                w: 8,
                h: 6,
                wall: 9,
                items: vec![(3, 2, KEY, 1), (3, 4, KEY, 1), (7, 2, LOCKED, 1)],
                ..Default::default()
            },
            RoomSpec {
                name: "VAULT",
                w: 6,
                h: 6,
                x: 7,
                wall: 9,
                items: vec![
                    (0, 2, DOOR, 2),
                    (5, 2, DOOR, 3),
                    (3, 3, PILLAR, 0),
                    (2, 4, PILLAR, 0),
                    (4, 4, PILLAR, 0),
                ],
                ..Default::default()
            },
            RoomSpec {
                name: "FINAL",
                w: 5,
                h: 5,
                x: 12,
                wall: 9,
                items: vec![(0, 2, DOOR, 4)],
                ..Default::default()
            },
        ],
        portals: vec![
            [0, 0],
            [0x11, 0x12], // HALL's locked door -> VAULT (1,2)
            [0x10, 0x62], // VAULT -> HALL (6,2)
            [0x12, 0x12], // VAULT -> FINAL (1,2)
            [0x11, 0x42], // FINAL -> VAULT (4,2)
        ],
        residents: vec![(0, "TEMPLATE", template)],
        ..Default::default()
    });
    let w = f.l.world_map;
    f.put(w + wm::TERRAIN, &[1 | 1 << 6, 0]); // grass, open
    f.put(w + wm::TERRAIN + 12 * 2, &[5 | 1 << 6, 0]); // door terrain
    let a = w + wm::MAP + 19 * 20 + 10; // (20,19): high nibble
    f.data[a] = (f.data[a] & 0x0F) | 12 << 4;
    f.put(w + wm::START_X, &[20, 20]);
    f.put(w + wm::POINTS, &[20, 19]);
    f.put(w + wm::DOOR_DESTINATIONS, &[0x10, 0x22]);
    let mut a = decode(&f.build()).expect("decodes").adventure;
    a.regions[0].residents[0].in_use = false;
    let t = &mut a.things[SCROLL as usize - 1];
    t.kind = ThingKind::Space {
        picture: 4,
        access: Access::InvokeSpell {
            action: SpellAction {
                kind: SpellActionKind::LongMessage,
                parameter: 7,
            },
        },
    };
    t.raw = [0; 4];
    a
}

/// Puts `thing` on `(x, y)` of `room` (on top), through to `to` when given.
fn put(a: &mut Adventure, room: usize, x: u8, y: u8, thing: u8, to: Option<Destination>) {
    a.regions[0].rooms[room].items.insert(
        0,
        RoomItem {
            x,
            y,
            thing: thing as u16,
            // A carriable thing's count.
            parameter: 1,
            flag: false,
            destination: to,
            gold: None,
        },
    );
}

/// Takes away what lies on `(x, y)` of `room`.
fn clear(a: &mut Adventure, room: usize, x: u8, y: u8) {
    a.regions[0].rooms[room].items.retain(|it| (it.x, it.y) != (x, y));
}

fn room(room: u8, x: u8, y: u8) -> Location {
    Location::Room {
        region: 1,
        room,
        x,
        y,
    }
}

fn run(adv: &Adventure, script: &str, seed: u64) -> crate::play::Outcome {
    let script = parse(script, adv).expect("parses");
    play(adv, &script, seed, 1)
}

const WALKTHROUGH: &str = "\
# Two players into the vault and on to the final room.
player 1 name=ANNA
player 2 name=\"BEN B\" speed=5

as 1
goto 1 0                # through the map door
expect at 1 0 2 2
take 3 2
expect owns key
door 7 2                # the locked door: ANNA owns a KEY
expect at 1 1 1 2
as 2
expect not owns KEY
goto 1 0
take 3 4
goto 1 1 2 2
as 1
goto 1 2 2 2
as 2
goto 1 2 3 2
expect at 1 2 3 2
as 1
expect at 1 2 2 2
as both
expect alive
expect life >= 10
";

#[test]
fn two_players_reach_the_final_room() {
    let adv = adventure();
    let script = parse(WALKTHROUGH, &adv).expect("parses");
    let out = play(&adv, &script, 7, 1);
    assert_eq!(out.failure, None, "{}", out.progress.join("\n"));
    let p = out.game.players();
    assert_eq!(p.len(), 2);
    assert_eq!(p[1].name, "BEN B");
    assert_eq!(p[1].speed, 5);
    for (pl, x) in p.iter().zip([2, 3]) {
        assert_eq!(
            pl.at,
            Location::Room {
                region: 1,
                room: 2,
                x,
                y: 2
            }
        );
        assert!(pl.owns(KEY as u16));
    }
    // The save replays to the same game.
    let save = SaveFile::from_json(&out.save.to_json()).expect("reads");
    let r = save.restore(&adv).expect("restores");
    assert!(!r.diverged);
    assert_eq!(r.game.players(), out.game.players());
    // The trace is one JSON object a line, steps among them.
    assert!(out
        .trace
        .iter()
        .all(|l| serde_json::from_str::<serde_json::Value>(l).is_ok()));
    assert!(out.trace.iter().any(|l| l.contains("\"step\"")));
    assert!(out.trace.iter().any(|l| l.contains("picked_up")));
}

#[test]
fn a_locked_door_turns_back_the_keyless() {
    let adv = adventure();
    let script = parse(
        "goto 1 0\ntake 3 2\ndrop key\nexpect not owns key\ndoor 7 2 avoid-takes\n",
        &adv,
    )
    .expect("parses");
    let out = play(&adv, &script, 1, 1);
    let f = out.failure.expect("fails");
    assert!(f.contains("line 5: door 7 2 avoid-takes"), "{f}");
    assert!(f.contains("door 7,2 is unreachable"), "{f}");
    // The other key would have opened it.
    assert!(f.contains("takes left alone: 3,4 (KEY)"), "{f}");
}

#[test]
fn an_unreachable_square_says_what_was_reachable() {
    let adv = adventure();
    let script = parse("goto 1 0\ntake 3 2\ndoor 7 2\nwalk 3 4\nexpect alive\n", &adv).expect("parses");
    let out = play(&adv, &script, 3, 1);
    let f = out.failure.expect("fails");
    assert!(f.contains("line 4: walk 3 4"), "{f}");
    assert!(
        f.contains("square 3,4 is unreachable from VAULT (region 1 KEEP, room 1) at 1,2"),
        "{f}"
    );
    // Nearest first: the pillars round 3,4 aren't reachable, so the open
    // squares by the ring lead the list.
    assert!(
        f.contains("reachable: 12 squares, nearest first: 2,3 4,3 3,2"),
        "{f}"
    );
    assert!(f.contains("Life Force"), "the player's state: {f}");
    assert!(f.contains("last events"), "{f}");
    // The save and trace so far are still made.
    assert!(out.game.log().len() > 2);
    assert!(out.trace.last().is_some_and(|l| l.contains("failure")));
}

#[test]
fn script_errors_name_the_line() {
    let adv = adventure();
    let e = parse("walk 1\n", &adv).unwrap_err();
    assert!(e.starts_with("line 1:"), "{e}");
    let e = parse("as 1\nuse NOTHING\n", &adv).unwrap_err();
    assert!(e.contains("no thing is named \"NOTHING\""), "{e}");
    let e = parse("wait\nplayer 2 name=X\n", &adv).unwrap_err();
    assert!(e.contains("header"), "{e}");
    // Names and numbers both name things; a trailing square is a target.
    let s = parse("use key 3 4\ndrop LOCKED DOOR e\nready 1\n", &adv).unwrap();
    assert_eq!(s.steps.len(), 3);
}

#[test]
fn a_take_pushes_from_beside_and_the_taker_stays() {
    let adv = adventure();
    // 3,2 is beside the arrival; 3,4 is a walk away.
    let out = run(&adv, "goto 1 0\ntake 3 2\nexpect at 1 0 2 2\ntake 3 4\n", 1);
    assert_eq!(out.failure, None, "{}", out.progress.join("\n"));
    let p = &out.game.players()[0];
    assert_eq!(p.count(KEY as u16), 2);
    let (x, y) = p.at.xy();
    assert!(
        (x as i16 - 3).abs() + (y as i16 - 4).abs() == 1,
        "beside 3,4: {x},{y}"
    );
    // Nothing there: the failure says what the push did.
    let out = run(&adv, "goto 1 0\ntake 4 2\n", 1);
    let f = out.failure.expect("fails");
    assert!(f.contains("nothing was taken at 4,2"), "{f}");
}

#[test]
fn a_joining_players_turn_without_moves_is_waited_out() {
    let adv = adventure();
    // BEN's wounds leave him no capacity, so his turns start with no
    // moves until he heals. No leading `wait` for either player.
    let out = run(
        &adv,
        "player 2 name=BEN strength=1 constitution=20 life=12\n\
         as 2\nwalk 19 21\nexpect at 0 0 19 21\nas 1\nwalk 21 21\nexpect at 0 0 21 21\n",
        2,
    );
    assert_eq!(out.failure, None, "{}", out.progress.join("\n"));
}

#[test]
fn a_portal_into_the_same_room_is_a_door() {
    let mut adv = adventure();
    put(
        &mut adv,
        0,
        1,
        4,
        DOOR,
        Some(Destination::Room {
            region: 1,
            room: 0,
            x: 6,
            y: 4,
        }),
    );
    let out = run(&adv, "goto 1 0\ndoor 1 4\nexpect at 1 0 6 4\n", 1);
    assert_eq!(out.failure, None, "{}", out.progress.join("\n"));
}

/// HALL with an aggressive enemy on 6,4 of Speed `speed` and Life Force
/// `life`, coming for the player.
fn with_brute(speed: u8, life: u8) -> Adventure {
    let mut adv = adventure();
    let mut c = adv.regions[0].residents[0].clone();
    c.in_use = true;
    c.name = "BRUTE".into();
    c.number = 2;
    c.region = 1;
    c.room = 0;
    (c.x, c.y) = (6, 4);
    c.strategy.alignment = Alignment::Enemy;
    c.strategy.aggressive = true;
    c.strategy.brave = true;
    c.speed = speed;
    c.constitution = life;
    c.life_force = life;
    adv.regions[0].residents[1] = c;
    adv
}

const BRUTE: Who = Who::Creature { region: 1, slot: 1 };

#[test]
fn an_attack_follows_the_foe_where_it_goes() {
    let adv = with_brute(3, 6);
    let script = "player 1 possessions=SWORD melee=127 strength=30\ngoto 1 0\nattack 6 4\nexpect alive\n";
    for seed in 1..=4 {
        let out = run(&adv, script, seed);
        assert_eq!(out.failure, None, "seed {seed}: {}", out.progress.join("\n"));
        assert!(!out.game.alive(BRUTE), "seed {seed}: the foe still lives");
        assert!(
            out.trace.iter().any(|l| l.contains("creature_moved")),
            "seed {seed}: the foe never moved"
        );
    }
    // A slow player meets the foe coming for it instead of walking to
    // where it stood: at most 3 squares before the first blow.
    let adv = with_brute(4, 60);
    let script = "player 1 possessions=SWORD melee=80 strength=8 speed=3 life=63 constitution=63\n\
                  goto 1 0\nattack 6 4 3\n";
    for seed in 1..=3 {
        let out = run(&adv, script, seed);
        assert_eq!(out.failure, None, "seed {seed}: {}", out.progress.join("\n"));
        let from = out
            .trace
            .iter()
            .position(|l| l.contains("attack 6 4"))
            .expect("the step");
        let blow = out.trace[from..]
            .iter()
            .position(|l| l.contains("\"attacked\"") && l.contains("\"by\":{\"player\""))
            .expect("the player attacks");
        let moves = out.trace[from..from + blow]
            .iter()
            .filter(|l| l.contains("\"command\":\"move\"") && l.contains("\"event\":\"moved\""))
            .count();
        assert!(moves <= 3, "seed {seed}: {moves} squares before the first blow");
    }
}

/// HALL with one key, in the corner 1,4 behind the arrival: the way to
/// the locked door goes back over squares the search saw without it.
fn key_in_the_corner() -> Adventure {
    let mut adv = adventure();
    clear(&mut adv, 0, 3, 2);
    clear(&mut adv, 0, 3, 4);
    put(&mut adv, 0, 1, 4, KEY, None);
    adv
}

#[test]
fn a_walk_goes_back_over_squares_once_the_room_changed() {
    let adv = key_in_the_corner();
    let out = run(
        &adv,
        "goto 1 0\ndoor 7 2\nexpect owns key\nexpect at 1 1 1 2\n",
        1,
    );
    assert_eq!(out.failure, None, "{}", out.progress.join("\n"));
}

#[test]
fn avoid_takes_keeps_off_carriable_things() {
    let adv = key_in_the_corner();
    let out = run(&adv, "goto 1 0\ndoor 7 2 avoid-takes\n", 1);
    let f = out.failure.expect("fails");
    assert!(f.contains("door 7,2 is unreachable from HALL"), "{f}");
    assert!(f.contains("avoiding takes"), "{f}");
    assert!(f.contains("takes left alone: 1,4 (KEY)"), "{f}");
    // A walk's own target is still taken.
    let out = run(&adv, "goto 1 0\nwalk 1 4 avoid-takes\nexpect owns key\n", 1);
    assert_eq!(out.failure, None, "{}", out.progress.join("\n"));
    // Any other walk leaves the key alone.
    let out = run(&adv, "goto 1 0\nwalk 1 5 avoid-takes\n", 1);
    assert!(out.failure.is_some(), "1,5 is the wall");
    let out = run(
        &adv,
        "goto 1 0\nwalk 2 4 avoid-takes\nwalk 1 3 avoid-takes\nexpect not owns key\n",
        1,
    );
    assert_eq!(out.failure, None, "{}", out.progress.join("\n"));
    assert!(parse("wait 2 avoid-takes\n", &adv).is_err());
}

#[test]
fn a_goto_fallback_stays_in_the_target_region_and_near_the_route() {
    // A HALL door onto the map, and a map door at 22,20 into FINAL: a
    // keyless way to VAULT, but only back over the map. No keys in HALL.
    let keyless = || {
        let mut adv = adventure();
        clear(&mut adv, 0, 3, 2);
        clear(&mut adv, 0, 3, 4);
        adv
    };
    let mut adv = keyless();
    put(
        &mut adv,
        0,
        0,
        3,
        DOOR,
        Some(Destination::WorldMap { x: 20, y: 21 }),
    );
    adv.world.tiles[20][22] = 12;
    adv.world.doors.push(MapDoor {
        index: 1,
        at: MapPoint { x: 22, y: 20 },
        destination: Destination::Room {
            region: 1,
            room: 2,
            x: 1,
            y: 2,
        },
        unknown_y_bit7: false,
    });
    let out = run(&adv, "goto 1 0\ngoto 1 1\n", 1);
    let f = out.failure.expect("fails");
    assert!(f.contains("line 2: goto 1 1"), "{f}");
    assert!(f.contains("without leaving region 1"), "{f}");
    assert!(f.contains("door 7,2 of (1, 0) to (1, 1)"), "{f}");
    assert_eq!(place(&out.game), (1, 0), "still in HALL");

    // A HALL door straight into FINAL: two doors instead of the one that
    // failed is within the detour.
    let mut adv = keyless();
    put(
        &mut adv,
        0,
        0,
        3,
        DOOR,
        Some(Destination::Room {
            region: 1,
            room: 2,
            x: 1,
            y: 2,
        }),
    );
    let out = run(&adv, "goto 1 0\ngoto 1 1\n", 1);
    assert_eq!(out.failure, None, "{}", out.progress.join("\n"));
    assert_eq!(place(&out.game), (1, 1));
}

fn place(g: &Game) -> (u8, u8) {
    match g.players()[0].at {
        Location::Room { region, room, .. } => (region, room),
        Location::World { .. } => (0, 0),
    }
}

#[test]
fn several_expects_read_the_last_actions_messages() {
    let mut adv = adventure();
    put(&mut adv, 0, 2, 3, SCROLL, None);
    let out = run(
        &adv,
        "goto 1 0\nwalk 2 3\nexpect message 7\nexpect at 1 0 2 3\nexpect message 7\nwalk 2 2\nexpect message 7\n",
        1,
    );
    let f = out.failure.expect("fails");
    // The first two read the walk onto the scroll; the third the walk off.
    assert!(f.contains("line 7: expect message 7"), "{f}");
    assert!(f.contains("in the last action step; shown: []"), "{f}");
    assert_eq!(out.game.players()[0].at, room(0, 2, 2));
}

/// The tutorial (docs/content.md 6), built from its source folder, plays
/// to its congratulations from the walkthrough beside it, on several
/// seeds: a corpus-free replay of an adventure of our own.
#[test]
fn first_voyage_plays_to_the_end() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adventures/first-voyage");
    let folder = acs_build::Folder::read(&dir).expect("the tutorial's folder");
    let adv = match acs_build::build(&folder) {
        Ok(b) => b.adventure,
        Err(p) => panic!(
            "{}",
            p.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n")
        ),
    };
    // Its own looks for a new character (characters 2.4), and naming them
    // takes no kit creature in with them.
    assert_eq!(adv.player_pictures.len(), 7);
    assert_eq!(adv.creatures.iter().filter(|c| c.in_use).count(), 1);
    let text = std::fs::read_to_string(dir.join("walkthrough.txt")).unwrap();
    let script = parse(&text, &adv).unwrap_or_else(|e| panic!("{e}"));
    for seed in 1..=8 {
        let out = play(&adv, &script, seed, 1);
        assert_eq!(out.failure, None, "seed {seed}:\n{}", out.progress.join("\n"));
        assert!(out.game.players()[0].alive, "seed {seed}");
    }
}

const BOW: u16 = 8;

/// `with_brute`, the player holding a bow of range 12 (thing 8).
fn with_bow(speed: u8, life: u8) -> Adventure {
    let mut adv = with_brute(speed, life);
    let t = &mut adv.things[BOW as usize - 1];
    t.name = "BOW".into();
    t.kind = ThingKind::MissileWeapon {
        picture: 7,
        disappears_after_use: false,
        carry: Carry { weight: 5, value: 10 },
        stats: WeaponStats {
            power: 10,
            range: Some(12),
            break_chance: 0,
            attack_adjustment: 0,
            usable_by_anyone: true,
            magic: false,
        },
    };
    t.raw = [0; 4];
    adv
}

#[test]
fn a_fire_shoots_the_foe_until_it_dies() {
    // A brute that holds its ground on 6,4 of HALL.
    let mut adv = with_bow(3, 6);
    adv.regions[0].residents[1].strategy.aggressive = false;
    let script =
        "player 1 possessions=BOW missile=127 dexterity=30\ngoto 1 0\nready BOW\nfire 6 4\nexpect alive\n";
    for seed in 1..=4 {
        let out = run(&adv, script, seed);
        assert_eq!(out.failure, None, "seed {seed}: {}", out.progress.join("\n"));
        assert!(!out.game.alive(BRUTE), "seed {seed}: the foe still lives");
        assert!(out.trace.iter().any(|l| l.contains("\"fired\"")), "seed {seed}");
        assert!(
            !out.trace
                .iter()
                .any(|l| l.contains("\"command\":\"move\"") && l.contains("\"attacked\"")),
            "seed {seed}: no blows"
        );
    }
    // N turns: one volley, the foe (Life Force 60) still standing.
    let adv = {
        let mut a = with_bow(3, 60);
        a.regions[0].residents[1].strategy.aggressive = false;
        a
    };
    let out = run(
        &adv,
        "player 1 possessions=BOW\ngoto 1 0\nready BOW\nfire 6 4 1\n",
        1,
    );
    assert_eq!(out.failure, None, "{}", out.progress.join("\n"));
    assert!(out.game.alive(BRUTE));
    assert_eq!(out.trace.iter().filter(|l| l.contains("\"fired\"")).count(), 1);
    assert!(out
        .progress
        .iter()
        .any(|l| l.contains("fire stopped after 1 turns")));
}

#[test]
fn a_refused_fire_fails_with_the_engines_reason() {
    // The brute beside the arrival (2,2): FIRE is refused.
    let mut adv = with_bow(3, 60);
    (adv.regions[0].residents[1].x, adv.regions[0].residents[1].y) = (2, 3);
    let out = run(
        &adv,
        "player 1 possessions=BOW\ngoto 1 0\nready BOW\nfire 2 3\n",
        1,
    );
    let f = out.failure.expect("fails");
    assert!(f.contains("line 4: fire 2 3"), "{f}");
    assert!(
        f.contains("refused: PLAYER 1 may not use a missile weapon while next to BRUTE"),
        "{f}"
    );
    // No missile weapon held: the engine's NONE.
    let mut adv = with_bow(3, 60);
    adv.regions[0].residents[1].strategy.aggressive = false;
    let out = run(&adv, "goto 1 0\nfire 6 4\n", 1);
    let f = out.failure.expect("fails");
    assert!(f.contains("no missile weapon is readied"), "{f}");
    assert!(parse("fire 6\n", &adv).is_err());
}

/// The world map's grass rolls a brute (record 0, on 2,3 of its area) at
/// `chance` percent a step.
fn with_sea_brute(chance: u8) -> Adventure {
    let mut adv = with_brute(3, 6);
    let mut c = adv.regions[0].residents[1].clone();
    adv.regions[0].residents[1].in_use = false;
    c.region = 0;
    adv.world.random_creatures[0] = WorldCreature {
        creature: c,
        terrain: 0,
        chance,
    };
    adv
}

#[test]
fn a_walk_into_an_encounter_stays_in_its_area() {
    let adv = with_sea_brute(60);
    // A plain walk steers round encounters it can't avoid, and fails.
    let out = run(&adv, "walk 25 20\n", 1);
    assert!(out.failure.is_some(), "{}", out.progress.join("\n"));
    let script = "player 1 possessions=SWORD melee=127 strength=30\n\
                  walk 25 20 encounter\nexpect area 1\nexpect area\nattack 2 3\nexpect area 1\nexpect alive\n";
    for seed in 1..=4 {
        let out = run(&adv, script, seed);
        assert_eq!(out.failure, None, "seed {seed}: {}", out.progress.join("\n"));
        let copy = Who::Creature { region: 0, slot: 8 };
        assert!(!out.game.alive(copy), "seed {seed}: the copy still lives");
        // The encounter came on the way east, not behind the player.
        let entered = out
            .trace
            .iter()
            .find(|l| l.contains("\"entered_area\""))
            .expect("entered an area");
        let v: serde_json::Value = serde_json::from_str(entered).unwrap();
        let from = v["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["event"] == "entered_area")
            .map(|e| (e["from"]["x"].as_u64().unwrap(), e["from"]["y"].as_u64().unwrap()))
            .unwrap();
        assert!(
            (21..=25).contains(&from.0) && from.1 == 20,
            "seed {seed}: drawn in off the way at {from:?}"
        );
    }
    // No encounter on the way: the step fails, saying so.
    let out = run(&with_sea_brute(0), "walk 22 20 encounter\n", 1);
    let f = out.failure.expect("fails");
    assert!(f.contains("reached 22,20 with no encounter on the way"), "{f}");
    let out = run(&with_sea_brute(0), "walk 22 20\nexpect area\n", 1);
    assert!(out.failure.expect("fails").contains("in an encounter area"));
    assert!(parse(
        "walk 3 3 avoid-takes encounter\nwalk 3 3 encounter avoid-takes\n",
        &adv
    )
    .is_ok());
    assert!(parse("door 3 3 encounter\n", &adv).is_err());
    assert!(parse("expect area 5\n", &adv).is_err());
}

#[test]
fn a_fire_finds_the_encounter_copy_in_its_area() {
    let mut adv = with_sea_brute(60);
    let t = with_bow(3, 6).things[BOW as usize - 1].clone();
    adv.things[BOW as usize - 1] = t;
    adv.world.random_creatures[0].creature.strategy.aggressive = false;
    let script = "player 1 possessions=BOW missile=127 dexterity=30\nready BOW\n\
                  walk 25 20 encounter\nfire 2 3\nexpect alive\n";
    let out = run(&adv, script, 1);
    assert_eq!(out.failure, None, "{}", out.progress.join("\n"));
    assert!(!out.game.alive(Who::Creature { region: 0, slot: 8 }));
}
