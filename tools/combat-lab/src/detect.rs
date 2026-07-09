//! Balance detectors over the matchup matrix. Each maps to a named failure mode
//! from `keeping-systems-interesting.md`.

use crate::Character;
use crate::resolver::{Duel, Outcome, duel, margin};

/// The full matchup grid.
pub struct Matrix {
    pub names: Vec<String>,
    /// `duels[i][j]` is the duel of i (attacker-perspective) vs j; `None` on the
    /// diagonal.
    pub duels: Vec<Vec<Option<Duel>>>,
    /// `outcome[i][j]` from i's perspective; `Draw` on the diagonal.
    pub outcome: Vec<Vec<Outcome>>,
}

pub fn build_matrix(chars: &[Character]) -> Matrix {
    let n = chars.len();
    let names = chars.iter().map(|c| c.name.clone()).collect();
    let mut duels = vec![vec![None; n]; n];
    let mut outcome = vec![vec![Outcome::Draw; n]; n];
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let d = duel(&chars[i], &chars[j]);
            outcome[i][j] = d.outcome;
            duels[i][j] = Some(d);
        }
    }
    Matrix {
        names,
        duels,
        outcome,
    }
}

pub struct Texture {
    pub walls: u32,
    pub edges: u32,
    pub coinflips: u32,
    pub stalemates: u32,
}

/// Above this TTK ratio a finite matchup counts as a real "edge" rather than a
/// coin-flip.
const EDGE_RATIO: f64 = 1.25;

pub struct Analysis {
    /// (wins, losses, draws) per character.
    pub records: Vec<(u32, u32, u32)>,
    /// `dominated[i]` = `Some(k)` if k weakly dominates i (junk).
    pub dominated: Vec<Option<usize>>,
    pub bosses: Vec<usize>,
    pub doormats: Vec<usize>,
    /// Pairs (i, k) with identical matchup rows.
    pub clones: Vec<(usize, usize)>,
    /// Strongly-connected components of the win-graph (tiers / roundtables).
    pub sccs: Vec<Vec<usize>>,
    pub texture: Texture,
    /// Copeland score (wins − losses) per character; 0 everywhere = balanced.
    pub copeland: Vec<i32>,
    /// Variance of the Copeland scores; the regularity measure (lower = better).
    pub copeland_variance: f64,
    /// Count of infinite-weight edges — ordered pairs where the attacker is
    /// walled (RTK = ∞). The docs' immunity count; ideal is low.
    pub immunities: u32,
    /// Approximate symmetric Nash equilibrium mix (fictitious play).
    pub nash: Vec<f64>,
    /// L2 distance of the Nash mix from uniform; 0 = every build equally played.
    pub nash_distance: f64,
}

impl Analysis {
    pub fn viable(&self) -> Vec<usize> {
        (0..self.records.len())
            .filter(|&i| self.dominated[i].is_none())
            .collect()
    }
}

pub fn analyze(m: &Matrix) -> Analysis {
    let n = m.names.len();

    let mut records = vec![(0u32, 0u32, 0u32); n];
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            match m.outcome[i][j] {
                Outcome::Win => records[i].0 += 1,
                Outcome::Loss => records[i].1 += 1,
                Outcome::Draw => records[i].2 += 1,
            }
        }
    }

    // Weak domination: k dominates i if k scores >= i against *every* opponent —
    // including the head-to-head — and strictly better somewhere. Iterating all
    // columns keeps the diagonal (both Draw) neutral while the j==i / j==k columns
    // enforce that k must also beat-or-draw i directly. Without that, a build that
    // hard-counters k would be wrongly pruned as junk.
    let mut dominated = vec![None; n];
    for i in 0..n {
        for k in 0..n {
            if k == i {
                continue;
            }
            let mut all_ge = true;
            let mut any_gt = false;
            for j in 0..n {
                let sk = m.outcome[k][j].score();
                let si = m.outcome[i][j].score();
                if sk < si {
                    all_ge = false;
                    break;
                }
                if sk > si {
                    any_gt = true;
                }
            }
            if all_ge && any_gt {
                dominated[i] = Some(k);
                break;
            }
        }
    }

    let bosses = (0..n).filter(|&i| records[i].0 as usize == n - 1).collect();
    let doormats = (0..n).filter(|&i| records[i].1 as usize == n - 1).collect();

    // Clones: identical outcome rows (ignoring the diagonal).
    let mut clones = Vec::new();
    for i in 0..n {
        for k in (i + 1)..n {
            if (0..n).all(|j| j == i || j == k || m.outcome[i][j] == m.outcome[k][j]) {
                clones.push((i, k));
            }
        }
    }

    let sccs = strongly_connected(m);

    // Texture over unordered pairs.
    let mut texture = Texture {
        walls: 0,
        edges: 0,
        coinflips: 0,
        stalemates: 0,
    };
    for i in 0..n {
        for j in (i + 1)..n {
            let d = m.duels[i][j].expect("off-diagonal duel");
            match (d.rtk_ab, d.rtk_ba) {
                (None, None) => texture.stalemates += 1,
                (None, Some(_)) | (Some(_), None) => texture.walls += 1,
                (Some(_), Some(_)) => {
                    if margin(&d) > EDGE_RATIO {
                        texture.edges += 1;
                    } else {
                        texture.coinflips += 1;
                    }
                }
            }
        }
    }

    // Copeland scores and their variance (regularity).
    let copeland: Vec<i32> = records
        .iter()
        .map(|&(w, l, _)| w as i32 - l as i32)
        .collect();
    let mean = copeland.iter().sum::<i32>() as f64 / n as f64;
    let copeland_variance = copeland
        .iter()
        .map(|&c| (c as f64 - mean).powi(2))
        .sum::<f64>()
        / n as f64;

    // Immunities: ordered pairs where the attacker can never kill (∞ edge).
    let mut immunities = 0u32;
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            if let Some(d) = m.duels[i][j] {
                if d.rtk_ab.is_none() {
                    immunities += 1;
                }
            }
        }
    }

    let (nash, nash_distance) = nash_equilibrium(m);

    Analysis {
        records,
        dominated,
        bosses,
        doormats,
        clones,
        sccs,
        texture,
        copeland,
        copeland_variance,
        immunities,
        nash,
        nash_distance,
    }
}

/// Approximate the symmetric Nash equilibrium of the win/loss/draw game via
/// single-population fictitious play (converges for symmetric zero-sum games),
/// and report its L2 distance from the uniform mix.
fn nash_equilibrium(m: &Matrix) -> (Vec<f64>, f64) {
    let n = m.names.len();
    if n == 0 {
        return (Vec::new(), 0.0);
    }
    let payoff = |i: usize, j: usize| -> f64 {
        match m.outcome[i][j] {
            Outcome::Win => 1.0,
            Outcome::Loss => -1.0,
            Outcome::Draw => 0.0,
        }
    };
    // Cumulative play counts; best-respond to the empirical mix each step.
    let mut counts = vec![0.0f64; n];
    counts[0] = 1.0;
    let iters = 20_000;
    for _ in 0..iters {
        let mut best = 0;
        let mut best_val = f64::NEG_INFINITY;
        for i in 0..n {
            let mut v = 0.0;
            for j in 0..n {
                if i != j {
                    v += payoff(i, j) * counts[j];
                }
            }
            if v > best_val {
                best_val = v;
                best = i;
            }
        }
        counts[best] += 1.0;
    }
    let total: f64 = counts.iter().sum();
    let p: Vec<f64> = counts.iter().map(|c| c / total).collect();
    let u = 1.0 / n as f64;
    let dist = p.iter().map(|pi| (pi - u).powi(2)).sum::<f64>().sqrt();
    (p, dist)
}

/// Tarjan's SCC over the win-graph (edge i->j iff i beats j).
fn strongly_connected(m: &Matrix) -> Vec<Vec<usize>> {
    let n = m.names.len();
    let mut state = Tarjan {
        m,
        n,
        index: vec![-1; n],
        low: vec![0; n],
        on_stack: vec![false; n],
        stack: Vec::new(),
        next: 0,
        out: Vec::new(),
    };
    for v in 0..n {
        if state.index[v] < 0 {
            state.strong(v);
        }
    }
    state.out
}

struct Tarjan<'a> {
    m: &'a Matrix,
    n: usize,
    index: Vec<i64>,
    low: Vec<i64>,
    on_stack: Vec<bool>,
    stack: Vec<usize>,
    next: i64,
    out: Vec<Vec<usize>>,
}

impl Tarjan<'_> {
    fn strong(&mut self, v: usize) {
        self.index[v] = self.next;
        self.low[v] = self.next;
        self.next += 1;
        self.stack.push(v);
        self.on_stack[v] = true;

        for w in 0..self.n {
            if w == v || self.m.outcome[v][w] != Outcome::Win {
                continue;
            }
            if self.index[w] < 0 {
                self.strong(w);
                self.low[v] = self.low[v].min(self.low[w]);
            } else if self.on_stack[w] {
                self.low[v] = self.low[v].min(self.index[w]);
            }
        }

        if self.low[v] == self.index[v] {
            let mut comp = Vec::new();
            loop {
                let w = self.stack.pop().expect("non-empty SCC stack");
                self.on_stack[w] = false;
                comp.push(w);
                if w == v {
                    break;
                }
            }
            comp.sort_unstable();
            self.out.push(comp);
        }
    }
}

/// Minimum uniform `+delta` to attacker `i`'s weapons for it to beat `j`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breach {
    /// Already wins.
    Already,
    /// Needs `+delta` Strike.
    Needs(u32),
    /// Not breached within the search range.
    Unbounded,
}

pub const BREACH_MAX: u32 = 50;

/// How much Strike `i` must gain to flip its result against `j`. Demonstrates the
/// "walls are relative — every bad matchup is overcomable" invariant.
pub fn breach_by_strike(i: &Character, j: &Character) -> Breach {
    if duel(i, j).outcome == Outcome::Win {
        return Breach::Already;
    }
    for delta in 1..=BREACH_MAX {
        let mut scaled = i.clone();
        scaled.might += delta;
        if duel(&scaled, j).outcome == Outcome::Win {
            return Breach::Needs(delta);
        }
    }
    Breach::Unbounded
}
