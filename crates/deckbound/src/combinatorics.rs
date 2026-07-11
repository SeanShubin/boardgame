//! Small pure-combinatorics helpers shared by the combat sub-phase machinery and the balance analysis.
//! Kept in the core crate (not `balance`) so nothing in the game depends on the analysis layer.

/// Every **composition** of `n` into `k` ordered non-negative parts (each `Vec` sums to `n`). Used to
/// enumerate how a budget splits across `k` slots (stat allocations, group sizes).
pub fn compositions_k(n: u32, k: usize) -> Vec<Vec<u32>> {
    if k == 0 {
        return if n == 0 { vec![vec![]] } else { vec![] };
    }
    if k == 1 {
        return vec![vec![n]];
    }
    let mut out = Vec::new();
    for first in 0..=n {
        for mut rest in compositions_k(n - first, k - 1) {
            let mut v = Vec::with_capacity(k);
            v.push(first);
            v.append(&mut rest);
            out.push(v);
        }
    }
    out
}
