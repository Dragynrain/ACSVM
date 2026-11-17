// SPDX-License-Identifier: GPL-3.0-or-later

//! `acs play` on a synthetic adventure built with acs-core's fixture
//! builder. No real adventure data.

use crate::play::play;
use crate::script::parse;
use crate::test_fixture::{Fixture, RegionSpec, RoomSpec};
use acs_core::layout::world_map as wm;
use acs_core::{decode, Adventure, Platform};
use acs_engine::{Location, SaveFile};

const KEY: u8 = 1; // treasure
const LOCKED: u8 = 2; // portal, owners of KEY
const DOOR: u8 = 3; // portal, anyone
const PILLAR: u8 = 4; // obstacle

/// One region, three rooms:
///
/// - HALL 8x6: the map door's arrival at (2,2); keys on (3,2) and (3,4);
///   a locked door (owners of KEY) on the east wall (7,2) into VAULT (1,2).
/// - VAULT 6x6: a door back to HALL (0,2); a door on (5,2) into FINAL
///   (1,2); pillars round (3,4), which no one can reach.
/// - FINAL 5x5: a door back to VAULT (0,2).
///
/// World map: open grass, start (20,20), a map door at (20,19) into HALL.
fn adventure() -> Adventure {
    let mut f = Fixture::new(Platform::C64);
    f.header("PLAYTEST", "", "");
    f.thing(KEY as usize, "KEY", [0x01, 1, 1, 3]);
    f.thing(LOCKED as usize, "LOCKED DOOR", [0x07, 5 | 2 << 6, 0, KEY]);
    f.thing(DOOR as usize, "DOOR", [0x27, 5, 0, 0]);
    f.thing(PILLAR as usize, "PILLAR", [0x0A, 8, 0, 0]);
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
    decode(&f.build()).expect("decodes").adventure
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
        "goto 1 0\ntake 3 2\ndrop key\nexpect not owns key\ndoor 7 2\n",
        &adv,
    )
    .expect("parses");
    let out = play(&adv, &script, 1, 1);
    let f = out.failure.expect("fails");
    assert!(f.contains("line 5: door 7 2"), "{f}");
    assert!(f.contains("door 7,2 is unreachable"), "{f}");
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
