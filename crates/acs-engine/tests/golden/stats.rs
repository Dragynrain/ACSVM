// SPDX-License-Identifier: GPL-3.0-or-later

//! Comparing the engine's outcomes over many seeds with a capture's counts.
//!
//! A capture's 256-run state sweep tries every value of the original's
//! one-byte random state once, so its counts are the original's own
//! distribution at one moment, quantized to 256ths and bent a little by the
//! states that draw again (docs/capture.md 10.9): where the outcome hangs
//! on the run's first roll, the sweep is all but exhaustive (the cast
//! sweep's four totals are within 3 of 256 x the chance). Where it hangs on
//! later rolls too, each later state follows from the first, so the 256
//! runs are more like 256 draws of a sample.
//!
//! The engine draws from SplitMix64 instead. A golden test runs the engine
//! over a fixed set of seeds (so it never flakes) and compares:
//!
//! - [`Sweep::Exhaustive`]: the capture's rate is taken as the original's
//!   own, give or take [`QUANTUM`]; the engine's may differ from it by its
//!   own [`Z_FAIL`] standard errors on top.
//! - [`Sweep::Sample`]: both sides are samples, a pooled two-proportion
//!   test at [`Z_FAIL`].
//! - Histograms: a chi-square test of homogeneity, failing below [`P_FAIL`].

/// Seeds each statistical golden test runs the engine with.
pub const SEEDS: u64 = 4096;

/// How far an exhaustive sweep's rate may sit from the original's own: the
/// redrawing states bend a 256-sweep by a few runs (capture.md 10.9).
pub const QUANTUM: f64 = 4.0 / 256.0;

/// What a capture's runs were, for [`same_rate`].
#[derive(Clone, Copy, Debug)]
pub enum Sweep {
    /// Every random state once, the outcome hanging on the first roll.
    Exhaustive,
    /// Runs whose outcome hangs on later rolls too, or jitter runs.
    Sample,
}

/// Two proportions differing by more than this many standard errors fail.
pub const Z_FAIL: f64 = 4.0;

/// A histogram whose chi-square p-value is below this fails.
pub const P_FAIL: f64 = 1e-4;

/// The z of two proportions, `a_k` of `a_n` and `b_k` of `b_n`, pooled.
pub fn z(a_k: u64, a_n: u64, b_k: u64, b_n: u64) -> f64 {
    let (ak, an, bk, bn) = (a_k as f64, a_n as f64, b_k as f64, b_n as f64);
    let p = (ak + bk) / (an + bn);
    let se = (p * (1.0 - p) * (1.0 / an + 1.0 / bn)).sqrt();
    if se == 0.0 {
        return if ak / an == bk / bn { 0.0 } else { f64::INFINITY };
    }
    (ak / an - bk / bn) / se
}

/// Asserts the capture's `cap_k` of `cap_n` agrees with the engine's `eng_k`
/// of `eng_n`.
pub fn same_rate(what: &str, sweep: Sweep, cap_k: u64, cap_n: u64, eng_k: u64, eng_n: u64) {
    let (cap, eng) = (cap_k as f64 / cap_n as f64, eng_k as f64 / eng_n as f64);
    let (ok, score) = match sweep {
        Sweep::Sample => {
            let z = z(cap_k, cap_n, eng_k, eng_n);
            (z.abs() <= Z_FAIL, format!("z {z:.2}"))
        }
        Sweep::Exhaustive => {
            let se = (eng * (1.0 - eng) / eng_n as f64).sqrt();
            let limit = Z_FAIL * se + QUANTUM;
            (
                (cap - eng).abs() <= limit,
                format!(
                    "off {:.1} points, limit {:.1}",
                    100.0 * (cap - eng).abs(),
                    100.0 * limit
                ),
            )
        }
    };
    let line = format!(
        "{what}: capture {cap_k}/{cap_n} ({:.1}%), engine {eng_k}/{eng_n} ({:.1}%), {score}",
        100.0 * cap,
        100.0 * eng,
    );
    println!("{line}");
    assert!(ok, "{line}");
}

/// Asserts two histograms over the same bins come from one distribution:
/// a chi-square test of homogeneity on the 2 x k table, after pooling bins
/// whose expected counts are below 5 into their neighbour.
pub fn same_histogram(what: &str, captured: &[u64], engine: &[u64]) {
    assert_eq!(captured.len(), engine.len(), "{what}: bins");
    let (cn, en) = (captured.iter().sum::<u64>(), engine.iter().sum::<u64>());
    assert!(cn > 0 && en > 0, "{what}: empty");
    let total = (cn + en) as f64;
    // Pool small bins, left to right, then the last into the one before.
    let mut bins: Vec<(u64, u64)> = Vec::new();
    let mut acc = (0, 0);
    for (&c, &e) in captured.iter().zip(engine) {
        acc = (acc.0 + c, acc.1 + e);
        let col = (acc.0 + acc.1) as f64;
        if col * cn.min(en) as f64 / total >= 5.0 {
            bins.push(acc);
            acc = (0, 0);
        }
    }
    if acc != (0, 0) {
        match bins.last_mut() {
            Some(last) => *last = (last.0 + acc.0, last.1 + acc.1),
            None => bins.push(acc),
        }
    }
    let mut chi = 0.0;
    for &(c, e) in &bins {
        let col = (c + e) as f64;
        for (obs, n) in [(c, cn), (e, en)] {
            let exp = col * n as f64 / total;
            if exp > 0.0 {
                chi += (obs as f64 - exp).powi(2) / exp;
            }
        }
    }
    let df = bins.len().saturating_sub(1);
    let p = if df == 0 {
        1.0
    } else {
        chi_square_sf(chi, df as f64)
    };
    let line =
        format!("{what}: capture {captured:?}, engine {engine:?}, chi2 {chi:.2} on {df} df, p {p:.2e}");
    println!("{line}");
    assert!(p >= P_FAIL, "{line}");
}

/// P(X > x) for a chi-square of `k` degrees of freedom.
pub fn chi_square_sf(x: f64, k: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    gamma_q(k / 2.0, x / 2.0)
}

/// The regularized upper incomplete gamma function Q(a, x) (Numerical
/// Recipes' series and continued fraction).
fn gamma_q(a: f64, x: f64) -> f64 {
    let gln = ln_gamma(a);
    if x < a + 1.0 {
        let (mut sum, mut del, mut ap) = (1.0 / a, 1.0 / a, a);
        for _ in 0..500 {
            ap += 1.0;
            del *= x / ap;
            sum += del;
            if del.abs() < sum.abs() * 1e-15 {
                break;
            }
        }
        1.0 - sum * (-x + a * x.ln() - gln).exp()
    } else {
        let tiny = 1e-300;
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / tiny;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..500 {
            let an = -(i as f64) * (i as f64 - a);
            b += 2.0;
            d = an * d + b;
            if d.abs() < tiny {
                d = tiny;
            }
            c = b + an / c;
            if c.abs() < tiny {
                c = tiny;
            }
            d = 1.0 / d;
            let del = d * c;
            h *= del;
            if (del - 1.0).abs() < 1e-15 {
                break;
            }
        }
        (-x + a * x.ln() - gln).exp() * h
    }
}

/// ln Γ(x), Lanczos.
fn ln_gamma(x: f64) -> f64 {
    const G: [f64; 6] = [
        76.180_091_729_471_46,
        -86.505_320_329_416_77,
        24.014_098_240_830_91,
        -1.231_739_572_450_155,
        0.001_208_650_973_866_179,
        -0.000_005_395_239_384_953,
    ];
    let mut y = x;
    let tmp = x + 5.5;
    let tmp = tmp - (x + 0.5) * tmp.ln();
    let mut ser = 1.000_000_000_190_015;
    for g in G {
        y += 1.0;
        ser += g / y;
    }
    -tmp + (2.506_628_274_631_000_5 * ser / x).ln()
}

#[test]
fn chi_square_reference_values() {
    // Tables: P(chi2_1 > 3.841) = 0.05, P(chi2_4 > 13.277) = 0.01,
    // P(chi2_10 > 29.588) = 0.001.
    for (x, k, p) in [(3.841, 1.0, 0.05), (13.277, 4.0, 0.01), (29.588, 10.0, 0.001)] {
        let got = chi_square_sf(x, k);
        assert!((got - p).abs() < p * 0.01, "{x} {k}: {got}");
    }
}

#[test]
fn the_limits_pass_the_256ths_and_catch_a_few_points() {
    // The cast sweep's four totals against their exact chances pass.
    for (k, p) in [(231, 0.90), (74, 0.30), (50, 0.20), (117, 0.45)] {
        same_rate("cast", Sweep::Exhaustive, k, 256, (p * 4096.0) as u64, 4096);
    }
    // 74 of 256 (28.9%) against an engine at 23%: fails.
    let r = std::panic::catch_unwind(|| same_rate("off", Sweep::Exhaustive, 74, 256, 942, 4096));
    assert!(r.is_err());
}
