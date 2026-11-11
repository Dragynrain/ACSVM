// SPDX-License-Identifier: GPL-3.0-or-later

//! Measurements read off the rendered frame (docs/capture.md section 5).
//! Everything here looks at pixels only, the way a player looks at the screen.

use crate::screen::Frame;
use serde::Serialize;

/// An axis-aligned box in inner-screen pixels, inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Box2 {
    pub x0: usize,
    pub y0: usize,
    pub x1: usize,
    pub y1: usize,
    pub pixels: usize,
}

impl Box2 {
    pub fn w(&self) -> usize {
        self.x1 - self.x0 + 1
    }
    pub fn h(&self) -> usize {
        self.y1 - self.y0 + 1
    }
}

/// Connected regions (4-neighbour) of palette index `colour` inside
/// `area` (x, y, w, h of the inner screen), as bounding boxes, largest first.
pub fn blobs(f: &Frame, colour: u8, area: (usize, usize, usize, usize)) -> Vec<Box2> {
    let (ax, ay, aw, ah) = area;
    let (_, _, iw, ih) = f.inner();
    let aw = aw.min(iw.saturating_sub(ax));
    let ah = ah.min(ih.saturating_sub(ay));
    let mut seen = vec![false; aw * ah];
    let mut out = Vec::new();
    for sy in 0..ah {
        for sx in 0..aw {
            if seen[sy * aw + sx] || f.at(ax + sx, ay + sy) != colour {
                continue;
            }
            let mut b = Box2 {
                x0: sx,
                y0: sy,
                x1: sx,
                y1: sy,
                pixels: 0,
            };
            let mut stack = vec![(sx, sy)];
            seen[sy * aw + sx] = true;
            while let Some((x, y)) = stack.pop() {
                b.pixels += 1;
                b.x0 = b.x0.min(x);
                b.y0 = b.y0.min(y);
                b.x1 = b.x1.max(x);
                b.y1 = b.y1.max(y);
                let mut push = |nx: usize, ny: usize| {
                    let i = ny * aw + nx;
                    if !seen[i] && f.at(ax + nx, ay + ny) == colour {
                        seen[i] = true;
                        stack.push((nx, ny));
                    }
                };
                if x > 0 {
                    push(x - 1, y);
                }
                if y > 0 {
                    push(x, y - 1);
                }
                if x + 1 < aw {
                    push(x + 1, y);
                }
                if y + 1 < ah {
                    push(x, y + 1);
                }
            }
            b.x0 += ax;
            b.x1 += ax;
            b.y0 += ay;
            b.y1 += ay;
            out.push(b);
        }
    }
    out.sort_by_key(|b| std::cmp::Reverse(b.pixels));
    out
}
