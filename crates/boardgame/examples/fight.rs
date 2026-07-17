//! **Fight** - a clickable UI to play a combat by hand, driving the pure [`rules::combat`] game through the
//! generic interface. Pick an option; the board updates; repeat to a verdict.
//!
//! Every option button shows, for the route it opens, **how many complete lines end in a win and how many in a
//! loss** (a tie counts as a loss), plus the solver's verdict. That is the "how forgiving is this move" number:
//! a route with many winning lines and few losing ones is safe; one Doomed line is a trap the button says so. The
//! counts carry a completeness sign: `>=` while the tally is still a growing lower bound, flipping to `=` once
//! that whole subtree has been walked (a `Doomed` is always `=` - it can only be proven by exhaustion).
//!
//! Each option also shows its **best route** - `best Nd/Nr/Nhp` = the single best line through it under the stated
//! priority (win, then fewest heroes **d**owned, then fewest **r**ounds, then least **hp** flipped), `<=` while
//! that search is still a provisional bound. The header shows the best route from the position itself.
//!
//! **Crossings are two beats, in fiction order.** A Raid or Slip is not shown pre-answered; it is one card. Pick
//! it and - because the foes are deterministic, so who intercepts you is known - the UI names who caught you
//! (`The Wall catches you at the line`) and *then* asks how to answer: Evade, Push, or turn and fight the catcher.
//! An unopposed crossing skips the second beat and just crosses. Under the hood the solver still sees the atomic
//! `Raid(target, Answer)`; the two beats are purely how the choice is presented.
//!
//! Space-efficient by intent: a compact unit table with stat abbreviations (M/V/G/C/F = Might / Vitality /
//! Grit / Cadence / Finesse) and F/b/o ranks (front vanguard / back rearguard / loose outrider). There are two
//! lines - yours and theirs - so a crossing reads as "into their line", not a region letter.
//!
//! **Back** steps to the previous decision - a pointer move over the kept solver memo, so nothing is recomputed,
//! and the log unwinds with it (a first Back also just closes an open crossing). Two files mirror the session
//! live: `fight-screen.txt` (a snapshot of the current screen) and `fight-log.txt` (the *entire* running log).
//!
//! Run: `cargo run --release -p boardgame --example fight -- [encounter#] [kit]` - a **solo** encounter (0-3) is
//! fielded by exactly ONE kit (the keystone's counter by default; name another - Raider/Marksman/Bastion/
//! Bombardier - to override); a **party** encounter (4-7) musters the whole roster.

use bevy::prelude::*;

use deckbound_board::units::{encounter_beasts, kit};
use deckbound_content::catalog::{self, Encounter};
use rules::combat::game::{Choice, Combat, Decider, Human, Instinct, Score, Scorer, State};
use rules::combat::regions::{Act, Answer, Board, Rank, catchers, play_round};
use rules::combat::resolve::{Combatant, Side};
use rules::core::{Game, PathCounter, Paths, Solver, Verdict};

// ---- palette -------------------------------------------------------------------------------------------
const FELT: Color = Color::srgb(0.08, 0.09, 0.11);
const PANEL: Color = Color::srgb(0.14, 0.15, 0.18);
const SUNK: Color = Color::srgb(0.11, 0.12, 0.14);
const INK: Color = Color::srgb(0.93, 0.94, 0.96);
const MUTED: Color = Color::srgb(0.60, 0.64, 0.70);
const GOOD: Color = Color::srgb(0.45, 0.80, 0.65);
const WARN: Color = Color::srgb(0.90, 0.72, 0.35);
const BAD: Color = Color::srgb(0.90, 0.42, 0.44);
const FOE: Color = Color::srgb(0.95, 0.72, 0.72);

/// **The MOST node-work any one frame may do.** A frame is bounded to this, so it never stalls: the window stays
/// at framerate and a click is always handled next frame *no matter how hard the position* - the search just
/// spreads over more frames, it never freezes one. A **work** budget, not a wall clock, deliberately - it must
/// behave identically on WASM (where `std::time::Instant` panics) as on desktop, and these small boards make
/// node-count a good time proxy. Tune down for a smaller frame, up for faster convergence.
const FRAME_BUDGET: u64 = 12_000;
/// One item's node grant - a verdict push, or one refinement pass of an option's win/loss tally. Small enough
/// that several items get a turn inside [`FRAME_BUDGET`]; the shared memo carries each walk deeper across frames,
/// so verdicts settle and tallies fill in over a handful of frames, and a still-incomplete tally is an honest
/// ">=" lower bound in the meantime.
const STEP_NODES: u64 = 4_000;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let idx: usize = args.get(1).and_then(|a| a.parse().ok()).unwrap_or_else(|| {
        catalog::ENCOUNTERS
            .iter()
            .position(|e| e.party)
            .unwrap_or(0)
    });
    // A **solo** encounter is fielded by exactly one kit (see `build`); an optional second arg names which.
    let requested_kit = args.get(2).cloned();

    // Terminal hint: what is fielded, and (for a solo) how to change the kit.
    let e = &catalog::ENCOUNTERS[idx % catalog::ENCOUNTERS.len()];
    let kits: Vec<&str> = catalog::ROSTER.iter().map(|(n, _, _)| *n).collect();
    if e.party {
        println!(
            "{} - party encounter: full roster ({}).",
            e.location,
            kits.join(", ")
        );
    } else {
        println!(
            "{} - SOLO: one kit, [{}]. Change it with a second arg, one of: {}.",
            e.location,
            solo_kit(idx, requested_kit.as_deref()).0,
            kits.join(", ")
        );
    }

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "fight - regions combat".into(),
                resolution: (1180u32, 820u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(FELT))
        .insert_resource(Fight::new(idx, requested_kit))
        .add_systems(Startup, (camera, rebuild).chain())
        .add_systems(Update, (on_click, grind, rebuild.run_if(is_dirty)).chain())
        .run();
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

// ---- state ---------------------------------------------------------------------------------------------

/// **A menu entry as the player meets it**, one narrative beat. A crossing (Raid/Slip) is not shown as three
/// pre-answered options; it is one entry that, when picked, reveals who caught you and asks how to answer -
/// matching the order the fiction hands you the decision. Everything else is a direct choice.
#[derive(Clone)]
enum Entry {
    /// A plain choice - index into [`Fight::options`].
    Direct(usize),
    /// A crossing that fans out into an [`Answer`] once you see the interception. `dest` is the region crossed to
    /// (to find the catchers); `answers` pairs each answer with its underlying `options` index.
    Crossing {
        label: String,
        dest: u8,
        answers: Vec<(Answer, usize)>,
    },
}

/// **Beat two of a crossing:** the player declared a Raid/Slip, the line caught them, and now they answer it. Held
/// only while that second decision is open; cleared the moment an answer is chosen or the crossing is cancelled.
struct Drill {
    /// The crossing's own label, e.g. `Raid The Swarm`.
    label: String,
    /// Who caught the crosser, already joined for display (e.g. `The Wall x3`). Never empty - an unopposed
    /// crossing skips the drill entirely and just crosses.
    catchers: String,
    /// Each answer and the underlying `options` index that applies it.
    answers: Vec<(Answer, usize)>,
}

/// **One undo point** - the whole decision-point state, snapshotted so Back is a pointer move with **nothing
/// recomputed**: the solver/counter memos are kept across moves (their key is a pure function of the position, so
/// a restored position is already solved), and the scored options are captured here too, so undo restores
/// instantly without even a re-scan. `log_len` is the log height to unwind the history stack back to.
struct Step {
    state: State,
    log_len: usize,
    options: Vec<Choice>,
    entries: Vec<Entry>,
    opt_verdict: Vec<Option<Verdict>>,
    opt_paths: Vec<Option<Paths>>,
    opt_score: Vec<Option<Score>>,
    opt_score_done: Vec<bool>,
    verdict: Option<Verdict>,
}

#[derive(Resource)]
struct Fight {
    encounter: usize,
    /// The kit requested on the command line for a solo encounter (`None` = the keystone's counter). Kept so
    /// Restart / Next re-field the same solo kit.
    requested_kit: Option<String>,
    state: State,
    /// **Who decides each body**, indexed by body: a party body is a [`Human`] (the player at the UI), a foe an
    /// [`Instinct`]. The play loop ([`reposition`](Fight::reposition)) polls the deciding body's Decider and never
    /// learns which kind it asked - a scripted foe, a random one, or the human all commit the same way. Rebuilt
    /// with the roster. (The current policies are stateless, so Back need not rewind them; a stateful policy like
    /// `Random` would have to be snapshotted into [`Step`].) `Send + Sync` because the `Fight` is a Bevy resource.
    deciders: Vec<Box<dyn Decider + Send + Sync>>,
    /// The legal options right now. Their scores are computed lazily, a slice per frame - never on the click.
    options: Vec<Choice>,
    /// The player-facing grouping of `options` into narrative entries (crossings collapsed to one beat). Rebuilt
    /// with the options; scoring still runs over the flat `options`, this only changes how they are shown.
    entries: Vec<Entry>,
    /// Set while a crossing's second beat is open (the player picked Raid/Slip and must answer the line).
    drill: Option<Drill>,
    /// The undo stack: one [`Step`] per decision the player has taken, most recent last. Back pops it.
    history: Vec<Step>,
    /// Each option's verdict and win/loss tally; `None` means "still computing" (shown as `...`).
    opt_verdict: Vec<Option<Verdict>>,
    opt_paths: Vec<Option<Paths>>,
    /// Each option's **best route** (win > fewest downed > fewest rounds > least hp), and whether that search is
    /// exhausted (exact) rather than still a provisional `<=` bound.
    opt_score: Vec<Option<Score>>,
    opt_score_done: Vec<bool>,
    /// The position's own verdict; `None` while thinking.
    verdict: Option<Verdict>,
    /// A solver, counter, and best-route scorer kept ACROSS frames, so their memos survive and the work converges
    /// instead of restarting. Rebuilt only when the roster changes (the scorer's hp reference is the fight-start
    /// Vitality, fixed for the fight).
    solver: Solver<Combat>,
    counter: PathCounter<Combat>,
    scorer: Scorer,
    /// Round-robin cursors over the options, so each option's tally and each option's best-route search get a fair
    /// share of refinement each frame - no single hard one starves the others.
    refine: usize,
    refine_score: usize,
    /// Each unit's Vitality (its full health), snapshotted at the start so the table can show max **and** current
    /// health - the live `health` field only ever holds the current value, so the maximum has to be kept here.
    max_health: Vec<u32>,
    log: Vec<String>,
    dirty: bool,
}

impl Fight {
    fn new(encounter: usize, requested_kit: Option<String>) -> Self {
        let mut f = Fight {
            encounter,
            state: build(encounter, requested_kit.as_deref()),
            requested_kit,
            deciders: Vec::new(),
            options: Vec::new(),
            entries: Vec::new(),
            drill: None,
            history: Vec::new(),
            opt_verdict: Vec::new(),
            opt_paths: Vec::new(),
            opt_score: Vec::new(),
            opt_score_done: Vec::new(),
            verdict: None,
            solver: Solver::new(),
            counter: PathCounter::new(),
            scorer: Scorer::new(Vec::new(), 0),
            refine: 0,
            refine_score: 0,
            max_health: Vec::new(),
            log: Vec::new(),
            dirty: true,
        };
        f.snapshot_max();
        f.build_deciders();
        f.rebuild_scorer();
        f.log_roster(); // list ranks at the top, before any forced declarations are logged
        f.reposition();
        f.sync_log();
        f
    }

    /// Assign each body its [`Decider`]: the party plays by hand ([`Human`] - the loop defers to a click), the
    /// foes by [`Instinct`] (the deterministic scripted policy, the same one the solver plugs in as its
    /// environment, so its verdict stays true to the fight). Rebuilt whenever the roster changes.
    fn build_deciders(&mut self) {
        self.deciders = self
            .state
            .board()
            .units
            .iter()
            .map(|u| -> Box<dyn Decider + Send + Sync> {
                if u.side == Side::Party {
                    Box::new(Human)
                } else {
                    Box::new(Instinct)
                }
            })
            .collect();
    }

    /// (Re)build the best-route scorer with the fight-start Vitality as its hp reference - called once per roster
    /// (the reference is fixed for the whole fight, so the scorer memo stays valid across moves).
    fn rebuild_scorer(&mut self) {
        self.scorer = Scorer::new(self.max_health.clone(), 0);
    }

    /// Record every unit's full health for this encounter, so the table can show max vs current.
    fn snapshot_max(&mut self) {
        self.max_health = self.state.board().units.iter().map(|u| u.health).collect();
    }

    /// The opening roster: every combatant's starting **rank**, plus its (static) reach and shape. Ranks are
    /// listed once here and thereafter only NARRATED when they change (a crossing makes an Outrider, a dissolution
    /// sends one home). Reach and shape never change, so listing them once grounds the strike verbs (`fires`,
    /// `sweeps`, ...) without repeating on every line.
    fn log_roster(&mut self) {
        let loc = self.enc().location; // &'static, so it holds no borrow of self
        let rows: Vec<String> = {
            let b = self.state.board();
            let mut rows = vec![
                format!("=== {loc} ==="),
                "Combatants (rank / reach / shape):".to_string(),
            ];
            for i in 0..b.units.len() {
                let u = &b.units[i];
                let mark = if u.side == Side::Party { " " } else { "*" };
                let rank = match b.ranks[i] {
                    Rank::Vanguard => "vanguard",
                    Rank::Rearguard => "rearguard",
                    Rank::Outrider => "outrider",
                };
                let reach = if u.ranged && !u.melee {
                    "ranged"
                } else {
                    "melee"
                };
                let shape = if u.aoe { "area" } else { "single" };
                let horde = if u.horde {
                    format!(" (horde x{})", u.health)
                } else {
                    String::new()
                };
                rows.push(format!(
                    "  {mark}{:<12} {rank:<10} {reach:<6} {shape}{horde}",
                    u.name
                ));
            }
            rows
        };
        self.log.extend(rows);
    }

    fn enc(&self) -> &'static Encounter {
        &catalog::ENCOUNTERS[self.encounter % catalog::ENCOUNTERS.len()]
    }

    fn reset(&mut self) {
        self.state = build(self.encounter, self.requested_kit.as_deref());
        self.snapshot_max();
        self.build_deciders();
        // A new roster (Restart or Next encounter) is the ONE thing that invalidates the memos: the state key
        // omits unit stats, so a key from one encounter must never answer for another. Start fresh here - the one
        // place the units change. Ordinary moves keep the memo (see `reposition`).
        self.solver = Solver::new();
        self.counter = PathCounter::new();
        self.rebuild_scorer();
        self.log.clear();
        self.history.clear(); // a new roster invalidates every prior decision point
        self.log_roster();
        self.reposition();
        self.sync_log();
    }

    /// **The position changed** - list the new options and arm the lazy compute. Nothing is searched here, so a
    /// click returns instantly and the board is drawn at once; the scores fill in over the next frames.
    fn reposition(&mut self) {
        // Poll the deciding body's Decider until it defers. An AUTONOMOUS policy (a foe's [`Instinct`]) commits its
        // act here and now - this is where creature declarations get applied and, crucially, LOGGED, through the
        // same `apply_choice` path a click takes. A [`Human`] defers (`commit` -> None): the loop stops and shows
        // its real options, unless there is only one (a forced move), which it auto-advances so the UI only ever
        // presents a genuine decision. The loop never learns which kind of Decider it asked.
        loop {
            if Combat::outcome(&self.state).is_some() {
                self.options = Combat::options(&self.state);
                break;
            }
            let Some(i) = self.state.deciding() else {
                self.options = Combat::options(&self.state);
                break;
            };
            match self.deciders[i].commit(self.state.board(), i) {
                Some(act) => self.apply_choice(&Choice::Act(act)),
                None => {
                    let opts = Combat::options(&self.state);
                    if opts.len() == 1 {
                        let c = opts[0].clone();
                        self.apply_choice(&c);
                    } else {
                        self.options = opts;
                        break;
                    }
                }
            }
        }
        // Group the flat options into narrative entries, and drop any half-open crossing decision: a new position
        // is a fresh set of beats.
        self.entries = build_entries(self.state.board(), &self.options);
        self.drill = None;
        self.opt_verdict = vec![None; self.options.len()];
        self.opt_paths = vec![None; self.options.len()];
        self.opt_score = vec![None; self.options.len()];
        self.opt_score_done = vec![false; self.options.len()];
        self.verdict = None;
        // KEEP the solver/counter memos across a move - do NOT re-new them here. The game is deterministic and the
        // memo key is a pure function of the position, so any subtree already explored while scoring the parent's
        // options is reused now: the child we stepped into was walked to produce the win/loss tally we just showed
        // on its button, so its verdict and counts are already in the memo and land instantly. Only a *roster*
        // change invalidates them, and that is handled in `reset`. So stepping through a fight re-computes only the
        // genuinely new frontier, never a node already seen.
        self.refine = 0;
        self.refine_score = 0;
        self.dirty = true;
    }

    /// **One frame's worth of scoring, bounded to [`FRAME_BUDGET`] nodes** so the frame never stalls. Priorities,
    /// each item granted [`STEP_NODES`] at a time and retried across frames until it settles: the position's own
    /// verdict, then each option's verdict, then the option tallies refined round-robin. No grant ever escalates,
    /// so worst-case frame time is bounded no matter how hard the position - a brutal search just takes more
    /// frames. Returns whether anything changed (redraw if so).
    fn grind(&mut self) -> bool {
        if Combat::outcome(&self.state).is_some() {
            return false;
        }
        let mut spent = 0u64;
        let mut changed = false;

        // 1. the position's own verdict - push it a step; the memo carries the walk deeper next frame.
        if spent < FRAME_BUDGET && self.verdict.is_none() {
            self.solver.grant(STEP_NODES);
            if let v @ (Verdict::Winnable | Verdict::Doomed) = self.solver.verdict(&self.state) {
                self.verdict = Some(v);
            }
            spent += STEP_NODES;
            changed = true;
        }

        // 2. each option's verdict, in turn.
        for i in 0..self.options.len() {
            if spent >= FRAME_BUDGET {
                break;
            }
            if self.opt_verdict[i].is_none() {
                let next = Combat::apply(&self.state, &self.options[i]);
                self.solver.grant(STEP_NODES);
                if let v @ (Verdict::Winnable | Verdict::Doomed) = self.solver.verdict(&next) {
                    self.opt_verdict[i] = Some(v);
                }
                spent += STEP_NODES;
                changed = true;
            }
        }

        // 3. the option tallies AND the best-route scores, each refined ROUND-ROBIN so no one hard search starves
        //    the rest. The frame's remaining budget is split between them (half each), and each advances the first
        //    still-incomplete option under its own cursor - so both fill in over a handful of frames while the
        //    shared memos make each visit more complete. A partial tally is an honest ">="; a partial score an
        //    honest "<=".
        let n = self.options.len();
        if n > 0 && spent < FRAME_BUDGET {
            let remaining = FRAME_BUDGET - spent;
            let half = (remaining / 2).max(1);

            // one option's win/loss tally
            let mut looked = 0;
            while looked < n {
                let i = self.refine % n;
                self.refine = (self.refine + 1) % n;
                looked += 1;
                if !self.opt_paths[i].is_some_and(|p| p.complete) {
                    let next = Combat::apply(&self.state, &self.options[i]);
                    self.counter.grant(half);
                    self.opt_paths[i] = Some(self.counter.count(&next));
                    changed = true;
                    break;
                }
            }

            // one option's best route
            let mut looked = 0;
            while looked < n {
                let i = self.refine_score % n;
                self.refine_score = (self.refine_score + 1) % n;
                looked += 1;
                if !self.opt_score_done[i] {
                    let next = Combat::apply(&self.state, &self.options[i]);
                    self.scorer.grant(remaining - half);
                    self.opt_score[i] = self.scorer.best(&next);
                    self.opt_score_done[i] = !self.scorer.aborted();
                    changed = true;
                    break;
                }
            }
        }
        changed
    }

    /// How many options are fully scored (verdict + tally), for the progress line.
    fn scored_count(&self) -> usize {
        (0..self.options.len())
            .filter(|&i| self.opt_verdict[i].is_some() && self.opt_paths[i].is_some())
            .count()
    }

    fn choose(&mut self, i: usize) {
        if i >= self.options.len() || Combat::outcome(&self.state).is_some() {
            return;
        }
        // Snapshot this decision point for Back BEFORE we mutate: a pointer into the (memoized) tree plus the log
        // height, so undo restores instantly with nothing recomputed. One entry per player decision - the forced
        // and foe declarations that `reposition` auto-advances are part of this same step, not their own.
        self.history.push(Step {
            state: self.state.clone(),
            log_len: self.log.len(),
            options: self.options.clone(),
            entries: self.entries.clone(),
            opt_verdict: self.opt_verdict.clone(),
            opt_paths: self.opt_paths.clone(),
            opt_score: self.opt_score.clone(),
            opt_score_done: self.opt_score_done.clone(),
            verdict: self.verdict,
        });
        self.drill = None;
        let c = self.options[i].clone();
        self.apply_choice(&c);
        self.reposition();
        self.sync_log();
    }

    /// **Back one step.** The first press closes an open crossing (beat two -> beat one); otherwise it pops the
    /// last decision-point snapshot and restores it wholesale. It is a **pointer move, not a recompute**: the
    /// solver/counter memos are kept (a restored position is already solved), the scored options come straight
    /// out of the snapshot, and the log stack unwinds to the height it had at that point.
    fn undo(&mut self) {
        if self.drill.take().is_some() {
            self.dirty = true; // just close the crossing; the move itself has not been made yet
            return;
        }
        let Some(step) = self.history.pop() else {
            return; // nothing to undo
        };
        self.state = step.state;
        self.log.truncate(step.log_len);
        self.options = step.options;
        self.entries = step.entries;
        self.opt_verdict = step.opt_verdict;
        self.opt_paths = step.opt_paths;
        self.opt_score = step.opt_score;
        self.opt_score_done = step.opt_score_done;
        self.verdict = step.verdict;
        self.refine = 0;
        self.refine_score = 0;
        self.sync_log();
        self.dirty = true;
    }

    /// Mirror the **entire** running log to a file, rewritten on every change, so the whole history can be tailed
    /// live as the fight is played. (The on-screen panel and `fight-screen.txt` show only the last lines; this is
    /// all of it.)
    fn sync_log(&self) {
        let mut body = self.log.join("\n");
        if !body.is_empty() {
            body.push('\n');
        }
        let _ = std::fs::write(LOG_FILE, body);
    }

    /// **Pick a crossing (beat one).** Work out who would catch the crosser - deterministically, since the foes
    /// are scripted - and either open the answer beat (if the line reaches you) or, when the crossing is
    /// unopposed, just cross (Push spends nothing and there is nothing to answer).
    fn enter_crossing(&mut self, entry: usize) {
        let Some(Entry::Crossing {
            label,
            dest,
            answers,
        }) = self.entries.get(entry).cloned()
        else {
            return;
        };
        let Some(mover) = self.state.deciding() else {
            return;
        };
        let names: Vec<String> = catchers(self.state.board(), mover, dest)
            .into_iter()
            .map(|j| self.state.board().units[j].name.clone())
            .collect();
        if names.is_empty() {
            // Unopposed: no one to answer, so there is no second beat - cross cleanly and move on.
            let opt = answers
                .iter()
                .find(|(a, _)| *a == Answer::Push)
                .or_else(|| answers.first())
                .map(|&(_, i)| i);
            if let Some(i) = opt {
                self.choose(i);
            }
            return;
        }
        self.drill = Some(Drill {
            label,
            catchers: join_counts(&names),
            answers,
        });
        self.dirty = true;
    }

    /// The best verdict achievable across a crossing's answers (Winnable if any answer is, Doomed only if every
    /// answer is), or `None` while any is still computing - what the collapsed crossing card shows.
    fn agg_verdict(&self, members: &[(Answer, usize)]) -> Option<Verdict> {
        if members
            .iter()
            .any(|&(_, i)| self.opt_verdict[i] == Some(Verdict::Winnable))
        {
            return Some(Verdict::Winnable);
        }
        if members.iter().any(|&(_, i)| self.opt_verdict[i].is_none()) {
            return None;
        }
        if members
            .iter()
            .all(|&(_, i)| self.opt_verdict[i] == Some(Verdict::Doomed))
        {
            Some(Verdict::Doomed)
        } else {
            Some(Verdict::Evaluating)
        }
    }

    /// The win/loss line counts summed over a crossing's answers (every line reachable by crossing, whatever the
    /// answer). `=` when every answer's tally is exhausted, `>=` while any is still a lower bound.
    fn agg_counts(&self, members: &[(Answer, usize)]) -> String {
        if members.iter().any(|&(_, i)| self.opt_paths[i].is_none()) {
            return "counting lines...".into();
        }
        let (mut wins, mut losses, mut complete) = (0u64, 0u64, true);
        for &(_, i) in members {
            let p = self.opt_paths[i].unwrap();
            wins = wins.saturating_add(p.wins);
            losses = losses.saturating_add(p.losses);
            complete &= p.complete;
        }
        counts_line(wins, losses, complete)
    }

    /// The best route achievable across a crossing's answers - the min [`Score`] over them (whichever answer plays
    /// out best), and whether every answer's search is exhausted (so the min is exact, not a provisional `<=`).
    fn agg_route(&self, members: &[(Answer, usize)]) -> (Option<Score>, bool) {
        let mut best: Option<Score> = None;
        let mut done = true;
        for &(_, i) in members {
            done &= self.opt_score_done[i];
            if let Some(s) = self.opt_score[i] {
                best = Some(best.map_or(s, |b| b.min(s)));
            }
        }
        (best, done)
    }

    /// The best route from **this position** - the min over all options' best routes, and whether all are done.
    fn best_route(&self) -> (Option<Score>, bool) {
        let mut best: Option<Score> = None;
        let mut done = !self.opt_score.is_empty();
        for i in 0..self.opt_score.len() {
            done &= self.opt_score_done[i];
            if let Some(s) = self.opt_score[i] {
                best = Some(best.map_or(s, |b| b.min(s)));
            }
        }
        (best, done)
    }

    /// **Apply one declaration and record it in the history** - the single path EVERY choice takes, whether a
    /// player clicked it or a foe (or a forced hero) auto-advanced it. It logs who declared what, applies it, and
    /// - when the choice closes a round - narrates the slip contests and the net damage. Because foes now declare
    /// through this same path, their attacks appear in the log with no reconstruction: a hero that falls has the
    /// creature that felled it named a line or two above.
    fn apply_choice(&mut self, c: &Choice) {
        // Snapshot the whole STATE: the board says what changed, and `pending()` says the acts it changed FROM -
        // the only way to explain damage a slip contest dealt (no declared attack accounts for it).
        let before_state = self.state.clone();
        let before = before_state.board();
        let acting = before_state.deciding();
        let round_before = before_state.round();

        if let Some(idx) = acting {
            // Mark a foe with '*', exactly as the unit table does, so hero and creature declarations read apart.
            let mark = if before.units[idx].side == Side::Party {
                ""
            } else {
                "*"
            };
            self.log.push(format!(
                "{mark}{}: {}",
                before.units[idx].name,
                describe(before, c)
            ));
        }

        // The full act vector this apply would resolve with, if it is the round-closer: every body's pending
        // declaration, plus the one being made now. When it is NOT the closer this is unused.
        let mut acts: Vec<Act> = before_state
            .pending()
            .iter()
            .map(|p| p.unwrap_or(Act::Hold))
            .collect();
        if let (Some(idx), Choice::Act(a)) = (acting, c) {
            acts[idx] = *a;
        }

        self.state = Combat::apply(&self.state, c);

        // A round resolves on exactly one apply - the one where the round counter advances. (There is no setup,
        // so the fight opens on round 1 and every advance is a resolved round that may have drawn blood.)
        let resolved = self.state.round() != round_before;
        if resolved {
            // The round, phase by phase: re-run the deterministic resolution on a throwaway clone (it never
            // touches `self.state`) and walk the transcript, so every pool addition, card flip, crossing and death
            // is logged UNDER the ring it happened in.
            let events = narrate_round(before, &acts);
            if events.is_empty() {
                self.log.push("  (no blood drawn)".into());
            } else {
                for line in events {
                    self.log.push(line);
                }
            }
        }
        if self.state.round() != round_before {
            match Combat::outcome(&self.state) {
                Some(o) => self.log.push(format!("========== {o:?} ==========")),
                None => self.log.push(format!(
                    "================= round {} =================",
                    self.state.round()
                )),
            }
        }
    }
}

fn act_answer(a: &Act) -> Option<Answer> {
    match a {
        Act::Cross(_, x) => Some(*x),
        _ => None,
    }
}

/// The verb for a body's attack, by **reach x shape** - so the log line carries melee/ranged and single/area in
/// the verb itself, no tag needed. A horde keeps its shape verb; the `x N bodies` in the damage clause marks the
/// volley.
fn strike_verb(u: &Combatant) -> &'static str {
    match (u.ranged && !u.melee, u.aoe) {
        (true, true) => "salvos on",
        (true, false) => "fires on",
        (false, true) => "sweeps",
        (false, false) => "strikes",
    }
}

/// A `SubPhaseLog` phase string to its **coordinate** in the ring structure: `(ring number, ring NAME, sub-phase
/// number, sub-phase name)`, nearest-first. So the log addresses every event as `ring.subphase`, and a reader
/// always knows where in the round they are.
fn phase_coord(phase: &'static str) -> (u8, &'static str, u8, &'static str) {
    match phase {
        "Inner Ring: Outriders" => (1, "INNER", 1, "Outriders"),
        "Crossing Ring: Intercept" => (2, "CROSSING", 1, "Intercept"),
        "Crossing Ring: Volley" => (2, "CROSSING", 2, "Volley"),
        "Crossing Ring: Raid" => (2, "CROSSING", 4, "Raid"),
        "Outer Ring: Fire" => (3, "OUTER", 1, "Fire"),
        "Outer Ring: Clash" => (3, "OUTER", 2, "Clash"),
        other => (0, "", 0, other),
    }
}

/// **The round, phase by phase - every state change spelled out, none left invisible.** Re-runs the
/// deterministic resolution on a throwaway clone of the pre-round board (identical to what `self.state` just
/// resolved) and walks the [`SubPhaseLog`] transcript `play_round` returns.
///
/// Output is a **coordinate language**: a `[ring N] NAME` header when a ring opens, a `ring.subphase Subphase`
/// header per active sub-phase, and every event line prefixed with its **exchange step** (`reach` / `dodge` /
/// `strike` / `absorb` / `move` / `death`) - so any line locates itself as `round . ring.subphase . step`.
///
/// **The completeness rule: a body's every mutable field is snapshotted each phase, and a change to any of them
/// prints a line.** Tempo, Health, rank and region are all diffed against the phase before, so no spend, flip,
/// crossing or dissolution can happen silently. A tempo spend with no blow behind it (a slipper paying to evade,
/// a catcher whose target slipped away) was exactly the kind of change that used to hide; now it does not.
///
/// Within a phase, in order:
/// - **Strikes** - one line per attacker: the tempo it spent (reaching + pouring), the Might, and the damage it
///   banked (`(Might - armor)` per blow, a horde swinging its whole body count at once; against a horde it *fells
///   bodies* instead).
/// - **Tempo spent with no blow** - a slipper evading, or a reach the target slipped: the cost, made visible.
/// - **Crossings** landed or turned back, with the rank they take.
/// - **Absorb / flips** - the damage a target soaked, the Grit bar, the Health cards that flipped (or fell short),
///   and any remainder discarded when the pile closes.
/// - **Rank / region** changes not already narrated (a dissolved outrider rejoining its line).
/// - **Deaths**.
///
/// Snapshots enter the first phase at full Health and full Tempo (Cadence, stood back up by the Reset); indices are
/// stable across the clone, so names / stats are read from `before`. A phase that did nothing prints nothing.
fn narrate_round(before: &Board, acts: &[Act]) -> Vec<String> {
    let mut clone = before.clone();
    let transcript = play_round(&mut clone, acts);

    let rank_word = |r: Rank| match r {
        Rank::Vanguard => "a Vanguard",
        Rank::Rearguard => "a Rearguard",
        Rank::Outrider => "an Outrider",
    };

    let mut out = Vec::new();
    let mut current_ring = 0u8; // emit a "[ring N] NAME" header only when the ring changes
    let mut prev_hp: Vec<u32> = before.units.iter().map(|u| u.health).collect();
    let mut prev_tp: Vec<u32> = before.units.iter().map(|u| u.cadence).collect(); // Reset stands tempo up to Cadence
    let mut prev_rk: Vec<Rank> = before.ranks.clone();
    let mut prev_rg: Vec<u8> = before.regions.clone();
    for log in &transcript {
        // Each event tagged with its exchange step; rendered under the sub-phase header with a step column.
        let mut lines: Vec<(&'static str, String)> = Vec::new();
        // The crossing LAND (through/aborted) is its own sub-phase (2.3), even though the resolver attaches it to
        // the Volley log - so it does not read as a "volley".
        let mut land_lines: Vec<String> = Vec::new();

        // --- Strikes: sum blows per (attacker -> target) in strike order, then one line each. ---
        let mut order: Vec<(usize, usize)> = Vec::new();
        let mut blows: Vec<u32> = Vec::new();
        for hit in &log.hits {
            let key = (hit.attacker, hit.target);
            match order.iter().position(|k| *k == key) {
                Some(p) => blows[p] += hit.hits,
                None => {
                    order.push(key);
                    blows.push(hit.hits);
                }
            }
        }
        // Each strike, in the pool -> flow vocabulary: the attacker FLIPS tempo (at its Finesse) to GENERATE the
        // reach that lands the contact, then STRIKES for damage (Might per blow). The reach/tempo is a per-attacker
        // fact - a sweep hits many for one flip - so it is stated once, on the attacker's first strike this phase.
        let mut tempo_said: Vec<usize> = Vec::new();
        for (&(a, t), &n) in order.iter().zip(&blows) {
            let (an, tn) = (&before.units[a].name, &before.units[t].name);
            // Reach x shape rides the VERB (melee/ranged, single/area). Rank is NOT tagged here - it is listed in
            // the opening roster and narrated when it changes.
            let verb = strike_verb(&before.units[a]);
            let mult = if before.units[a].horde {
                prev_hp[a].max(1) // body count ENTERING this phase - what `land` and the bid both read
            } else {
                1
            };
            // The reach clause (once per attacker): tempo flipped x Finesse x bodies = the reach it generated, plus
            // any tempo poured for extra strikes. Recovered from the recorded bid, so it always matches `land`.
            let reach = if tempo_said.contains(&a) {
                String::new()
            } else {
                tempo_said.push(a);
                let f = before.units[a].finesse.max(1);
                let total = prev_tp[a].saturating_sub(log.tempo[a]);
                match log
                    .reaches
                    .iter()
                    .find(|r| r.attacker == a && r.target == t && !r.evaded)
                {
                    Some(r) => {
                        let rt = r.bid / (f * mult).max(1); // tempo cards that made the bid (bid = rt x f x bodies)
                        let fclause = if before.units[a].horde {
                            format!("Finesse {f} x {mult} bodies")
                        } else {
                            format!("Finesse {f}")
                        };
                        let pour = total.saturating_sub(rt);
                        let poured = if pour > 0 {
                            format!("pours {pour} more tempo, ")
                        } else {
                            String::new()
                        };
                        format!(
                            "flips {rt} tempo at {fclause} to generate {} reach, {poured}",
                            r.bid
                        )
                    }
                    // A sweep forms no reach contest (unevadable); the verb already says it swept.
                    None => format!("flips {total} tempo, "),
                }
            };
            let body = if before.units[t].horde {
                // A horde's bodies are separate Grit-strong pools, no spill. A blow fells a body iff it penetrates
                // (Might - armor >= Grit); a sweep clears the WHOLE pack at once, an aimed blow one body per blow.
                // Both operands of the gate on the page.
                let g = before.units[t].grit.max(1);
                let m = before.units[a].might;
                if m.saturating_sub(before.units[t].armor) < g {
                    format!("cannot dent the pack (Might {m} < Grit {g})")
                } else if before.units[a].aoe {
                    format!("fells the whole pack, {n} bodies (Might {m} >= Grit {g})")
                } else {
                    format!("fells {n} bodies (Might {m} >= Grit {g}, one per blow)")
                }
            } else {
                // Aimed fire on a horde banks into its Grit pile like any body (a horde is defence-normal now).
                // Banked damage = (Might - armor) per blow; a horde attacker swings its whole body count at once.
                let per_blow = before.units[a].might.saturating_sub(before.units[t].armor);
                let dmg = per_blow * mult * n;
                let armor = before.units[t].armor;
                // Show armor only when it bites, so `Might - armor = per-blow` is legible (0 for the roster today).
                let base = if armor > 0 {
                    format!(
                        "Might {} - armor {armor} = {per_blow}",
                        before.units[a].might
                    )
                } else {
                    format!("Might {}", before.units[a].might)
                };
                let how = if before.units[a].horde {
                    format!("{base} x {mult} bodies")
                } else if n > 1 {
                    format!("{base}, {n} strikes")
                } else {
                    base
                };
                format!("for {dmg} damage ({how})")
            };
            lines.push(("strike", format!("{an} {verb} {tn}: {reach}{body}")));
        }

        // --- Tempo spent with NO blow behind it - the slip contest, ordered CAUSE BEFORE EFFECT. Resolution is
        // sequential: the reaching side commits first (`engage`), then the defender responds having seen the exact
        // bid (`resolve_evade`). So the reaches are logged first, then the dodges that answer them. Both are
        // products (flip tempo x Finesse = reach); the higher reach wins. A landed strike was stated above. ---
        //
        // Pass 1 - the reaches that were dodged (a landed reach became a Hit, so a body still here reached for a
        // target that out-reached it).
        for i in 0..log.tempo.len() {
            let spent = prev_tp[i].saturating_sub(log.tempo[i]);
            if spent == 0 || log.hits.iter().any(|h| h.attacker == i) {
                continue;
            }
            let f = before.units[i].finesse.max(1);
            let mult = if before.units[i].horde {
                prev_hp[i].max(1)
            } else {
                1
            };
            for r in log.reaches.iter().filter(|r| r.attacker == i) {
                let cards = r.bid / (f * mult).max(1);
                let fclause = if before.units[i].horde {
                    format!("Finesse {f} x {mult} bodies")
                } else {
                    format!("Finesse {f}")
                };
                lines.push((
                    "reach",
                    format!(
                        "{} reaches for {}: flips {cards} tempo at {fclause} to generate {} reach, dodged",
                        before.units[i].name,
                        before.units[r.target].name,
                        r.bid
                    ),
                ));
            }
        }
        // Pass 2 - the dodge that ANSWERED each reach (it saw the bid, then out-reached it), plus any other tempo
        // that bought no reach.
        for i in 0..log.tempo.len() {
            let spent = prev_tp[i].saturating_sub(log.tempo[i]);
            if spent == 0
                || log.hits.iter().any(|h| h.attacker == i)
                || log.reaches.iter().any(|r| r.attacker == i)
            {
                continue; // no spend, already struck, or already shown as a reacher in pass 1
            }
            let name = &before.units[i].name;
            if let Some(worst) = log
                .reaches
                .iter()
                .filter(|r| r.target == i && r.evaded)
                .map(|r| r.bid)
                .max()
            {
                // The same flow, responding: flip tempo to generate reach that OUTWEIGHS the incoming bid. Both
                // values on the page (multiply, never divide): "4 reach clears the 2 reaching it".
                let f = before.units[i].finesse.max(1);
                let dodge = spent * f;
                lines.push((
                    "dodge",
                    format!(
                        "{name}: flips {spent} tempo at Finesse {f} to generate {dodge} reach, dodging the {worst} reaching it"
                    ),
                ));
            } else {
                lines.push((
                    "reach",
                    format!("{name}: flips {spent} tempo, no reach connects"),
                ));
            }
        }

        // --- Crossings that resolved this phase (the Land step attaches through/aborted to the Volley log). ---
        for &i in &log.through {
            let name = &before.units[i].name;
            let verb = match act_answer(&acts[i]) {
                Some(Answer::Push) => "pushes through the line",
                _ => "slips through the line",
            };
            land_lines.push(format!("{name}: {verb}, now {}", rank_word(log.ranks[i])));
        }
        for &i in &log.aborted {
            land_lines.push(format!(
                "{}: turns and fights at the line",
                before.units[i].name
            ));
        }

        // --- Absorb / flips (normal bodies): the pile closes each sub-phase, so pair damage with the cards it
        // flipped. A HORDE is not here - its bodies are felled per penetrating blow (the strike line), not piled -
        // so it is skipped. First total the banked damage per target (armor per blow, a horde attacker's whole body
        // count at once) - same formula as the strike lines - so the two always agree. ---
        let mut dmg_to = vec![0u32; log.health.len()];
        for h in &log.hits {
            if before.units[h.target].horde {
                continue; // a horde takes body-fells (per blow), not pile damage
            }
            let per_blow = before.units[h.attacker]
                .might
                .saturating_sub(before.units[h.target].armor);
            let bodies = if before.units[h.attacker].horde {
                prev_hp[h.attacker].max(1)
            } else {
                1
            };
            dmg_to[h.target] += per_blow * bodies * h.hits;
        }
        for i in 0..log.health.len() {
            if before.units[i].horde {
                continue; // felled per blow on the strike line, not a Grit pile
            }
            let (h0, h1) = (prev_hp[i], log.health[i]);
            let name = &before.units[i].name;
            let grit = before.units[i].grit.max(1);
            if h1 < h0 {
                // Flip a Health card at Grit each to ABSORB damage. Flipped x Grit is what the cards soaked; the
                // pile closes each sub-phase, so any damage past that is discarded.
                let flipped = h0 - h1;
                let absorbed = flipped * grit;
                let overflow = dmg_to[i].saturating_sub(absorbed);
                let over = if overflow > 0 {
                    format!(" ({overflow} overflow, discarded)")
                } else {
                    String::new()
                };
                let remain = if h1 > 0 {
                    format!(", {h1} health left")
                } else {
                    String::new()
                };
                lines.push((
                    "absorb",
                    format!(
                        "{name}: flips {flipped} health at Grit {grit} to absorb {absorbed} damage{over}{remain}"
                    ),
                ));
            } else if dmg_to[i] > 0 {
                // Banked damage that flipped no card: short of Grit, and the pile clears when this sub-phase closes.
                lines.push((
                    "absorb",
                    format!(
                        "{name}: takes {} damage - under Grit {grit}, no health flips (discarded)",
                        dmg_to[i]
                    ),
                ));
            }
        }

        // --- Rank / region changes not already narrated by a crossing (a dissolved outrider rejoining its line). A
        // crosser's new rank was shown on its crossing line; anything else that moved or changed rank is caught
        // here, so no repositioning is silent. ---
        for i in 0..log.ranks.len() {
            if log.through.contains(&i) {
                continue; // its move was narrated as a crossing
            }
            let rank_changed = prev_rk[i] != log.ranks[i];
            let region_changed = prev_rg[i] != log.regions[i];
            if !rank_changed && !region_changed {
                continue;
            }
            let name = &before.units[i].name;
            if prev_rk[i] == Rank::Outrider && log.ranks[i] != Rank::Outrider {
                // Its host formation was wiped, so the outrider state dissolved.
                if region_changed {
                    lines.push((
                        "move",
                        format!(
                            "{name}: outrider dissolves - rejoins its own line as {}",
                            rank_word(log.ranks[i])
                        ),
                    ));
                } else {
                    lines.push((
                        "move",
                        format!(
                            "{name}: outrider dissolves - reforms as {} where it stands",
                            rank_word(log.ranks[i])
                        ),
                    ));
                }
            } else if region_changed {
                lines.push((
                    "move",
                    format!(
                        "{name}: moves across the line (now {})",
                        rank_word(log.ranks[i])
                    ),
                ));
            } else {
                lines.push((
                    "move",
                    format!("{name}: becomes {}", rank_word(log.ranks[i])),
                ));
            }
        }

        // --- Deaths this phase. ---
        for &i in &log.fallen {
            let name = &before.units[i].name;
            if before.units[i].horde {
                lines.push(("death", format!("{name}: no bodies remaining, wiped out")));
            } else {
                lines.push(("death", format!("{name}: no health remaining, downed")));
            }
        }

        prev_hp = log.health.clone();
        prev_tp = log.tempo.clone();
        prev_rk = log.ranks.clone();
        prev_rg = log.regions.clone();
        if !lines.is_empty() || !land_lines.is_empty() {
            // The coordinate: a ring header when the ring opens, then numbered sub-phases, then the step column.
            let (rn, rname, sn, sname) = phase_coord(log.phase);
            if rn != current_ring {
                out.push(format!("[ring {rn}] {rname}"));
                current_ring = rn;
            }
            if !lines.is_empty() {
                out.push(format!("  {rn}.{sn} {sname}"));
                // Order events by exchange step (reach -> dodge -> strike -> absorb -> move -> death) so the line
                // reads in the order it resolves, then print each in its step column.
                let rank = |s: &str| match s {
                    "reach" => 0,
                    "dodge" => 1,
                    "strike" => 2,
                    "absorb" => 3,
                    "move" => 4,
                    _ => 5, // death, and anything else, last
                };
                let mut evs = lines;
                evs.sort_by_key(|(s, _)| rank(s));
                for (step, text) in evs {
                    out.push(format!("      {step:<6} {text}"));
                }
            }
            // The Land (2.3): the crossers that arrived or turned back, all `move`.
            if !land_lines.is_empty() {
                out.push("  2.3 Land".to_string());
                for text in land_lines {
                    out.push(format!("      {:<6} {text}", "move"));
                }
            }
        }
    }
    out
}

/// Join names, collapsing repeats into "The Swarm x2" so a pack reads as one catcher, not a wall of text.
fn join_counts(names: &[String]) -> String {
    let mut order: Vec<String> = Vec::new();
    let mut counts: Vec<usize> = Vec::new();
    for n in names {
        match order.iter().position(|o| o == n) {
            Some(p) => counts[p] += 1,
            None => {
                order.push(n.clone());
                counts.push(1);
            }
        }
    }
    order
        .iter()
        .zip(counts)
        .map(|(n, c)| {
            if c > 1 {
                format!("{n} x{c}")
            } else {
                n.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The tempo a body has to spend in a round: its Cadence pool (`refresh_round`), hordes included. A horde no
/// longer swarms with body-count tempo - its size now shows up as a body-count volley of damage and a body-count
/// reach, not extra tempo (see `rules::combat::regions::land` / `engage`).
fn round_tempo(u: &Combatant) -> u32 {
    u.cadence
}

/// The single kit that fields a **solo** encounter: the requested one if it names a real kit, else the
/// keystone's counter (so `fight 3` opens the Bombardier-vs-Storm solo the diagonal actually tests). Returns the
/// kit name and its ROSTER spec.
fn solo_kit(
    encounter: usize,
    requested: Option<&str>,
) -> (&'static str, (&'static str, [u8; 5], &'static str)) {
    let e = &catalog::ENCOUNTERS[encounter % catalog::ENCOUNTERS.len()];
    let counter = catalog::creature(e.keystone)
        .map(catalog::creature_counter)
        .unwrap_or("");
    let want = requested.unwrap_or(counter);
    let spec = catalog::ROSTER
        .iter()
        .copied()
        .find(|(n, _, _)| n.eq_ignore_ascii_case(want))
        .or_else(|| {
            // an unknown kit name falls back to the counter, so a typo still yields a legal solo
            catalog::ROSTER
                .iter()
                .copied()
                .find(|(n, _, _)| n.eq_ignore_ascii_case(counter))
        })
        .unwrap_or(catalog::ROSTER[0]);
    (spec.0, spec)
}

/// Build the state for an encounter. A **party** encounter musters the whole roster; a **solo** encounter is
/// fielded by exactly ONE kit ([`solo_kit`]) - the requirement that a solo is a single-kit test, matching the
/// diagonal, not the full party.
fn build(encounter: usize, requested_kit: Option<&str>) -> State {
    let e = &catalog::ENCOUNTERS[encounter % catalog::ENCOUNTERS.len()];
    let mut units: Vec<Combatant> = if e.party {
        catalog::ROSTER.iter().copied().map(kit).collect()
    } else {
        vec![kit(solo_kit(encounter, requested_kit).1)]
    };
    units.extend(encounter_beasts(e)); // numbered when duplicated, so two Walls read apart
    State::new(units)
}

// ---- choice / board formatting -------------------------------------------------------------------------

/// A choice label. The active body is shown once above the options and marked on the table, so it is never
/// repeated per action.
fn describe(b: &Board, c: &Choice) -> String {
    let Choice::Act(a) = c;
    act_label(b, a)
}

fn act_label(b: &Board, a: &Act) -> String {
    let ans = |x: &Answer| match x {
        Answer::Evade => "evade",
        Answer::Push => "push",
        Answer::Abort => "abort",
    };
    // Name the target WITH its current health, so two same-named bodies in different states (e.g. two Walls at 2 hp
    // and 4 hp) read as the distinct choices they are. A horde's health is its body count.
    let who = |t: usize| {
        let u = &b.units[t];
        let kind = if u.horde { "bodies" } else { "hp" };
        format!("{} ({} {kind})", u.name, u.health)
    };
    match a {
        Act::Clash(t) => format!("Clash {}", who(*t)),
        Act::Cross(Some(t), x) => format!("Raid {} / {}", who(*t), ans(x)),
        Act::Melee(t) => format!("Melee {}", who(*t)),
        Act::Cross(None, x) => format!("Slip into their line / {}", ans(x)),
        Act::Hold => "Hold".into(),
    }
}

/// Group the flat option list into narrative entries: a crossing's three answers collapse into one entry - a
/// `Cross(Some(target))` raid (grouped per target) or a `Cross(None)` slip - since legal_acts emits them adjacent.
/// Everything else stays a direct choice. Order is preserved.
fn build_entries(board: &Board, options: &[Choice]) -> Vec<Entry> {
    // A crossing always heads for the one enemy region (where the foes stand); the drill uses `dest` to name the
    // catchers that would intercept it.
    let enemy_region = board
        .units
        .iter()
        .position(|u| u.side == Side::Foe)
        .map(|i| board.regions[i])
        .unwrap_or(0);
    let mut entries = Vec::new();
    let mut i = 0;
    while i < options.len() {
        let Choice::Act(a) = &options[i];
        match a {
            Act::Cross(Some(t), _) => {
                let target = *t;
                let label = format!("Raid {}", board.units[target].name);
                let mut answers = Vec::new();
                while let Some(Choice::Act(Act::Cross(Some(t2), ans))) = options.get(i) {
                    if *t2 != target {
                        break;
                    }
                    answers.push((*ans, i));
                    i += 1;
                }
                entries.push(Entry::Crossing {
                    label,
                    dest: board.regions[target],
                    answers,
                });
            }
            Act::Cross(None, _) => {
                let label = "Slip into their line".to_string();
                let mut answers = Vec::new();
                while let Some(Choice::Act(Act::Cross(None, ans))) = options.get(i) {
                    answers.push((*ans, i));
                    i += 1;
                }
                entries.push(Entry::Crossing {
                    label,
                    dest: enemy_region,
                    answers,
                });
            }
            _ => {
                entries.push(Entry::Direct(i));
                i += 1;
            }
        }
    }
    entries
}

/// The win/loss line-count line, with the completeness sign: `=` once the tally is exhausted (the whole subtree
/// was walked), `>=` while it is still a growing lower bound.
fn counts_line(wins: u64, losses: u64, complete: bool) -> String {
    let sign = if complete { "=" } else { ">=" };
    format!("{sign}{} win / {sign}{} lose", abbrev(wins), abbrev(losses))
}

/// The counts line for one scored option, or a placeholder while it is still counting.
fn opt_counts(paths: Option<Paths>) -> String {
    match paths {
        Some(p) => counts_line(p.wins, p.losses, p.complete),
        None => "counting lines...".into(),
    }
}

/// How each crossing answer reads once you know who caught you - the `Abort` option is named after the actual
/// catcher ("Turn and fight The Wall"), not the abstract verb.
fn answer_label(a: Answer, catchers: &str) -> String {
    match a {
        Answer::Evade => "Evade the line".into(),
        Answer::Push => "Push through, take the hits".into(),
        Answer::Abort => format!("Turn and fight {catchers}"),
    }
}

// ---- input ---------------------------------------------------------------------------------------------

#[derive(Component, Clone, Copy)]
enum Hit {
    /// Apply `options[usize]` directly (a direct choice, or a chosen crossing answer).
    Option(usize),
    /// Open the second beat of `entries[usize]` (a crossing): show who caught you, then the answers.
    Crossing(usize),
    /// Cancel an open crossing and return to the top-level options.
    Back,
    /// Step back one decision (close a crossing first, else pop the undo stack).
    Undo,
    Reset,
    Next,
}

#[derive(Component)]
struct Root;

fn is_dirty(f: Res<Fight>) -> bool {
    f.dirty
}

/// Do one frame's scoring and redraw if a result landed. The window stays live because this is bounded.
fn grind(mut f: ResMut<Fight>) {
    if f.grind() {
        f.dirty = true;
    }
}

fn on_click(mut f: ResMut<Fight>, q: Query<(&Interaction, &Hit), Changed<Interaction>>) {
    for (i, hit) in &q {
        if *i != Interaction::Pressed {
            continue;
        }
        match *hit {
            Hit::Option(k) => f.choose(k),
            Hit::Crossing(k) => f.enter_crossing(k),
            Hit::Back => {
                f.drill = None;
                f.dirty = true;
            }
            Hit::Undo => f.undo(),
            Hit::Reset => f.reset(),
            Hit::Next => {
                f.encounter += 1;
                f.reset();
            }
        }
    }
}

// ---- rendering -----------------------------------------------------------------------------------------

fn verdict_color(v: Verdict) -> Color {
    match v {
        Verdict::Winnable => GOOD,
        Verdict::Evaluating => WARN,
        Verdict::Doomed => BAD,
    }
}

fn text(p: &mut ChildSpawnerCommands, s: impl Into<String>, size: f32, c: Color) {
    p.spawn((
        Text::new(s),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(c),
    ));
}

fn rebuild(mut commands: Commands, mut f: ResMut<Fight>, old: Query<Entity, With<Root>>) {
    for e in &old {
        commands.entity(e).despawn();
    }
    f.dirty = false;
    let f = &*f;

    commands
        .spawn((
            Root,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                padding: UiRect::all(Val::Px(12.0)),
                column_gap: Val::Px(12.0),
                ..default()
            },
        ))
        .with_children(|root| {
            // left column: header, unit table, log
            root.spawn(Node {
                width: Val::Percent(52.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|col| {
                header(col, f);
                unit_table(col, f);
                controls(col);
                log_panel(col, f);
            });

            // right column: the options
            root.spawn((
                Node {
                    width: Val::Percent(48.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(6.0),
                    overflow: Overflow::clip(),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(PANEL),
            ))
            .with_children(|col| options_panel(col, f));
        });

    // Mirror the screen to a file, clobbered every redraw. Not a log - a snapshot of exactly what is on screen
    // right now, so it can be read and asked about without describing it. Written here so it can never drift
    // from the render: same data, same moment.
    let _ = std::fs::write(SCREEN_FILE, screen_text(f));
}

/// Where the on-screen snapshot is written (relative to the run directory - the repo root under `cargo run`).
const SCREEN_FILE: &str = "fight-screen.txt";

/// Where the FULL running log is mirrored (every line, not just the on-screen tail), rewritten on every change.
const LOG_FILE: &str = "fight-log.txt";

/// A plain-text mirror of everything on screen: the same header, unit table, options, and history the UI draws,
/// with the same abbreviations. Pending values read `...` / `counting...`, exactly as they do on screen.
fn screen_text(f: &Fight) -> String {
    use std::fmt::Write;
    let b = f.state.board();
    let mut s = String::new();
    let e = f.enc();

    writeln!(s, "{} - {}", e.location, e.title).ok();
    match (Combat::outcome(&f.state), f.verdict) {
        (Some(o), _) => writeln!(s, "*** {o:?} ***").ok(),
        (None, Some(v)) => writeln!(s, "round {}   position: {v:?}", f.state.round()).ok(),
        (None, None) => writeln!(s, "round {}   position: computing...", f.state.round()).ok(),
    };
    if let Some(i) = f.state.deciding() {
        writeln!(s, "acting: {}", b.units[i].name).ok();
    }
    if Combat::outcome(&f.state).is_none() {
        let (done, n) = (f.scored_count(), f.options.len());
        if f.verdict.is_none() || done < n {
            writeln!(s, "scoring options... {done}/{n} done").ok();
        }
    }

    // The unit table - same columns and widths as the UI.
    writeln!(s, "\nUNITS").ok();
    let cols = ["unit", "rk", "M", "V", "G", "C", "F", "hp", "tp", "kind"];
    let w = [16usize, 4, 3, 3, 3, 3, 3, 4, 4, 10];
    let row = |cells: &[String]| -> String {
        cells
            .iter()
            .zip(w)
            .map(|(c, width)| format!("{c:<width$}"))
            .collect::<String>()
    };
    writeln!(s, "{}", row(&cols.map(String::from))).ok();
    let active = f.state.deciding();
    for i in 0..b.units.len() {
        let u = &b.units[i];
        let mark = if active == Some(i) { "> " } else { "  " };
        let side = if u.side == Side::Party { "" } else { "*" };
        let place = match b.ranks[i] {
            Rank::Vanguard => "F",
            Rank::Rearguard => "b",
            Rank::Outrider => "o",
        }
        .to_string();
        let mut kind = String::new();
        for (flag, tag) in [
            (u.melee, "me "),
            (u.ranged, "rg "),
            (u.aoe, "aoe "),
            (u.horde, "hd"),
        ] {
            if flag {
                kind.push_str(tag);
            }
        }
        let dead = if u.fallen { " (down)" } else { "" };
        let tp = if u.fallen {
            "-".to_string()
        } else {
            round_tempo(u).to_string()
        };
        let vitality = f.max_health.get(i).copied().unwrap_or(u.health);
        writeln!(
            s,
            "{}",
            row(&[
                format!("{mark}{side}{}{dead}", u.name),
                place,
                u.might.to_string(),
                vitality.to_string(),
                u.grit.to_string(),
                u.cadence.to_string(),
                u.finesse.to_string(),
                u.health.to_string(),
                tp,
                kind.trim().to_string(),
            ])
        )
        .ok();
    }
    writeln!(
        s,
        "(M might  V vitality  G grit  C cadence  F finesse  hp current health  tp tempo/round, horde=bodies;  rk = rank, F vanguard / b rearguard / o outrider (loose in their line))"
    )
    .ok();

    // The options - same order and content as the buttons, including the two-beat crossings.
    writeln!(s, "\nOPTIONS").ok();
    if Combat::outcome(&f.state).is_none() {
        let (rs, rd) = f.best_route();
        writeln!(
            s,
            "best route from here: {}  (downed/rounds/hp, minimized in that priority)",
            route_cell(rs, rd)
        )
        .ok();
    }
    let vtag = |v: Option<Verdict>| v.map(|x| format!("{x:?}")).unwrap_or_else(|| "...".into());
    if Combat::outcome(&f.state).is_some() {
        writeln!(s, "the fight is over.").ok();
    } else if let Some(drill) = &f.drill {
        // Beat two: the line caught you, now answer it.
        writeln!(
            s,
            "{} - {} catches you at the line. How do you answer?",
            drill.label, drill.catchers
        )
        .ok();
        for (n, &(ans, opt)) in drill.answers.iter().enumerate() {
            writeln!(
                s,
                "[{n}] {:<28} {:<12} {:<22} {}",
                answer_label(ans, &drill.catchers),
                vtag(f.opt_verdict[opt]),
                opt_counts(f.opt_paths[opt]),
                fmt_route(f.opt_score[opt], f.opt_score_done[opt])
            )
            .ok();
        }
        writeln!(s, "[<] choose a different action").ok();
    } else {
        // Beat one: entries, crossings collapsed to one line.
        for (k, entry) in f.entries.iter().enumerate() {
            let (title, v, counts, route) = match entry {
                Entry::Direct(opt) => (
                    describe(b, &f.options[*opt]),
                    vtag(f.opt_verdict[*opt]),
                    opt_counts(f.opt_paths[*opt]),
                    fmt_route(f.opt_score[*opt], f.opt_score_done[*opt]),
                ),
                Entry::Crossing { label, answers, .. } => {
                    let (rs, rd) = f.agg_route(answers);
                    (
                        format!("{label}  >"),
                        vtag(f.agg_verdict(answers)),
                        f.agg_counts(answers),
                        fmt_route(rs, rd),
                    )
                }
            };
            writeln!(s, "[{k}] {title:<28} {v:<12} {counts:<22} {route}").ok();
        }
    }

    if !f.log.is_empty() {
        writeln!(s, "\nHISTORY - what actually happened (most recent last)").ok();
        for line in f
            .log
            .iter()
            .rev()
            .take(HISTORY_LINES)
            .collect::<Vec<_>>()
            .iter()
            .rev()
        {
            writeln!(s, "  {line}").ok();
        }
    }
    s
}

fn header(p: &mut ChildSpawnerCommands, f: &Fight) {
    let e = f.enc();
    p.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(2.0),
        ..default()
    })
    .with_children(|h| {
        text(h, format!("{} - {}", e.location, e.title), 20.0, INK);
        // A solo encounter is a single-kit test; name which kit is fielded so it reads apart from a party fight.
        if !e.party {
            let solo: Vec<&str> = f
                .state
                .board()
                .units
                .iter()
                .filter(|u| u.side == Side::Party)
                .map(|u| u.name.as_str())
                .collect();
            text(h, format!("solo - {}", solo.join(", ")), 12.0, MUTED);
        }
        let status = match (Combat::outcome(&f.state), f.verdict) {
            (Some(o), _) => (
                format!("*** {o:?} ***"),
                verdict_color(f.verdict.unwrap_or(Verdict::Evaluating)),
            ),
            (None, Some(v)) => (
                format!("round {}   position: {v:?}", f.state.round()),
                verdict_color(v),
            ),
            (None, None) => (
                format!("round {}   position: computing...", f.state.round()),
                WARN,
            ),
        };
        text(h, status.0, 14.0, status.1);
        // Say WHO is deciding, so an action is never ambiguous about which body.
        if let Some(i) = f.state.deciding() {
            let who = &f.state.board().units[i].name;
            text(h, format!("acting: {who}"), 13.0, GOOD);
        }
        // The best route from HERE, under the priority order - the headline number to steer by.
        if Combat::outcome(&f.state).is_none() {
            let (rs, rd) = f.best_route();
            text(
                h,
                format!("best route: {}  (downed/rounds/hp)", route_cell(rs, rd)),
                12.0,
                GOOD,
            );
        }
        // What you are waiting on: a live progress line, so a busy UI is never a silent one.
        if Combat::outcome(&f.state).is_none() {
            let n = f.options.len();
            let done = f.scored_count();
            if f.verdict.is_none() || done < n {
                text(
                    h,
                    format!("scoring options... {done}/{n} done"),
                    12.0,
                    MUTED,
                );
            }
        }
    });
}

/// A compact unit table: name, side, region/post, the five stats abbreviated, HP, and reach flags.
fn unit_table(p: &mut ChildSpawnerCommands, f: &Fight) {
    let b = f.state.board();
    p.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(8.0)),
            row_gap: Val::Px(1.0),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(SUNK),
    ))
    .with_children(|t| {
        // header row
        row_cells(
            t,
            &["unit", "rk", "M", "V", "G", "C", "F", "hp", "tp", "kind"],
            MUTED,
            12.0,
        );
        let active = f.state.deciding();
        for i in 0..b.units.len() {
            let u = &b.units[i];
            let is_active = active == Some(i);
            let colour = if is_active {
                GOOD // the hero currently deciding, matched to the ">" heading above the options
            } else if u.fallen {
                MUTED
            } else if u.side == Side::Party {
                INK
            } else {
                Color::srgb(0.95, 0.72, 0.72)
            };
            let side = if u.side == Side::Party { "" } else { "*" };
            let place = match b.ranks[i] {
                Rank::Vanguard => "F",
                Rank::Rearguard => "b",
                Rank::Outrider => "o",
            }
            .to_string();
            let mut kind = String::new();
            if u.melee {
                kind.push_str("me ");
            }
            if u.ranged {
                kind.push_str("rg ");
            }
            if u.aoe {
                kind.push_str("aoe ");
            }
            if u.horde {
                kind.push_str("hd");
            }
            let marker = if is_active { "> " } else { "  " };
            let tp = if u.fallen {
                "-".to_string()
            } else {
                round_tempo(u).to_string()
            };
            let vitality = f.max_health.get(i).copied().unwrap_or(u.health);
            row_cells(
                t,
                &[
                    &format!("{marker}{side}{}", u.name),
                    &place,
                    &u.might.to_string(),
                    &vitality.to_string(), // V = Vitality, the FULL health
                    &u.grit.to_string(),
                    &u.cadence.to_string(),
                    &u.finesse.to_string(),
                    &u.health.to_string(), // hp = current health, drops as it bleeds
                    &tp, // tempo available THIS round - a horde swarms with its body count, not its Cadence
                    kind.trim(),
                ],
                colour,
                12.5,
            );
        }
        text(
            t,
            "M might  V vitality  G grit  C cadence  F finesse  hp current health  tp tempo/round (horde = bodies)   rk = rank (F vanguard / b rearguard / o outrider)",
            10.0,
            MUTED,
        );
    });
}

fn row_cells(p: &mut ChildSpawnerCommands, cells: &[&str], colour: Color, size: f32) {
    // fixed widths so columns line up without a real grid
    let widths = [132.0, 34.0, 26.0, 26.0, 26.0, 26.0, 26.0, 30.0, 30.0, 90.0];
    p.spawn(Node {
        flex_direction: FlexDirection::Row,
        ..default()
    })
    .with_children(|r| {
        for (k, c) in cells.iter().enumerate() {
            r.spawn(Node {
                width: Val::Px(*widths.get(k).unwrap_or(&40.0)),
                ..default()
            })
            .with_children(|cell| text(cell, *c, size, colour));
        }
    });
}

fn controls(p: &mut ChildSpawnerCommands) {
    p.spawn(Node {
        flex_direction: FlexDirection::Row,
        column_gap: Val::Px(8.0),
        ..default()
    })
    .with_children(|row| {
        button(row, Hit::Undo, "Back", PANEL);
        button(row, Hit::Reset, "Restart", PANEL);
        button(row, Hit::Next, "Next encounter", PANEL);
    });
}

fn button(p: &mut ChildSpawnerCommands, hit: Hit, label: &str, bg: Color) {
    p.spawn((
        Button,
        hit,
        Node {
            padding: UiRect::axes(Val::Px(12.0), Val::Px(7.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor::all(MUTED.with_alpha(0.4)),
    ))
    .with_children(|b| text(b, label, 13.0, INK));
}

/// How many trailing history lines the panel and the mirror both show.
const HISTORY_LINES: usize = 22;

/// Colour a history line by what it is: a declaration (who chose what), an effect (indented - damage, a move), a
/// death, a round marker, or the final outcome. So the player can read the story of the fight at a glance.
fn history_color(line: &str) -> Color {
    if line.starts_with("===") {
        if line.contains("Win") { GOOD } else { BAD }
    } else if line.starts_with("---") {
        MUTED // a round marker
    } else if line.ends_with("is down") {
        BAD // a body fell
    } else if line.starts_with('*') {
        FOE // a foe's declaration: "*The Wall: Clash Marksman"
    } else if line.starts_with("  ") {
        WARN // an effect of the round: damage, a slip, a fall-back
    } else {
        INK // a hero's declaration: "Raider: Clash Wall"
    }
}

fn log_panel(p: &mut ChildSpawnerCommands, f: &Fight) {
    p.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(8.0)),
            flex_grow: 1.0,
            overflow: Overflow::clip(),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(SUNK),
    ))
    .with_children(|panel| {
        text(panel, "history - what actually happened", 11.0, MUTED);
        for line in f
            .log
            .iter()
            .rev()
            .take(HISTORY_LINES)
            .collect::<Vec<_>>()
            .iter()
            .rev()
        {
            text(panel, (*line).clone(), 12.0, history_color(line));
        }
    });
}

/// One clickable choice row: its title and counts on the left, its verdict tag (and the border colour) on the
/// right. Shared by direct options, collapsed crossings, and crossing answers so they all read the same.
fn choice_button(
    p: &mut ChildSpawnerCommands,
    hit: Hit,
    title: String,
    counts: String,
    route: String,
    v: Option<Verdict>,
) {
    let border = v.map(verdict_color).unwrap_or(WARN); // amber while still evaluating
    let vtag = v
        .map(|x| format!("{x:?}"))
        .unwrap_or_else(|| "...".to_string());
    p.spawn((
        Button,
        hit,
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(9.0), Val::Px(6.0)),
            border_radius: BorderRadius::all(Val::Px(5.0)),
            border: UiRect::left(Val::Px(3.0)),
            ..default()
        },
        BackgroundColor(SUNK),
        BorderColor::all(border),
    ))
    .with_children(|row| {
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(1.0),
            ..default()
        })
        .with_children(|left| {
            text(left, title, 14.0, INK);
            text(left, counts, 11.0, MUTED);
            text(left, route, 11.0, GOOD); // the best route through this option, in the "good" colour
        });
        text(row, vtag, 12.0, border);
    });
}

/// The bare best-route cell `Nd/Nr/Nhp` (downed / rounds / hp lost), prefixed `<=` while the search is still a
/// provisional bound; `no win` when proven unwinnable, `...` while still computing.
fn route_cell(score: Option<Score>, done: bool) -> String {
    match score {
        Some(s) => {
            let le = if done { "" } else { "<=" };
            format!("{le}{}d/{}r/{}hp", s.downed, s.rounds, s.hp_lost)
        }
        None if done => "no win".to_string(),
        None => "...".to_string(),
    }
}

/// The best-route line for one option button - the [`route_cell`] with a "best " prefix (or the full "no winning
/// route" when the option is proven doomed).
fn fmt_route(score: Option<Score>, done: bool) -> String {
    match (score, done) {
        (None, true) => "no winning route".to_string(),
        _ => format!("best {}", route_cell(score, done)),
    }
}

/// The options, each a clickable button carrying its verdict and win/loss line counts.
fn options_panel(p: &mut ChildSpawnerCommands, f: &Fight) {
    if Combat::outcome(&f.state).is_some() {
        text(p, "your options", 16.0, INK);
        text(
            p,
            "the fight is over - Restart or Next encounter.",
            13.0,
            MUTED,
        );
        return;
    }
    // The active hero, once and prominently - every option below belongs to it, so it is not repeated per row.
    // (It is also marked with a > on its row in the unit table.)
    match f.state.deciding() {
        Some(i) => {
            let u = &f.state.board().units[i];
            text(p, format!("> {} is choosing an action", u.name), 17.0, GOOD);
        }
        None => text(p, "your options", 16.0, INK),
    }
    // Beat two of a crossing: the line caught you - narrate who, then offer the answers (Abort named after the
    // catcher). Nothing has been applied yet, so a "choose again" card backs out to beat one.
    if let Some(drill) = &f.drill {
        text(
            p,
            format!(
                "{} - {} catches you at the line.",
                drill.label, drill.catchers
            ),
            14.0,
            WARN,
        );
        text(p, "how do you answer?", 11.0, MUTED);
        for &(ans, opt) in &drill.answers {
            choice_button(
                p,
                Hit::Option(opt),
                answer_label(ans, &drill.catchers),
                opt_counts(f.opt_paths[opt]),
                fmt_route(f.opt_score[opt], f.opt_score_done[opt]),
                f.opt_verdict[opt],
            );
        }
        p.spawn((
            Button,
            Hit::Back,
            Node {
                padding: UiRect::axes(Val::Px(9.0), Val::Px(6.0)),
                border_radius: BorderRadius::all(Val::Px(5.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(MUTED.with_alpha(0.4)),
        ))
        .with_children(|b| text(b, "< choose a different action", 12.0, MUTED));
        return;
    }

    text(
        p,
        "each shows: solver verdict, then winning / losing lines through it (a tie is a loss)",
        11.0,
        MUTED,
    );

    // Beat one: the entries. A crossing is one card ("Raid X  >"); picking it opens beat two.
    for (k, entry) in f.entries.iter().enumerate() {
        match entry {
            Entry::Direct(opt) => choice_button(
                p,
                Hit::Option(*opt),
                describe(f.state.board(), &f.options[*opt]),
                opt_counts(f.opt_paths[*opt]),
                fmt_route(f.opt_score[*opt], f.opt_score_done[*opt]),
                f.opt_verdict[*opt],
            ),
            Entry::Crossing { label, answers, .. } => {
                let (rs, rd) = f.agg_route(answers);
                choice_button(
                    p,
                    Hit::Crossing(k),
                    format!("{label}  >"),
                    f.agg_counts(answers),
                    fmt_route(rs, rd),
                    f.agg_verdict(answers),
                )
            }
        }
    }
}

/// A compact count: exact under 10k, then k/M with one decimal, saturating at "max".
fn abbrev(n: u64) -> String {
    if n == u64::MAX {
        "max".into()
    } else if n < 10_000 {
        n.to_string()
    } else if n < 1_000_000 {
        format!("{:.1}k", n as f64 / 1e3)
    } else {
        format!("{:.1}M", n as f64 / 1e6)
    }
}
