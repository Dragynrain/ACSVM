// SPDX-License-Identifier: GPL-3.0-or-later

//! Room and region layout (`acs_core::scene`) on synthetic adventures.

mod common;

use acs_core::scene::{self, Base, GridRect, RoomScene, Shown};
use acs_core::{decode, Platform};
use common::{creature_record, Fixture, RegionSpec, RoomSpec};

fn fixture(platform: Platform) -> Fixture {
    let mut f = Fixture::new(platform);
    f.header("SCENES", "", "");
    f.thing(1, "COVER", [0x08, 2, 0, 0]); // space, picture 2
    f.thing(2, "CHEST", [0x01, 1, 1, 3]); // treasure, picture 3 (byte 3)
    f.thing(3, "DOOR", [0x07, 4, 0, 0]); // portal, picture 4
    f.thing(4, "ZAP", [0x06, 0, 0, 0]); // magic spell: no picture
    f.thing(127, "EMPTY FLOOR", [0x00, 1, 0, 0]);
    f.thing(128, "ROOM BORDER", [0x00, 9, 0, 0]);
    let mut guard = creature_record(10, 0x50, 70);
    guard[0x0D] = 2; // x
    guard[0x0E] = 1; // y
    guard[0x10] = 0x11; // region 1, room 1
    f.region(RegionSpec {
        name: "KEEP",
        rooms: vec![
            RoomSpec {
                name: "HALL",
                w: 5,
                h: 4,
                x: 10,
                y: 3,
                wall: 9,
                items: vec![
                    (2, 2, 1, 0), // cover on top of the chest
                    (2, 2, 2, 0),
                    (4, 1, 3, 1), // door in the east wall
                    (1, 1, 4, 0), // spell, then chest: the chest shows
                    (1, 1, 2, 0),
                    (15, 14, 2, 0), // stash square, off the room
                ],
                ..Default::default()
            },
            RoomSpec {
                name: "CELL",
                w: 4,
                h: 4,
                x: 14,
                y: 3,
                wall: 5,
                items: vec![(1, 1, 2, 0)],
                ..Default::default()
            },
        ],
        portals: vec![[0, 0], [0x11, 0x11]],
        residents: vec![(0, "GUARD", guard)],
        ..Default::default()
    });
    f
}

#[test]
fn room_squares_on_both_platforms() {
    for p in [Platform::Pc, Platform::C64] {
        let adv = decode(&fixture(p).build()).unwrap().adventure;
        let region = &adv.regions[0];
        let hall = &region.rooms[0];
        let s = RoomScene::new(region, hall);
        assert_eq!((s.width, s.height, s.squares.len()), (5, 4, 20), "{p}");

        // Edges are walls, the inside is floor.
        assert_eq!(s.square(0, 0).unwrap().base, Base::Wall);
        assert_eq!(s.square(4, 2).unwrap().base, Base::Wall);
        assert_eq!(s.square(1, 1).unwrap().base, Base::Floor);
        assert_eq!(s.square(3, 2).unwrap().base, Base::Floor);
        assert_eq!(s.square(1, 3).unwrap().base, Base::Wall);

        // The first listed item is the top of the stack.
        let stack = s.square(2, 2).unwrap();
        assert_eq!(stack.items, vec![0, 1]);
        assert_eq!(scene::shown(&adv, region, hall, stack), (Shown::Item(0), Some(2)));
        // A picture-less spell on top lets the next item show.
        let sq = s.square(1, 1).unwrap();
        assert_eq!(scene::shown(&adv, region, hall, sq), (Shown::Item(4), Some(3)));
        // A door replaces the wall picture.
        let sq = s.square(4, 1).unwrap();
        assert_eq!(scene::shown(&adv, region, hall, sq), (Shown::Item(2), Some(4)));
        // Bare wall and floor.
        let sq = s.square(0, 0).unwrap();
        assert_eq!(scene::shown(&adv, region, hall, sq), (Shown::Wall, Some(9)));
        let sq = s.square(3, 2).unwrap();
        assert_eq!(scene::shown(&adv, region, hall, sq), (Shown::Floor, Some(1)));
        // The stash square is outside every room.
        assert_eq!(s.off_room, vec![5]);
        assert!(s.squares.iter().all(|q| q.residents.is_empty()));

        // The resident stands in room 1 at 2,1, above everything.
        let cell = &region.rooms[1];
        let c = RoomScene::new(region, cell);
        let sq = c.square(2, 1).unwrap();
        assert_eq!(sq.residents, vec![0]);
        assert_eq!(
            scene::shown(&adv, region, cell, sq),
            (Shown::Resident(0), Some(70))
        );
        assert_eq!(c.square(1, 1).unwrap().items, vec![0]);
    }
}

#[test]
fn missing_floor_thing_has_no_picture() {
    let mut f = fixture(Platform::Pc);
    f.thing(127, "", [0x0F, 0, 0, 0]); // undefined slot
    let adv = decode(&f.build()).unwrap().adventure;
    let region = &adv.regions[0];
    let hall = &region.rooms[0];
    let s = RoomScene::new(region, hall);
    let sq = s.square(3, 2).unwrap();
    assert_eq!(scene::shown(&adv, region, hall, sq), (Shown::Floor, None));
}

#[test]
fn region_layout_bounds_rooms_sharing_a_wall() {
    let adv = decode(&fixture(Platform::C64).build()).unwrap().adventure;
    let (rooms, bounds) = scene::region_layout(&adv.regions[0]);
    assert_eq!(
        rooms,
        vec![
            (
                0,
                GridRect {
                    x: 10,
                    y: 3,
                    w: 5,
                    h: 4
                }
            ),
            (
                1,
                GridRect {
                    x: 14,
                    y: 3,
                    w: 4,
                    h: 4
                }
            ),
        ]
    );
    // HALL's east wall (x 14) is CELL's west wall.
    assert_eq!(
        bounds,
        Some(GridRect {
            x: 10,
            y: 3,
            w: 8,
            h: 4
        })
    );
    assert!(bounds.unwrap().contains(17, 6));
    assert!(!bounds.unwrap().contains(18, 6));
}
