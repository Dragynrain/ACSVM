// SPDX-License-Identifier: GPL-3.0-or-later

//! Smoothed render mode's upscaler (client 5): Scale2x, Andrea Mazzoleni's
//! AdvMAME2x rules, written from the published description. Each pixel E
//! becomes four; a corner takes the colour of its two neighbours where they
//! agree and the other two differ, so staircases become diagonals and no
//! new colour appears.

/// One Scale2x pass over a `w` x `h` image of RGBA pixels: `2w` x `2h`.
/// Neighbours past the edge repeat the edge pixel.
pub fn scale2x(src: &[[u8; 4]], w: usize, h: usize) -> Vec<[u8; 4]> {
    assert_eq!(src.len(), w * h);
    let at = |x: isize, y: isize| {
        src[y.clamp(0, h as isize - 1) as usize * w + x.clamp(0, w as isize - 1) as usize]
    };
    let mut out = vec![[0u8; 4]; 4 * w * h];
    for y in 0..h as isize {
        for x in 0..w as isize {
            let e = at(x, y);
            let (b, d, f, hh) = (at(x, y - 1), at(x - 1, y), at(x + 1, y), at(x, y + 1));
            let corners = if b != hh && d != f {
                [
                    if d == b { d } else { e },
                    if b == f { f } else { e },
                    if d == hh { d } else { e },
                    if hh == f { f } else { e },
                ]
            } else {
                [e; 4]
            };
            let (ox, oy) = (2 * x as usize, 2 * y as usize);
            out[oy * 2 * w + ox] = corners[0];
            out[oy * 2 * w + ox + 1] = corners[1];
            out[(oy + 1) * 2 * w + ox] = corners[2];
            out[(oy + 1) * 2 * w + ox + 1] = corners[3];
        }
    }
    out
}

/// Scale2x twice: 4x each way (client 5).
pub fn scale4x(src: &[[u8; 4]], w: usize, h: usize) -> Vec<[u8; 4]> {
    scale2x(&scale2x(src, w, h), 2 * w, 2 * h)
}

#[cfg(test)]
mod tests {
    use super::*;

    const K: [u8; 4] = [0, 0, 0, 255];
    const W: [u8; 4] = [255, 255, 255, 255];

    #[test]
    fn a_flat_image_stays_flat_and_doubles() {
        let out = scale2x(&[W; 6], 3, 2);
        assert_eq!(out.len(), 24);
        assert!(out.iter().all(|&p| p == W));
        assert_eq!(scale4x(&[K; 4], 2, 2).len(), 64);
    }

    #[test]
    fn a_lone_pixel_stays_a_square() {
        // Its neighbours all agree, so no corner changes.
        let mut img = [W; 9];
        img[4] = K;
        let out = scale2x(&img, 3, 3);
        let px = |x: usize, y: usize| out[y * 6 + x];
        assert_eq!([px(2, 2), px(3, 2), px(2, 3), px(3, 3)], [K; 4]);
        assert_eq!(out.iter().filter(|&&p| p == K).count(), 4);
    }

    #[test]
    fn a_staircase_becomes_a_diagonal() {
        // K W
        // W W  -> the black pixel's inner corner turns white.
        let out = scale2x(&[K, W, W, W], 2, 2);
        let px = |x: usize, y: usize| out[y * 4 + x];
        assert_eq!([px(0, 0), px(1, 0), px(0, 1), px(1, 1)], [K, K, K, W]);
        // No new colours, ever.
        assert!(out.iter().all(|&p| p == K || p == W));
    }
}
