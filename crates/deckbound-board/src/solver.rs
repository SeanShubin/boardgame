//! **Exact v2 winnability solver.** Answers "can the party *force* a win against the scripted greedy foe?"
//! by searching every party line (formation + per-mini-phase allocations) and the deterministic greedy foe
//! response through the [`crate::combat`] resolvers. The foe is a fixed policy, so this is a single-agent
//! reachability search: the party is winnable iff *some* sequence of its choices reaches a party-win state.
//!
//! Tractability rests on the v2 structure (memory `combat-frozen-revisit-after-tooling`): resolution is
//! order-free and deterministic, targets are SCHEDULE-gated, and bids are **threshold contests** — so the
//! only strike bids worth trying are `min-to-land` and `min-to-deny-evade`, not the whole `0..tempo` range.
//! States are memoized at sub-phase boundaries. Exponential in party size in the worst case; trivial for the
//! solo duel-locks matchups, which is what the diagonal balance property needs.

use std::collections::HashMap;

use deckbound_content::rank::Intention as Rank;
use deckbound_content::schedule::SCHEDULE;

use crate::battle::{Greedy, MAX_ROUNDS, Policy};
use crate::combat::{self, Blows, Combatant, Contact, Dodge, Engage, Side};

// ---- the oracle: a budgeted, resumable search a frame can afford ----------------------------------------

/// What the solver says about a position - the three states, exactly as named.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Some line from here wins.
    Winnable,
    /// The search ran out of budget. Not an answer - an answer *in progress*.
    Evaluating,
    /// The tree is exhausted and no line wins.
    Doomed,
}

/// The party's already-staged orders at the current step. The search must **honour them**: taking a choice
/// only stages it, so "does this choice still win?" means "is there some completion of the orders I have not
/// given yet that wins, given the ones I have?"
#[derive(Clone, Debug, Default)]
pub struct Fixed {
    /// Per unit: `Some(order)` pins that unit's engagement (`Some(None)` = it Holds); `None` = free to choose.
    pub engage: Vec<Option<Option<Engage>>>,
    /// Per unit: its dodge, if the player has answered for it.
    pub dodge: Vec<Option<Dodge>>,
    /// Per unit: how many blows it pours in, if the player has said.
    pub blows: Vec<Option<u32>>,
}

impl Fixed {
    pub fn empty(n: usize) -> Self {
        Fixed {
            engage: vec![None; n],
            dodge: vec![None; n],
            blows: vec![None; n],
        }
    }
}

/// Which mini-phase the position is at - the arena's `Step`, without the arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    Engage,
    Evade,
    Strike,
}

/// **The doom oracle.** One per fight: it holds the memo, so the first evaluation walks the tree and every
/// later one is a lookup - which is the whole reason this is affordable at all.
///
/// The search is **budgeted and restartable** rather than incremental: give it a node budget, and if it runs
/// out it answers [`Verdict::Evaluating`] and abandons the walk. Next call it starts again - but the memo
/// survives, so it gets much further, and it converges in a handful of frames. That buys a resumable search
/// without an explicit stack, and it runs identically on native and WebAssembly (no threads, no shared memory,
/// no COOP/COEP headers on the deploy).
///
/// The one rule it must never break: an aborted subtree is **not** memoized. A "no win found" that was really
/// "no win found *yet*" would poison the table permanently, and the indicator would say doomed forever.
#[derive(Default)]
pub struct Oracle {
    memo: HashMap<Key, bool>,
    /// Nodes visited over the fight - the honest measure of what this costs.
    pub nodes: u64,
}

impl Oracle {
    pub fn new() -> Self {
        Oracle::default()
    }

    /// How many positions it has scored and remembered.
    pub fn known(&self) -> usize {
        self.memo.len()
    }

    /// Can the party still force a win from here, honouring `fixed`? `budget` is a node allowance, decremented
    /// as it searches; [`Verdict::Evaluating`] means it ran out.
    #[allow(clippy::too_many_arguments)] // a position IS this many things; bundling them would only hide it
    pub fn score(
        &mut self,
        units: &[Combatant],
        round: usize,
        sub: usize,
        at: At,
        contacts: &[Contact],
        fixed: &Fixed,
        budget: u64,
    ) -> Verdict {
        let mut b = Budget {
            left: budget,
            spent: 0,
        };
        let r = match at {
            At::Engage => search_engage_c(units, round, sub, &mut self.memo, fixed, &mut b),
            At::Evade => search_evade_c(units, contacts, round, sub, &mut self.memo, fixed, &mut b),
            At::Strike => {
                search_strike_c(units, contacts, round, sub, &mut self.memo, fixed, &mut b)
            }
        };
        self.nodes += b.spent;
        match r {
            Some(true) => Verdict::Winnable,
            Some(false) => Verdict::Doomed,
            None => Verdict::Evaluating,
        }
    }
}

/// What it cost to map a fight out completely.
#[derive(Clone, Copy, Debug, Default)]
pub struct MapCost {
    /// Positions actually *evaluated* (a memo hit costs nothing).
    pub nodes: u64,
    /// Distinct positions the memo now holds - the memory an in-app oracle would carry.
    pub states: usize,
    /// Formations enumerated at the Marshal.
    pub formations: usize,
    /// Whether any line at all wins.
    pub winnable: bool,
}

/// **Map the whole fight out, from the initial Marshal.** Every formation, every allocation, every line, to the
/// end of every branch - and *no short-circuit anywhere*, even after a win is found.
///
/// This is the ceiling, not the working cost. The doom oracle stops a subtree the moment it finds a win, and it
/// starts *after* the formation is chosen. This starts at the Marshal, so the 3^heroes formation fan-out is in
/// the tree - which is precisely the cost a Marshal-screen indicator would have to pay, and the reason that one
/// was deferred rather than built.
pub fn map_out(party: &[Combatant], foes: &[Combatant]) -> MapCost {
    let n = party.len();
    let mut memo: HashMap<Key, bool> = HashMap::new();
    let mut nodes = 0u64;
    let mut winnable = false;
    let formations = 3usize.pow(n as u32);
    for f in 0..formations {
        let units = formed(party, foes, f);
        // No `||=` short-circuit: `explore` must run for every formation, or it is not a map.
        let win = explore(&units, 0, 0, &mut memo, &mut nodes);
        winnable |= win;
    }
    MapCost {
        nodes,
        states: memo.len(),
        formations,
        winnable,
    }
}

/// Map out **one** formation, from a fresh memo — so it stands alone.
///
/// The whole-fight [`map_out`] shares one memo across all the formations, which makes the first expensive and
/// the rest look cheap. That flatters whichever one happens to go first, and it hides the number you actually
/// need: *how long does the **worst** formation take, on its own?* That is the frame-hitch risk, and it is what
/// a Marshal-screen indicator would pay per formation it evaluates.
pub fn map_out_formation(party: &[Combatant], foes: &[Combatant], formation: usize) -> MapCost {
    let units = formed(party, foes, formation);
    let mut memo: HashMap<Key, bool> = HashMap::new();
    let mut nodes = 0u64;
    let winnable = explore(&units, 0, 0, &mut memo, &mut nodes);
    MapCost {
        nodes,
        states: memo.len(),
        formations: 1,
        winnable,
    }
}

/// The party ranked by formation index `f` (base-3 over the heroes), plus the foes.
fn formed(party: &[Combatant], foes: &[Combatant], f: usize) -> Vec<Combatant> {
    let mut units: Vec<Combatant> = party
        .iter()
        .enumerate()
        .map(|(k, p)| {
            let mut u = p.clone();
            u.rank = RANKS[(f / 3usize.pow(k as u32)) % 3];
            u
        })
        .collect();
    units.extend(foes.iter().cloned());
    units
}

/// How the party is ranked in formation `f` — so a probe can *name* the worst one rather than just number it.
pub fn formation_ranks(party_len: usize, f: usize) -> Vec<Rank> {
    (0..party_len)
        .map(|k| RANKS[(f / 3usize.pow(k as u32)) % 3])
        .collect()
}

/// [`forces_win`] with **every** branch visited - it never returns early on a win. The verdict is the same; the
/// difference is that the memo comes out *complete*, which is what "mapped out" means.
fn explore(
    units: &[Combatant],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
    nodes: &mut u64,
) -> bool {
    if let Some(done) = party_won(units) {
        return done;
    }
    if round >= MAX_ROUNDS {
        return false;
    }
    let key = key_of(units, round, sub);
    if let Some(&r) = memo.get(&key) {
        return r; // already mapped - the whole point
    }
    *nodes += 1;
    let mut u = units.to_vec();
    if sub == 0 {
        combat::refresh_round(&mut u);
    }

    let foe_eng = Greedy.engagements(&u, Side::Foe, sub);
    let mut win = false;
    for_each_combo(&party_engage_options(&u, sub), &mut |chosen| {
        let mut a = u.to_vec();
        let mut all: Vec<Engage> = chosen.iter().flatten().copied().collect();
        all.extend(foe_eng.iter().copied());
        let reaching = combat::resolve_engage(&mut a, &all);

        let reached: Vec<usize> = (0..a.len())
            .filter(|&i| {
                a[i].side == Side::Party
                    && !a[i].fallen
                    && combat::slip_cost(&a, &reaching, i).is_some_and(|c| c <= a[i].tempo)
            })
            .collect();
        let dodge_opts: Vec<Vec<Dodge>> = reached
            .iter()
            .map(|_| vec![Dodge::Stand, Dodge::Slip])
            .collect();
        for_each_combo(&dodge_opts, &mut |picks| {
            let dodges: Vec<Dodge> = (0..a.len())
                .map(|i| match reached.iter().position(|&r| r == i) {
                    Some(pos) => picks[pos],
                    None if a[i].side == Side::Foe => Greedy.dodge(&a, &reaching, i),
                    None => Dodge::Stand,
                })
                .collect();
            let mut b = a.to_vec();
            let contacts = combat::resolve_evade(&mut b, &reaching, &dodges);

            let foe_blows = Greedy.blows(&b, Side::Foe, &contacts);
            let edges: Vec<(usize, usize)> = (0..b.len())
                .filter(|&i| b[i].side == Side::Party && !b[i].fallen && b[i].tempo > 0)
                .filter_map(|i| combat::strike_target(&b, &contacts, i).map(|t| (i, t)))
                .collect();
            let blow_opts: Vec<Vec<Blows>> = edges
                .iter()
                .map(|&(i, target)| {
                    (0..=b[i].tempo)
                        .map(|cards| Blows {
                            unit: i,
                            target,
                            cards,
                        })
                        .collect()
                })
                .collect();
            let (nr, ns) = next(round, sub);
            for_each_combo(&blow_opts, &mut |bs| {
                let mut c = b.to_vec();
                let mut blows: Vec<Blows> = bs.iter().filter(|x| x.cards > 0).copied().collect();
                blows.extend(foe_blows.iter().copied());
                combat::resolve_strike(&mut c, &contacts, &blows);
                combat::end_sub_phase(&mut c);
                win |= explore(&c, nr, ns, memo, nodes);
            });
        });
    });
    memo.insert(key, win);
    win
}

/// Visit **every** combination. The deliberate opposite of [`any_combo`], which stops at the first that wins.
fn for_each_combo<T: Clone>(options: &[Vec<T>], f: &mut dyn FnMut(&[T])) {
    fn go<T: Clone>(options: &[Vec<T>], i: usize, acc: &mut Vec<T>, f: &mut dyn FnMut(&[T])) {
        if i == options.len() {
            f(acc);
            return;
        }
        for opt in &options[i] {
            acc.push(opt.clone());
            go(options, i + 1, acc, f);
            acc.pop();
        }
    }
    let mut acc = Vec::new();
    go(options, 0, &mut acc, f);
}

/// A node allowance. `None` from any search means it was exhausted - **not** that no win exists.
struct Budget {
    left: u64,
    spent: u64,
}

impl Budget {
    /// Charge one node. `false` once the allowance is gone.
    fn tick(&mut self) -> bool {
        if self.left == 0 {
            return false;
        }
        self.left -= 1;
        self.spent += 1;
        true
    }
}

/// Fold the per-unit options with the orders the player has already staged: a pinned unit gets exactly its
/// order, a free one gets everything it could do.
fn constrain<T: Clone>(free: Vec<Vec<T>>, idx: &[usize], fixed: &[Option<T>]) -> Vec<Vec<T>> {
    free.into_iter()
        .enumerate()
        .map(|(k, opts)| match idx.get(k).and_then(|&i| fixed.get(i)) {
            Some(Some(pinned)) => vec![pinned.clone()],
            _ => opts,
        })
        .collect()
}

/// `any_combo`, budgeted. `Some(true)` = a winning line; `Some(false)` = every line loses (exhaustive);
/// `None` = the budget ran out with no win found, so nothing is known.
fn any_combo_b<T: Clone>(
    options: &[Vec<T>],
    b: &mut Budget,
    f: &mut dyn FnMut(&[T], &mut Budget) -> Option<bool>,
) -> Option<bool> {
    fn go<T: Clone>(
        options: &[Vec<T>],
        i: usize,
        acc: &mut Vec<T>,
        b: &mut Budget,
        f: &mut dyn FnMut(&[T], &mut Budget) -> Option<bool>,
        unknown: &mut bool,
    ) -> bool {
        if i == options.len() {
            match f(acc, b) {
                Some(true) => return true,
                Some(false) => {}
                None => *unknown = true,
            }
            return false;
        }
        for opt in &options[i] {
            acc.push(opt.clone());
            let win = go(options, i + 1, acc, b, f, unknown);
            acc.pop();
            if win {
                return true;
            }
        }
        false
    }
    let mut unknown = false;
    let mut acc = Vec::new();
    if go(options, 0, &mut acc, b, f, &mut unknown) {
        return Some(true);
    }
    if unknown { None } else { Some(false) }
}

/// [`forces_win`], budgeted, with the memo shared across the whole fight.
fn forces_win_b(
    units: &[Combatant],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
    b: &mut Budget,
) -> Option<bool> {
    if let Some(done) = party_won(units) {
        return Some(done);
    }
    if round >= MAX_ROUNDS {
        return Some(false); // the round cap, undecided = a draw, not a win
    }
    let key = key_of(units, round, sub);
    if let Some(&r) = memo.get(&key) {
        return Some(r);
    }
    if !b.tick() {
        return None; // out of budget - the caller must not treat this as "no win"
    }
    let mut u = units.to_vec();
    if sub == 0 {
        combat::refresh_round(&mut u);
    }
    let win = search_engage_c(&u, round, sub, memo, &Fixed::empty(u.len()), b)?;
    // Only a DEFINITE answer is remembered. Memoizing an aborted subtree as `false` would poison the table
    // permanently, and the indicator would read doomed forever.
    memo.insert(key, win);
    Some(win)
}

fn search_engage_c(
    units: &[Combatant],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
    fixed: &Fixed,
    b: &mut Budget,
) -> Option<bool> {
    let foe = Greedy.engagements(units, Side::Foe, sub);
    let idx: Vec<usize> = party_engage_idx(units);
    let options = constrain(party_engage_options(units, sub), &idx, &fixed.engage);
    any_combo_b(&options, b, &mut |chosen, b| {
        let mut u = units.to_vec();
        let mut all: Vec<Engage> = chosen.iter().flatten().copied().collect();
        all.extend(foe.iter().copied());
        let reaching = combat::resolve_engage(&mut u, &all);
        // Only the CURRENT step is constrained by what the player has staged; everything after is free.
        search_evade_c(
            &u,
            &reaching,
            round,
            sub,
            memo,
            &Fixed {
                engage: vec![None; u.len()],
                dodge: fixed.dodge.clone(),
                blows: fixed.blows.clone(),
            },
            b,
        )
    })
}

fn search_evade_c(
    units: &[Combatant],
    reaching: &[Contact],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
    fixed: &Fixed,
    b: &mut Budget,
) -> Option<bool> {
    let reached: Vec<usize> = (0..units.len())
        .filter(|&i| {
            units[i].side == Side::Party
                && !units[i].fallen
                && combat::slip_cost(units, reaching, i).is_some_and(|c| c <= units[i].tempo)
        })
        .collect();
    let free: Vec<Vec<Dodge>> = reached
        .iter()
        .map(|_| vec![Dodge::Stand, Dodge::Slip])
        .collect();
    let options = constrain(free, &reached, &fixed.dodge);
    any_combo_b(&options, b, &mut |chosen, b| {
        let dodges: Vec<Dodge> = (0..units.len())
            .map(|i| match reached.iter().position(|&r| r == i) {
                Some(pos) => chosen[pos],
                None if units[i].side == Side::Foe => Greedy.dodge(units, reaching, i),
                None => Dodge::Stand,
            })
            .collect();
        let mut u = units.to_vec();
        let contacts = combat::resolve_evade(&mut u, reaching, &dodges);
        search_strike_c(
            &u,
            &contacts,
            round,
            sub,
            memo,
            &Fixed {
                engage: vec![None; u.len()],
                dodge: vec![None; u.len()],
                blows: fixed.blows.clone(),
            },
            b,
        )
    })
}

fn search_strike_c(
    units: &[Combatant],
    contacts: &[Contact],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
    fixed: &Fixed,
    b: &mut Budget,
) -> Option<bool> {
    let foe = Greedy.blows(units, Side::Foe, contacts);
    let edges: Vec<(usize, usize)> = (0..units.len())
        .filter(|&i| units[i].side == Side::Party && !units[i].fallen && units[i].tempo > 0)
        .filter_map(|i| combat::strike_target(units, contacts, i).map(|t| (i, t)))
        .collect();
    let idx: Vec<usize> = edges.iter().map(|&(i, _)| i).collect();
    let free: Vec<Vec<Blows>> = edges
        .iter()
        .map(|&(i, target)| {
            (0..=units[i].tempo)
                .map(|cards| Blows {
                    unit: i,
                    target,
                    cards,
                })
                .collect()
        })
        .collect();
    // The staged blow count, turned into the one option that unit is allowed.
    let pinned: Vec<Option<Blows>> = (0..units.len())
        .map(|i| {
            let cards = fixed.blows.get(i).copied().flatten()?;
            let (_, target) = edges.iter().find(|&&(u, _)| u == i)?;
            Some(Blows {
                unit: i,
                target: *target,
                cards,
            })
        })
        .collect();
    let options = constrain(free, &idx, &pinned);
    let (nr, ns) = next(round, sub);
    any_combo_b(&options, b, &mut |chosen, b| {
        let mut u = units.to_vec();
        let mut blows: Vec<Blows> = chosen.iter().filter(|x| x.cards > 0).copied().collect();
        blows.extend(foe.iter().copied());
        combat::resolve_strike(&mut u, contacts, &blows);
        combat::end_sub_phase(&mut u);
        forces_win_b(&u, nr, ns, memo, b)
    })
}

/// The party units `party_engage_options` produces an option list for, in the same order.
fn party_engage_idx(units: &[Combatant]) -> Vec<usize> {
    (0..units.len())
        .filter(|&i| {
            units[i].side == Side::Party
                && !units[i].fallen
                && units[i].tempo > 0
                && combat::effective_in_rank(units[i].rank, units[i].melee, units[i].ranged)
        })
        .collect()
}

/// The three ranks a party unit may be assigned (the formation search space).
const RANKS: [Rank; 3] = [Rank::Vanguard, Rank::Outrider, Rank::Rearguard];

/// Whether the party can **force a win** vs the scripted greedy foe. `party` come rank-less (each formation
/// overwrites their `rank`); `foes` keep their scripted ranks. The party picks one formation for the battle
/// (round-0 ranks, held — a sufficient condition: if some fixed formation wins, the party wins).
pub fn winnable(party: &[Combatant], foes: &[Combatant]) -> bool {
    winnable_traced(party, foes, false).0
}

/// Whether the party can force a win **when it may re-Marshal every round** — the rule the *game* actually
/// plays, as against [`winnable`]'s fixed-formation assumption.
///
/// The game unranks the foes at each Reset and lets you move your heroes, so a position a fixed-formation
/// search calls lost may be winnable by re-ranking next round. If the two ever disagree, the fixed search is
/// **wrong about the game** - and a "doomed" verdict built on it would tell you to abandon a fight you could
/// still win, which is the one failure mode a certainty indicator may never have.
pub fn winnable_remarshal(party: &[Combatant], foes: &[Combatant]) -> bool {
    winnable_traced(party, foes, true).0
}

/// [`winnable`] / [`winnable_remarshal`], reporting the size of the memo table it built — the honest measure
/// of what an in-app solver would have to hold.
pub fn winnable_traced(party: &[Combatant], foes: &[Combatant], remarshal: bool) -> (bool, usize) {
    // Re-marshalling branches every formation at the top of each round anyway, so the opening ranks are
    // whatever: round 0 will overwrite them. A fixed search must enumerate them itself.
    if remarshal {
        let mut units: Vec<Combatant> = party.to_vec();
        units.extend(foes.iter().cloned());
        let mut memo = HashMap::new();
        let win = forces_win_with(&units, 0, 0, &mut memo, true);
        return (win, memo.len());
    }
    let n = party.len();
    let mut states = 0;
    for f in 0..3usize.pow(n as u32) {
        let mut units: Vec<Combatant> = party
            .iter()
            .enumerate()
            .map(|(k, p)| {
                let mut u = p.clone();
                u.rank = RANKS[(f / 3usize.pow(k as u32)) % 3];
                u
            })
            .collect();
        units.extend(foes.iter().cloned());
        let mut memo = HashMap::new();
        let win = forces_win(&units, 0, 0, &mut memo);
        states += memo.len();
        if win {
            return (true, states);
        }
    }
    (false, states)
}

/// Every way to rank the party's **living** heroes (the fallen have no formation). One entry per hero index.
fn formations(units: &[Combatant]) -> Vec<Vec<(usize, Rank)>> {
    let living: Vec<usize> = (0..units.len())
        .filter(|&i| units[i].side == Side::Party && !units[i].fallen)
        .collect();
    let mut out = Vec::new();
    for f in 0..3usize.pow(living.len() as u32) {
        out.push(
            living
                .iter()
                .enumerate()
                .map(|(k, &i)| (i, RANKS[(f / 3usize.pow(k as u32)) % 3]))
                .collect(),
        );
    }
    out
}

/// A memo key: the mutable state (per unit health/tempo/fallen/**pile**) plus the walk position.
///
/// The pile has to be in here. It used to be safe to omit it only because the pile was wiped at every
/// sub-phase boundary - i.e. it was *always zero* at exactly the points we memoize. Now that wounds carry
/// across a round's sub-phases, two positions with identical health and tempo but different accumulated
/// damage are genuinely different positions, and conflating them would make the solver return confidently
/// wrong answers rather than fail. It costs state space (a wound counter in `[0, grit)` per unit), which
/// is the price of the rule.
/// The **rank is in the key too**, and it has to be: once the party may re-Marshal, the formation is part of
/// the mutable state, not a constant of the battle. Leaving it out would conflate two genuinely different
/// positions and hand back a confidently wrong answer. (For a fixed-formation search it is a constant, so it
/// costs nothing but a few bytes.)
type Key = (Vec<(u32, u32, bool, u32, u8)>, usize, usize);

fn key_of(units: &[Combatant], round: usize, sub: usize) -> Key {
    let rank = |r: Rank| match r {
        Rank::Vanguard => 0u8,
        Rank::Outrider => 1,
        Rank::Rearguard => 2,
    };
    (
        units
            .iter()
            .map(|u| (u.health, u.tempo, u.fallen, u.pending, rank(u.rank)))
            .collect(),
        round,
        sub,
    )
}

fn party_won(units: &[Combatant]) -> Option<bool> {
    let party = units.iter().any(|u| u.side == Side::Party && !u.fallen);
    let foes = units.iter().any(|u| u.side == Side::Foe && !u.fallen);
    match (party, foes) {
        (true, true) => None,
        (won, _) => Some(won),
    }
}

/// Can the party force a win from the start of sub-phase `sub` in `round`?
fn forces_win(
    units: &[Combatant],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
) -> bool {
    forces_win_with(units, round, sub, memo, false)
}

fn forces_win_with(
    units: &[Combatant],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
    remarshal: bool,
) -> bool {
    if let Some(done) = party_won(units) {
        return done;
    }
    if round >= MAX_ROUNDS {
        return false; // hit the round cap undecided = a draw, not a win
    }
    let key = key_of(units, round, sub);
    if let Some(&r) = memo.get(&key) {
        return r;
    }
    let mut units = units.to_vec();
    if sub == 0 {
        combat::refresh_round(&mut units); // Tempo refreshes to Cadence each round
    }
    // **Marshal.** With re-marshalling the party re-declares its formation at the top of every round - which is
    // what the game actually lets you do. Without it, the ranks it walked in with are the ranks it fights with.
    let win = if remarshal && sub == 0 {
        formations(&units).into_iter().any(|f| {
            let mut u = units.clone();
            for (i, r) in f {
                u[i].rank = r;
            }
            search_engage_with(&u, round, sub, memo, remarshal)
        })
    } else {
        search_engage_with(&units, round, sub, memo, remarshal)
    };
    memo.insert(key, win);
    win
}

/// The **next** walk position after resolving sub-phase `sub`.
fn next(round: usize, sub: usize) -> (usize, usize) {
    if sub + 1 < SCHEDULE.len() {
        (round, sub + 1)
    } else {
        (round + 1, 0)
    }
}

/// Engage step: try every party engagement plan (joint over attackers), fold in the greedy foe, resolve,
/// recurse into Evade.
fn search_engage_with(
    units: &[Combatant],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
    rm: bool,
) -> bool {
    let foe_engagements = Greedy.engagements(units, Side::Foe, sub);
    let options = party_engage_options(units, sub);
    any_combo(&options, &mut |chosen| {
        let mut u = units.to_vec();
        let mut all: Vec<Engage> = chosen.iter().flatten().copied().collect();
        all.extend(foe_engagements.iter().copied());
        let reaching = combat::resolve_engage(&mut u, &all);
        search_evade(&u, &reaching, round, sub, memo, rm)
    })
}

/// Evade step: try every party dodge plan (Slip or Stand, per reached party unit), fold in the greedy foe,
/// resolve, recurse into Strike.
///
/// This is where the "no partial slip" rule pays for itself in the search: the branch is **binary** per unit,
/// not `0..tempo` wide. The dominated option was not merely bad play, it was a whole dimension of the tree.
fn search_evade(
    units: &[Combatant],
    reaching: &[Contact],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
    rm: bool,
) -> bool {
    // Party units that something is reaching for, and that can actually afford to escape it.
    let reached: Vec<usize> = (0..units.len())
        .filter(|&i| {
            units[i].side == Side::Party
                && !units[i].fallen
                && combat::slip_cost(units, reaching, i).is_some_and(|c| c <= units[i].tempo)
        })
        .collect();
    let options: Vec<Vec<Dodge>> = reached
        .iter()
        .map(|_| vec![Dodge::Stand, Dodge::Slip])
        .collect();
    any_combo(&options, &mut |chosen| {
        let dodges: Vec<Dodge> = (0..units.len())
            .map(|i| match reached.iter().position(|&r| r == i) {
                Some(pos) => chosen[pos],
                None if units[i].side == Side::Foe => Greedy.dodge(units, reaching, i),
                None => Dodge::Stand, // cannot afford to slip: standing is the only thing on offer
            })
            .collect();
        let mut u = units.to_vec();
        let contacts = combat::resolve_evade(&mut u, reaching, &dodges);
        search_strike(&u, &contacts, round, sub, memo, rm)
    })
}

/// Strike step: try every party blow plan (how many cards each contacted party unit pours in), fold in the
/// greedy foe, resolve, close the sub-phase, recurse to the next.
fn search_strike(
    units: &[Combatant],
    contacts: &[Contact],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
    rm: bool,
) -> bool {
    let foe_blows = Greedy.blows(units, Side::Foe, contacts);
    // Every party unit on an edge it may swing along - as the engager, or answering along a melee edge.
    let party_edges: Vec<(usize, usize)> = (0..units.len())
        .filter(|&i| units[i].side == Side::Party && !units[i].fallen && units[i].tempo > 0)
        .filter_map(|i| combat::strike_target(units, contacts, i).map(|t| (i, t)))
        .collect();
    let options: Vec<Vec<Blows>> = party_edges
        .iter()
        .map(|&(i, target)| {
            (0..=units[i].tempo)
                .map(|cards| Blows {
                    unit: i,
                    target,
                    cards,
                })
                .collect()
        })
        .collect();
    let (nr, ns) = next(round, sub);
    any_combo(&options, &mut |chosen| {
        let mut u = units.to_vec();
        let mut blows: Vec<Blows> = chosen.iter().filter(|b| b.cards > 0).copied().collect();
        blows.extend(foe_blows.iter().copied());
        combat::resolve_strike(&mut u, contacts, &blows);
        combat::end_sub_phase(&mut u);
        forces_win_with(&u, nr, ns, memo, rm)
    })
}

// ---- the party's pruned option sets -------------------------------------------------------------------

/// Each party attacker's engagement options this sub-phase: `None` (Hold), plus, for each legal and reachable
/// foe, **every commitment from one card up to the pin** — the pin being the fewest cards the target cannot
/// afford to slip.
///
/// **The search must offer exactly what the interface offers, and it used to offer less.** It gave only `{1,
/// pin}`, on the claim that everything between was dominated. That claim was wrong: an intermediate commitment
/// does not deny the escape, but it *taxes* it — the target pays more tempo to slip, and that tempo is blows it
/// will not throw. Fewer blows for you, a heavier toll on them: a genuine trade, and one the player can make.
///
/// Pruning it made the solver search a strictly smaller move set than the game allows, so a "lose" verdict was
/// never exhaustive — it could miss a winning line that needed an intermediate commit. For a balance harness
/// that is a false negative. For a **doomed** indicator it is the one thing it may never do: tell you to give
/// up a fight you could still win.
///
/// Above the pin *is* dominated (same contact, one fewer blow), which is why the UI bars it and this stops
/// there. Search space and offered moves now coincide exactly.
fn party_engage_options(units: &[Combatant], sub: usize) -> Vec<Vec<Option<Engage>>> {
    units
        .iter()
        .enumerate()
        .filter(|(_, u)| {
            u.side == Side::Party
                && !u.fallen
                && u.tempo > 0
                && combat::effective_in_rank(u.rank, u.melee, u.ranged)
        })
        .map(|(i, u)| {
            let mut opts = vec![None];
            for (j, v) in units.iter().enumerate() {
                if v.fallen
                    || v.side == Side::Party
                    || !combat::legal_strike(sub, u.rank, v.rank)
                    || !combat::back_access_ok(units, u.rank, j)
                {
                    continue;
                }
                if u.aoe {
                    // An area strike cannot be slipped - no commitment to tune, one card, no follow-up.
                    opts.push(Some(Engage {
                        attacker: i,
                        target: j,
                        cards: 1,
                    }));
                    continue;
                }
                // The pin: the fewest cards whose value they cannot out-spend at their Finesse. Past it, more
                // commitment buys nothing (the UI bars it); up to it, every card is a real choice.
                let pin = (1..=u.tempo)
                    .find(|&c| (c * u.finesse.max(1)) / v.finesse.max(1) + 1 > v.tempo);
                for cards in 1..=pin.unwrap_or(u.tempo) {
                    opts.push(Some(Engage {
                        attacker: i,
                        target: j,
                        cards,
                    }));
                }
            }
            dedup(opts)
        })
        .collect()
}

fn dedup(mut v: Vec<Option<Engage>>) -> Vec<Option<Engage>> {
    v.sort_by_key(|o| o.map(|c| (c.target, c.cards)));
    v.dedup();
    v
}

/// Try every combination of one option per slot; return `true` as soon as `f` accepts one (short-circuit).
fn any_combo<T: Clone>(options: &[Vec<T>], f: &mut dyn FnMut(&[T]) -> bool) -> bool {
    fn go<T: Clone>(
        options: &[Vec<T>],
        i: usize,
        acc: &mut Vec<T>,
        f: &mut dyn FnMut(&[T]) -> bool,
    ) -> bool {
        if i == options.len() {
            return f(acc);
        }
        for opt in &options[i] {
            acc.push(opt.clone());
            if go(options, i + 1, acc, f) {
                return true;
            }
            acc.pop();
        }
        false
    }
    go(options, 0, &mut Vec::new(), f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::play_battle;

    fn unit(
        name: &str,
        side: Side,
        might: u32,
        finesse: u32,
        cadence: u32,
        grit: u32,
        vitality: u32,
        melee: bool,
        ranged: bool,
    ) -> Combatant {
        Combatant {
            name: name.into(),
            side,
            rank: Rank::Vanguard, // overwritten per formation for party; scripted for foes
            might,
            finesse,
            cadence,
            grit,
            armor: 0,
            melee,
            ranged,
            aoe: false,
            horde: false,
            tempo: cadence,
            health: vitality,
            pending: 0,
            fallen: false,
        }
    }

    /// A clearly-dominant hero (out-hits, out-lasts) can force a win; a clearly-outmatched one cannot.
    #[test]
    fn dominant_hero_is_winnable_weak_one_is_not() {
        let strong = unit("Strong", Side::Party, 4, 2, 2, 1, 5, true, false);
        let weak = unit("Weak", Side::Party, 1, 1, 1, 1, 2, true, false);
        let foe = unit("Foe", Side::Foe, 2, 1, 1, 1, 3, true, false);

        assert!(winnable(&[strong], &[foe.clone()]));
        assert!(!winnable(&[weak], &[foe]));
    }

    /// Soundness: if the *greedy* party already wins the fight, the solver (which searches at least as well)
    /// must report winnable. Checked across a spread of solo matchups.
    #[test]
    fn winnable_dominates_greedy_play() {
        for might in 1..=4 {
            for vit in 2..=4 {
                let hero = unit("Hero", Side::Party, might, 2, 2, 1, vit, true, false);
                let mut foe = unit("Foe", Side::Foe, 2, 1, 1, 1, 3, true, false);
                foe.rank = Rank::Vanguard;
                let mut party = hero.clone();
                party.rank = Rank::Vanguard;
                if play_battle(vec![party, foe.clone()], &Greedy, &Greedy) == Some(true) {
                    assert!(
                        winnable(&[hero], &[foe]),
                        "greedy won but solver said not winnable (might {might}, vit {vit})"
                    );
                }
            }
        }
    }

    /// A ranged hero has a winnable formation (Rearguard, where it is effective) against a melee foe it
    /// out-values; the solver must find it even though the default Vanguard placement is dead weight.
    #[test]
    fn solver_finds_the_effective_formation_for_a_ranged_hero() {
        let archer = unit("Archer", Side::Party, 3, 2, 2, 1, 5, false, true);
        let mut foe = unit("Brute", Side::Foe, 1, 1, 1, 1, 3, true, false);
        foe.rank = Rank::Vanguard;
        assert!(winnable(&[archer], &[foe]));
    }
}
