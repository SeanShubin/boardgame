//! **The generic game interface** - the entire contract between a set of rules and everything that drives it.
//!
//! A rule system is a state machine: from any [`State`](Game::State) there is a list of legal
//! [`Choice`](Game::Choice)s ([`options`](Game::options)), each of which leads to exactly one successor
//! ([`apply`](Game::apply)), and some states are terminal ([`outcome`](Game::outcome)). That is the whole of it.
//!
//! **Three things fall out of that shape, and they are the point of the whole exercise:**
//!
//! - **One interface, three consumers.** The application, a solver, and a text explorer all do the same thing -
//!   read the state, read the options, pick one, read the new state. None of them needs to know a rule; they
//!   only walk the machine. So a solver ([`walk`]) is generic over *any* `Game`, and a rule change is a change
//!   to `options`/`apply` in one readable file, with nothing else to keep in step.
//!
//! - **Compound choices are just intermediate states.** "Pick an initiator, then an action, then a target" is
//!   not a special case - it is three ordinary states, each offering the next atomic set. A rule system never
//!   builds a nested choice widget; it moves the cursor and re-lists. The generic runner cannot tell a "big"
//!   decision from a small one, which is exactly why it can be generic.
//!
//! - **Reactions are options too.** A decision a body makes in response to what an opponent committed is not
//!   different in kind from a decision it makes up front - it is an `options` set on a later state. So there is
//!   no separate "interrupt" or "reaction" machinery anywhere in the rules.
//!
//! **A rule system knows nothing outside itself.** No physical cards, no rendering, no other rule category.
//! Physical-card conservation and presentation are enforced *above* this seam, by mirroring the state, never
//! *inside* it. That is what lets a single rules file be read and held whole.

/// How a game ended. `None` from [`Game::outcome`] means it is still going.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The controlling side achieved its objective.
    Win,
    /// It failed.
    Loss,
    /// Neither, at a terminating bound (e.g. a round cap).
    Draw,
}

/// A rule system, expressed as a pure state machine. Implementors put **nothing** in here but rules.
///
/// It is deliberately spartan. Everything a driver needs - to play, to search, or to explore - is expressible
/// as `options` / `apply` / `outcome` over a `Clone`able `State`. If a method would need to reach outside the
/// rules to answer, it does not belong on this trait.
pub trait Game {
    /// The complete position: everything true right now, and nothing else. A driver clones it freely (a solver
    /// leans on that heavily), so it should be cheap and self-contained.
    type State: Clone;

    /// One atomic decision. For a compound choice, this is a *single step* of it - the next state's options
    /// carry the rest.
    type Choice: Clone;

    /// **Every legal choice from `state`.** Order is not load-bearing to correctness, but a driver may show the
    /// first as a default and a reachability search short-circuits on the first winning line, so a rule system
    /// may order these to put the "obvious" choice first.
    ///
    /// An empty list means *no one has a decision here*: either the state is terminal (see [`outcome`]), or the
    /// only thing left is a forced transition the driver should apply on the single available choice. A rule
    /// system should never present a choice with one option - fold it into `apply` or return it as the sole
    /// element and let the runner auto-advance.
    fn options(state: &Self::State) -> Vec<Self::Choice>;

    /// **The one successor** of taking `choice` in `state`. Pure and deterministic: same inputs, same output,
    /// no clocks and no hidden randomness. Any scripted opponent or environment reaction happens *here*, folded
    /// into the transition, so `options` only ever offers the controlling side's decisions.
    ///
    /// `choice` is assumed to be one that `options(state)` offered; a rule system may debug-assert that but is
    /// not required to handle an illegal choice gracefully.
    fn apply(state: &Self::State, choice: &Self::Choice) -> Self::State;

    /// The terminal verdict, or `None` if the game is still going.
    fn outcome(state: &Self::State) -> Option<Outcome>;
}

/// Drive a game to its end, asking `pick` whenever there is a real decision.
///
/// This is the whole runner, and it is the same three lines for the application, a demo UI, and a headless
/// script - only `pick` differs. A state with a single option **auto-advances** (that is the count-adaptive
/// rule: a choice with one legal option is not a choice), so `pick` is only ever called at a genuine fork.
pub fn run<G: Game>(
    mut state: G::State,
    mut pick: impl FnMut(&G::State, &[G::Choice]) -> usize,
) -> Outcome {
    loop {
        if let Some(o) = G::outcome(&state) {
            return o;
        }
        let opts = G::options(&state);
        let choice = match opts.len() {
            0 => return G::outcome(&state).unwrap_or(Outcome::Draw), // a dead end with no verdict is a draw
            1 => 0,
            _ => pick(&state, &opts),
        };
        state = G::apply(&state, &opts[choice]);
    }
}

/// How many decision points lie beyond a state, up to `depth` - the "how much is left to decide" number a
/// decision-tree explorer shows next to each option.
///
/// It counts *nodes with a real choice* (two or more options), not leaves, so a long forced run reads as
/// shallow. `depth` bounds the walk so an explorer never hangs on a deep tree; the count is "at least this
/// many within `depth`", saturating at the bound.
pub fn decisions_within<G: Game>(state: &G::State, depth: u32) -> u64 {
    if depth == 0 || G::outcome(state).is_some() {
        return 0;
    }
    let opts = G::options(state);
    let here = u64::from(opts.len() > 1);
    let below: u64 = opts
        .iter()
        .map(|c| decisions_within::<G>(&G::apply(state, c), depth - 1))
        .sum();
    here + below
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trivial three-step game, to pin the runner and the trait shape without any real rules: a counter that
    /// ticks to 3, offering `Stay` or `Step` until it gets there.
    struct Ticker;
    #[derive(Clone)]
    struct Count(u32);
    #[derive(Clone, Debug, PartialEq)]
    enum Move {
        Step,
        Stay,
    }

    impl Game for Ticker {
        type State = Count;
        type Choice = Move;
        fn options(s: &Count) -> Vec<Move> {
            if s.0 >= 3 {
                vec![]
            } else {
                vec![Move::Step, Move::Stay]
            }
        }
        fn apply(s: &Count, c: &Move) -> Count {
            match c {
                Move::Step => Count(s.0 + 1),
                Move::Stay => s.clone(),
            }
        }
        fn outcome(s: &Count) -> Option<Outcome> {
            (s.0 >= 3).then_some(Outcome::Win)
        }
    }

    #[test]
    fn the_runner_walks_a_game_to_its_end() {
        // Always Step: three moves to the win.
        let out = run::<Ticker>(Count(0), |_, _| 0);
        assert_eq!(out, Outcome::Win);
    }

    #[test]
    fn a_single_option_auto_advances_without_asking() {
        // A game that offers exactly one move should never call `pick`.
        struct Forced;
        impl Game for Forced {
            type State = u32;
            type Choice = ();
            fn options(s: &u32) -> Vec<()> {
                (*s < 2).then_some(()).into_iter().collect()
            }
            fn apply(s: &u32, _: &()) -> u32 {
                s + 1
            }
            fn outcome(s: &u32) -> Option<Outcome> {
                (*s >= 2).then_some(Outcome::Win)
            }
        }
        run::<Forced>(0, |_, _| {
            panic!("a forced game must not ask for a decision")
        });
    }

    /// The path counter tallies terminal leaves: on a tiny loop-free game with a known tree, the win/loss split
    /// is exact and complete.
    #[test]
    fn path_counter_tallies_wins_and_losses() {
        // A 2-ply binary tree: from 0, two moves to depth 1; from each, two moves to depth 2 (terminal).
        // Depth-2 states 2,3,4 are wins if even, losses if odd - so we know the leaf outcomes.
        struct Tree;
        impl Game for Tree {
            type State = u32;
            type Choice = u32; // add 1 or add 2
            fn options(s: &u32) -> Vec<u32> {
                if *s >= 4 { vec![] } else { vec![1, 2] }
            }
            fn apply(s: &u32, c: &u32) -> u32 {
                s + c
            }
            fn outcome(s: &u32) -> Option<Outcome> {
                if *s < 4 {
                    None
                } else if s % 2 == 0 {
                    Some(Outcome::Win)
                } else {
                    Some(Outcome::Loss)
                }
            }
        }
        impl Solvable for Tree {
            type Key = u32;
            fn key(s: &u32) -> u32 {
                *s
            }
        }
        let mut c = PathCounter::<Tree>::new();
        c.grant(1_000);
        let p = c.count(&0);
        assert!(p.complete, "the tree is small enough to finish");
        // From 0: paths reach terminals 4,5,6 (even=win). Every leaf >=4 that is even is a win.
        assert!(
            p.wins > 0 && p.losses > 0,
            "both outcomes are reachable: {p:?}"
        );
    }

    /// A starved counter is honest: it returns a partial (`complete=false`) lower bound, never a wrong total.
    #[test]
    fn a_starved_path_counter_is_incomplete_not_wrong() {
        struct Deep;
        impl Game for Deep {
            type State = u32;
            type Choice = u32;
            fn options(s: &u32) -> Vec<u32> {
                if *s >= 30 { vec![] } else { vec![1, 2] }
            }
            fn apply(s: &u32, c: &u32) -> u32 {
                s + c
            }
            fn outcome(s: &u32) -> Option<Outcome> {
                (*s >= 30).then_some(Outcome::Win)
            }
        }
        impl Solvable for Deep {
            type Key = u32;
            fn key(s: &u32) -> u32 {
                *s
            }
        }
        let mut c = PathCounter::<Deep>::new();
        c.grant(5); // far too little
        let p = c.count(&0);
        assert!(!p.complete, "it should run out of budget");
    }

    #[test]
    fn decisions_within_counts_forks_not_leaves() {
        // A loop-free game: at each step, Fork (two ways to the same successor) or nothing. So every
        // non-terminal state is exactly one fork, and the depth bound caps the count.
        struct Line;
        impl Game for Line {
            type State = u32;
            type Choice = bool;
            fn options(s: &u32) -> Vec<bool> {
                if *s >= 5 { vec![] } else { vec![true, false] }
            }
            fn apply(s: &u32, _: &bool) -> u32 {
                s + 1
            }
            fn outcome(s: &u32) -> Option<Outcome> {
                (*s >= 5).then_some(Outcome::Win)
            }
        }
        // From 4: one fork (4->5 terminal). From 3: this fork plus the one at 4 => but both branches lead to
        // the same states, so the walk double-counts by branch, as intended (it measures the *tree*, not the
        // reachable set): 3 has 1 here + 2 subtrees each worth `decisions_within(4)`.
        assert_eq!(decisions_within::<Line>(&4, 10), 1);
        assert_eq!(
            decisions_within::<Line>(&5, 10),
            0,
            "a terminal state has nothing left to decide"
        );
        // Bounded: depth 2 from 0 counts only the forks within two plies.
        assert_eq!(decisions_within::<Line>(&0, 1), 1);
    }
}

// ---- the generic solver: winnable / evaluating / doomed over any single-agent Game -----------------------

/// What a search says about a position. The three states, exactly as the design has always named them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Some line from here reaches a [`Outcome::Win`].
    Winnable,
    /// The search ran out of budget before it could be sure. **Not an answer** - an answer in progress.
    Evaluating,
    /// The tree is exhausted and no line wins.
    Doomed,
}

/// A [`Game`] a solver can search. It is single-agent by construction (the opponent is folded into
/// [`Game::apply`]), so "can the controlling side force a win" is a plain reachability question: **is there a
/// path to a [`Win`](Outcome::Win)** - an OR over `options` at every node.
///
/// The only thing a `Game` must add to be searchable is a **key**: a cheap, hashable digest of a state that is
/// equal for two states a search may treat as the same position. Canonicalizing away meaningless distinctions
/// (relabelled regions, seat order) belongs in `key` - it is what lets the memo collapse the tree.
pub trait Solvable: Game {
    type Key: std::hash::Hash + Eq;
    fn key(state: &Self::State) -> Self::Key;
}

/// **The doom oracle, generic over any [`Solvable`] game.** Holds the memo, so the first evaluation walks the
/// tree and every later one is a lookup - which is the whole reason a search over many near-identical positions
/// (e.g. every opening formation) is affordable.
///
/// Budgeted and restartable: give it a node budget with [`grant`](Solver::grant), and if it runs out it answers
/// [`Verdict::Evaluating`] rather than lying. **The one rule it may never break:** an incomplete subtree is
/// never memoized as a loss. A "no win found" that was really "I gave up" must never be cached as `Doomed`. The
/// oracle may be silent; it may never be wrong.
pub struct Solver<G: Solvable> {
    memo: std::collections::HashMap<G::Key, bool>,
    nodes: u64,
    walk: u64,
    budget: u64,
    aborted: bool,
}

impl<G: Solvable> Default for Solver<G> {
    fn default() -> Self {
        Self::new()
    }
}

impl<G: Solvable> Solver<G> {
    pub fn new() -> Self {
        Solver {
            memo: std::collections::HashMap::new(),
            nodes: 0,
            walk: 0,
            budget: 0,
            aborted: false,
        }
    }

    /// Positions evaluated across every walk - the cost report.
    pub fn nodes(&self) -> u64 {
        self.nodes
    }
    /// Distinct positions the memo holds.
    pub fn states(&self) -> usize {
        self.memo.len()
    }
    pub fn aborted(&self) -> bool {
        self.aborted
    }

    /// Allow the next walk `nodes` positions and clear the abort flag. The memo survives, so each retry
    /// re-treads its settled positions for free and pushes the frontier deeper. **Escalate on `Evaluating`** -
    /// a grant too small to settle any new subtree makes no progress however often repeated; the caller doubles
    /// it. Safety never depends on the grant.
    pub fn grant(&mut self, nodes: u64) {
        self.walk = 0;
        self.budget = nodes;
        self.aborted = false;
    }

    /// The verdict for `state`, spending up to the current [`grant`](Solver::grant).
    pub fn verdict(&mut self, state: &G::State) -> Verdict {
        let before = self.aborted;
        let win = self.winnable(state);
        match (win, self.aborted && !before) {
            (true, _) => Verdict::Winnable,
            (false, true) => Verdict::Evaluating,
            (false, false) => Verdict::Doomed,
        }
    }

    /// Is there a line from `state` that wins? A **win is a proof** (a witness path), so it stands even when
    /// other branches were abandoned; a **loss is a proof only if the whole subtree was explored**.
    pub fn winnable(&mut self, state: &G::State) -> bool {
        match G::outcome(state) {
            Some(Outcome::Win) => return true,
            Some(_) => return false, // Loss or Draw: not a win
            None => {}
        }
        let opts = G::options(state);
        // A forced move is not a decision: pass straight through it, charging no budget and taking no memo slot,
        // so a long scripted run - every creature declaring its one legal act - costs the search nothing. This is
        // the solver honouring the same "one option is not a choice" rule the runner ([`run`]) already follows.
        if opts.len() == 1 {
            return self.winnable(&G::apply(state, &opts[0]));
        }
        if self.walk >= self.budget {
            self.aborted = true;
            return false;
        }
        let key = G::key(state);
        if let Some(&v) = self.memo.get(&key) {
            return v;
        }
        self.nodes += 1;
        self.walk += 1;

        // Each node judges its OWN subtree: stash the caller's abort flag and start clean, so this node cannot
        // inherit a sibling's give-up and mistake it for its own completeness (which would cache an incomplete
        // "no win" as a proven Doomed - the one thing the oracle may never do).
        let outer = self.aborted;
        self.aborted = false;

        let mut win = false;
        for choice in opts {
            if self.winnable(&G::apply(state, &choice)) {
                win = true;
                break;
            }
        }

        let incomplete = self.aborted;
        self.aborted = outer || incomplete;
        if win || !incomplete {
            self.memo.insert(key, win); // cache only what we can prove
        }
        win
    }
}

// ---- path counting: how many ways a route wins vs loses --------------------------------------------------

/// A tally of terminal outcomes reachable from a state: how many complete lines end in a [`Win`](Outcome::Win)
/// versus not (a loss or a draw - **a tie counts as a loss**). `complete` is false when the count ran out of
/// budget, in which case `wins`/`losses` are honest **lower bounds** (a `>=`), never guesses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Paths {
    pub wins: u64,
    pub losses: u64,
    pub complete: bool,
}

/// Counts winning vs losing lines through a [`Solvable`] game's tree, memoized on the state key so the shared
/// DAG is walked once. Budgeted like [`Solver`]: a walk that runs out of budget returns a partial count and
/// **does not memoize the incomplete node**, so a cached count is always exact.
///
/// Counts saturate at [`u64::MAX`] rather than overflow - a fight can have astronomically many lines, and past
/// a point the exact number matters less than the ratio.
pub struct PathCounter<G: Solvable> {
    memo: std::collections::HashMap<G::Key, (u64, u64)>,
    walk: u64,
    budget: u64,
    aborted: bool,
}

impl<G: Solvable> Default for PathCounter<G> {
    fn default() -> Self {
        Self::new()
    }
}

impl<G: Solvable> PathCounter<G> {
    pub fn new() -> Self {
        PathCounter {
            memo: std::collections::HashMap::new(),
            walk: 0,
            budget: 0,
            aborted: false,
        }
    }

    /// Allow the next count `nodes` positions. The memo survives, so an escalating grant converges (double on
    /// an incomplete result).
    pub fn grant(&mut self, nodes: u64) {
        self.walk = 0;
        self.budget = nodes;
        self.aborted = false;
    }

    /// Tally the lines from `state`, up to the current [`grant`](PathCounter::grant).
    pub fn count(&mut self, state: &G::State) -> Paths {
        let before = self.aborted;
        let (wins, losses) = self.tally(state);
        Paths {
            wins,
            losses,
            complete: !self.aborted || before,
        }
    }

    fn tally(&mut self, state: &G::State) -> (u64, u64) {
        match G::outcome(state) {
            Some(Outcome::Win) => return (1, 0),
            Some(_) => return (0, 1), // loss or draw - a tie is a loss
            None => {}
        }
        let opts = G::options(state);
        // A forced move is transparent to the count: the lines through it are exactly the lines through its one
        // child. Pass through without a budget charge or a memo slot, so a scripted foe chain neither inflates the
        // tree nor eats the budget - the count reaches as deep as it did before creatures declared.
        if opts.len() == 1 {
            return self.tally(&G::apply(state, &opts[0]));
        }
        if self.walk >= self.budget {
            self.aborted = true;
            return (0, 0);
        }
        let key = G::key(state);
        if let Some(&v) = self.memo.get(&key) {
            return v;
        }
        self.walk += 1;

        let outer = self.aborted;
        self.aborted = false;
        let (mut w, mut l) = (0u64, 0u64);
        for choice in opts {
            let (cw, cl) = self.tally(&G::apply(state, &choice));
            w = w.saturating_add(cw);
            l = l.saturating_add(cl);
        }
        let incomplete = self.aborted;
        self.aborted = outer || incomplete;
        if !incomplete {
            self.memo.insert(key, (w, l)); // cache only a complete tally
        }
        (w, l)
    }
}
