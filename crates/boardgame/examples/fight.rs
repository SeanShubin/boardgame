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
use rules::combat::game::Score;
use rules::combat::regions::{Board, Rank};
use rules::combat::resolve::{Combatant, Side};
use rules::combat::step_game::{Phase, StepChoice, StepCombat, StepScorer, StepState, step_policy};
use rules::combat::steps::{StepScript, play_steps};
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
    // `auto` anywhere in the args = SELF-PLAY: the party is driven by the same deterministic greedy the foes run
    // ([`Instinct`]), so the whole fight resolves on startup and `fight-log.txt` holds a complete transcript -
    // the headless way to check the log against the round-sequence doc, no clicks needed.
    let args: Vec<String> = std::env::args()
        .filter(|a| a != "auto" && a != "--auto")
        .collect();
    let auto = std::env::args().any(|a| a == "auto" || a == "--auto");
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
        .insert_resource(Fight::new(idx, requested_kit, auto))
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
/// **One undo point** - the whole decision-point state, snapshotted so Back is a pointer move with **nothing
/// recomputed**: the solver/counter memos are kept across moves (their key is a pure function of the position, so
/// a restored position is already solved), and the scored options are captured here too, so undo restores
/// instantly without even a re-scan. `log_len` is the log height to unwind the history stack back to. The
/// step machine resolves MID-round, so the round-start narration inputs (`round_board`, `script`, `wave_mark`)
/// snapshot too.
struct UndoPoint {
    state: StepState,
    log_len: usize,
    options: Vec<StepChoice>,
    opt_verdict: Vec<Option<Verdict>>,
    opt_paths: Vec<Option<Paths>>,
    opt_score: Vec<Option<Score>>,
    opt_score_done: Vec<bool>,
    verdict: Option<Verdict>,
    round_board: Board,
    script: StepScript,
    wave_mark: Option<(usize, Phase)>,
}

#[derive(Resource)]
struct Fight {
    encounter: usize,
    /// The kit requested on the command line for a solo encounter (`None` = the keystone's counter). Kept so
    /// Restart / Next re-field the same solo kit.
    requested_kit: Option<String>,
    state: StepState,
    /// The legal options right now. Their scores are computed lazily, a slice per frame - never on the click.
    options: Vec<StepChoice>,
    /// The undo stack: one [`UndoPoint`] per decision the player has taken, most recent last. Back pops it.
    history: Vec<UndoPoint>,
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
    solver: Solver<StepCombat>,
    counter: PathCounter<StepCombat>,
    scorer: StepScorer,
    /// Round-robin cursors over the options, so each option's tally and each option's best-route search get a fair
    /// share of refinement each frame - no single hard one starves the others.
    refine: usize,
    refine_score: usize,
    /// Each unit's Vitality (its full health), snapshotted at the start so the table can show max **and** current
    /// health - the live `health` field only ever holds the current value, so the maximum has to be kept here.
    max_health: Vec<u32>,
    /// **Self-play**: the party is driven by [`step_policy`] (the same per-step greedy the foes run), so the
    /// fight resolves without clicks and `fight-log.txt` holds a complete transcript.
    auto: bool,
    /// The board as it stood at the START of the current round - the input `narrate` re-simulates from once the
    /// round resolves. (Steps resolve live, so the state's own board is already mid-round.)
    round_board: Board,
    /// The declarations accumulated this round, per step - the other narration input.
    script: StepScript,
    /// The last wave header logged, `(round, phase)` - so each wave's header prints exactly once.
    wave_mark: Option<(usize, Phase)>,
    log: Vec<String>,
    dirty: bool,
}

impl Fight {
    fn new(encounter: usize, requested_kit: Option<String>, auto: bool) -> Self {
        let state = build(encounter, requested_kit.as_deref());
        let round_board = state.board().clone();
        let mut f = Fight {
            encounter,
            state,
            requested_kit,
            auto,
            options: Vec::new(),
            history: Vec::new(),
            opt_verdict: Vec::new(),
            opt_paths: Vec::new(),
            opt_score: Vec::new(),
            opt_score_done: Vec::new(),
            verdict: None,
            solver: Solver::new(),
            counter: PathCounter::new(),
            scorer: StepScorer::new(Vec::new(), 0),
            refine: 0,
            refine_score: 0,
            max_health: Vec::new(),
            round_board,
            script: StepScript::default(),
            wave_mark: None,
            log: Vec::new(),
            dirty: true,
        };
        f.snapshot_max();
        f.rebuild_scorer();
        f.log_roster(); // list ranks at the top, before any forced declarations are logged
        f.reposition();
        f.sync_log();
        f
    }

    /// (Re)build the best-route scorer with the fight-start Vitality as its hp reference - called once per roster
    /// (the reference is fixed for the whole fight, so the scorer memo stays valid across moves).
    fn rebuild_scorer(&mut self) {
        self.scorer = StepScorer::new(self.max_health.clone(), 0);
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
        self.round_board = self.state.board().clone();
        self.script = StepScript::default();
        self.wave_mark = None;
        self.snapshot_max();
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
        // Advance the waves until a party body has a GENUINE decision. A foe (or, under `auto`, everyone) commits
        // its scripted per-step choice ([`step_policy`]) here and now - this is where creature declarations get
        // applied and, crucially, LOGGED, through the same `apply_choice` path a click takes. A single-option
        // wave entry auto-advances so the UI only ever presents a real choice.
        loop {
            if StepCombat::outcome(&self.state).is_some() {
                self.options = Vec::new();
                break;
            }
            let Some(i) = self.state.deciding() else {
                self.options = Vec::new();
                break;
            };
            let scripted = self.auto || self.state.board().units[i].side != Side::Party;
            if scripted {
                let c = step_policy(&self.state, i);
                self.apply_choice(&c);
                continue;
            }
            let opts = StepCombat::options(&self.state);
            if opts.len() == 1 {
                let c = opts[0].clone();
                self.apply_choice(&c);
            } else {
                self.options = opts;
                break;
            }
        }
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
        if StepCombat::outcome(&self.state).is_some() {
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
                let next = StepCombat::apply(&self.state, &self.options[i]);
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
                    let next = StepCombat::apply(&self.state, &self.options[i]);
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
                    let next = StepCombat::apply(&self.state, &self.options[i]);
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
        if i >= self.options.len() || StepCombat::outcome(&self.state).is_some() {
            return;
        }
        // Snapshot this decision point for Back BEFORE we mutate: a pointer into the (memoized) tree plus the log
        // height, so undo restores instantly with nothing recomputed. One entry per player decision - the forced
        // and foe declarations that `reposition` auto-advances are part of this same step, not their own.
        self.history.push(UndoPoint {
            state: self.state.clone(),
            log_len: self.log.len(),
            options: self.options.clone(),
            opt_verdict: self.opt_verdict.clone(),
            opt_paths: self.opt_paths.clone(),
            opt_score: self.opt_score.clone(),
            opt_score_done: self.opt_score_done.clone(),
            verdict: self.verdict,
            round_board: self.round_board.clone(),
            script: self.script.clone(),
            wave_mark: self.wave_mark,
        });
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
        let Some(step) = self.history.pop() else {
            return; // nothing to undo
        };
        self.state = step.state;
        self.log.truncate(step.log_len);
        self.options = step.options;
        self.opt_verdict = step.opt_verdict;
        self.opt_paths = step.opt_paths;
        self.opt_score = step.opt_score;
        self.opt_score_done = step.opt_score_done;
        self.verdict = step.verdict;
        self.round_board = step.round_board;
        self.script = step.script;
        self.wave_mark = step.wave_mark;
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
    fn apply_choice(&mut self, c: &StepChoice) {
        let round_before = self.state.round();
        let phase = self.state.phase();
        let acting = self.state.deciding();

        if let Some(idx) = acting {
            // A wave header the first time this round reaches this step's declarations - the round.step
            // coordinate every commit below belongs to.
            if self.wave_mark != Some((round_before, phase)) {
                self.wave_mark = Some((round_before, phase));
                let (k, name) = phase_coord(phase);
                self.log
                    .push(format!("[round {round_before} - step {k}/8: {name}]"));
            }
            // Mark a foe with '*', exactly as the unit table does, so hero and creature declarations read
            // apart. The mark is the ONLY thing that says a body was scripted.
            let b = self.state.board();
            let mark = if b.units[idx].side == Side::Party {
                ""
            } else {
                "*"
            };
            let name = &b.units[idx].name;
            self.log.push(format!(
                "      commit  {mark}{name} -> {}",
                describe(phase, b, c)
            ));
            // Accumulate the declaration into this round's script - the narration re-simulates from it when the
            // round resolves. (Passes and stays accumulate nothing.)
            match (phase, c) {
                (Phase::Inner, StepChoice::Strike(Some(t))) => self.script.inner.push((idx, *t)),
                (Phase::Withdraw, StepChoice::Move(true)) => self.script.withdraw.push(idx),
                (Phase::Early, StepChoice::Strike(Some(t))) => self.script.early.push((idx, *t)),
                (Phase::Cross, StepChoice::Move(true)) => self.script.cross.push(idx),
                (Phase::Volley, StepChoice::Strike(Some(t))) => self.script.volley.push((idx, *t)),
                (Phase::Raid, StepChoice::Strike(Some(t))) => self.script.raid.push((idx, *t)),
                (Phase::Late, StepChoice::Strike(Some(t))) => self.script.late.push((idx, *t)),
                (Phase::Advance, StepChoice::Strike(Some(t))) => {
                    self.script.advance.push((idx, *t))
                }
                _ => {}
            }
        }

        self.state = StepCombat::apply(&self.state, c);

        // A round resolves on exactly one apply - the one where the round counter advances. Narrate it by
        // re-running the deterministic resolution from the round-start board and this round's script (a
        // throwaway clone; identical to what just resolved live), so every strike, flip, move and death is
        // logged under the step it happened in.
        if self.state.round() != round_before {
            let events = narrate_steps(&self.round_board, &self.script);
            if events.is_empty() {
                self.log.push("  (no blood drawn)".into());
            } else {
                self.log.extend(events);
            }
            self.round_board = self.state.board().clone();
            self.script = StepScript::default();
            match StepCombat::outcome(&self.state) {
                Some(o) => self.log.push(format!("========== {o:?} ==========")),
                None => self.log.push(format!(
                    "================= round {} =================",
                    self.state.round()
                )),
            }
        }
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

/// A [`Phase`] to its **step coordinate** `(number, name)` - the wave headers' vocabulary, matched one-for-one
/// by the round-sequence doc.
fn phase_coord(p: Phase) -> (u8, &'static str) {
    match p {
        Phase::Inner => (1, "Inner"),
        Phase::Withdraw => (2, "Withdraw"),
        Phase::Early => (3, "Early Trade"),
        Phase::Cross => (4, "Crossing"),
        Phase::Volley => (5, "Volley"),
        Phase::Raid => (6, "Raid"),
        Phase::Late => (7, "Late Trade"),
        Phase::Advance => (8, "Advance"),
    }
}

/// A `SubPhaseLog` phase string (the step resolvers' labels) to the same step coordinate - so the narration and
/// the wave headers speak one language.
fn step_coord(phase: &'static str) -> (u8, &'static str) {
    match phase {
        "Step 1: Inner" => (1, "Inner"),
        "Step 2: Withdraw" => (2, "Withdraw"),
        "Step 3: Early Trade" => (3, "Early Trade"),
        "Step 4: Crossing" => (4, "Crossing"),
        "Step 5: Volley" => (5, "Volley"),
        "Step 6: Raid" => (6, "Raid"),
        "Step 7: Late Trade" => (7, "Late Trade"),
        "Step 8: Advance" => (8, "Advance"),
        other => (0, other),
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
fn narrate_steps(before: &Board, script: &StepScript) -> Vec<String> {
    let mut clone = before.clone();
    let transcript = play_steps(&mut clone, script);

    let rank_word = |r: Rank| match r {
        Rank::Vanguard => "a Vanguard",
        Rank::Rearguard => "a Rearguard",
        Rank::Outrider => "an Outrider",
    };

    let mut out = Vec::new();
    let mut prev_hp: Vec<u32> = before.units.iter().map(|u| u.health).collect();
    let mut prev_tp: Vec<u32> = before.units.iter().map(|u| u.cadence).collect(); // Reset stands tempo up to Cadence
    let mut prev_rk: Vec<Rank> = before.ranks.clone();
    let mut prev_rg: Vec<u8> = before.regions.clone();
    for log in &transcript {
        // Each event tagged with its exchange step; rendered under the step's header with a step column.
        let mut lines: Vec<(&'static str, String)> = Vec::new();

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
                        // The dodge FLOOR the reach had to clear: the TARGET's utmost dodge - its whole tempo x its
                        // Finesse (no body multiplier, even for a horde). `reach_cards` sizes the bid to meet this,
                        // and the reacher wins ties - so a connecting strike shows BOTH compared numbers (the reach,
                        // and the dodge it beat), not just its own reach. This is why the tempo cannot be smaller:
                        // below the floor the target simply slips the blow, so only tempo ABOVE it can pour.
                        let tf = before.units[t].finesse.max(1);
                        let tt = prev_tp[t];
                        let against = if tt == 0 {
                            format!(" ({tn} has no tempo to dodge)")
                        } else {
                            format!(
                                " - clears {tn}'s top dodge {} ({tt} tempo x F{tf}), reacher wins ties",
                                tt * tf
                            )
                        };
                        let pour = total.saturating_sub(rt);
                        let poured = if pour > 0 {
                            format!(", then pours {pour} more tempo")
                        } else {
                            String::new()
                        };
                        format!(
                            "flips {rt} tempo at {fclause} = {} reach{against}{poured}, ",
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

        // --- Movements this step owns: the crossings (the step-4 log) and the withdrawals (the step-2 log).
        // Each step is its own transcript entry now, so its moves print in its OWN section as ordinary `move`
        // lines - no borrowed sub-phase headers.
        for &i in &log.through {
            lines.push((
                "move",
                format!(
                    "{}: walks into their line, now {}",
                    before.units[i].name,
                    rank_word(log.ranks[i])
                ),
            ));
        }
        for &i in &log.withdrew {
            lines.push((
                "move",
                format!(
                    "{}: withdraws from the enemy ranks, rejoining its line as {}",
                    before.units[i].name,
                    rank_word(log.ranks[i])
                ),
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
            if log.through.contains(&i) || log.withdrew.contains(&i) {
                continue; // its move was narrated as a crossing / a withdrawal
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
        if !lines.is_empty() {
            // The coordinate: this step's own header, then the events in resolution order, each in its step
            // column - so any line reads as round . step K/8 . event-kind.
            let (k, name) = step_coord(log.phase);
            out.push(format!("  [step {k}/8] {name}"));
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
    }
    out
}

/// The tempo a body has to spend in a round: its Cadence pool (`refresh_round`), hordes included - a horde's
/// size shows up as a body-count volley of damage and a body-count reach, not extra tempo.
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
fn build(encounter: usize, requested_kit: Option<&str>) -> StepState {
    let e = &catalog::ENCOUNTERS[encounter % catalog::ENCOUNTERS.len()];
    let mut units: Vec<Combatant> = if e.party {
        catalog::ROSTER.iter().copied().map(kit).collect()
    } else {
        vec![kit(solo_kit(encounter, requested_kit).1)]
    };
    units.extend(encounter_beasts(e)); // numbered when duplicated, so two Walls read apart
    StepState::new(units)
}

// ---- choice / board formatting -------------------------------------------------------------------------

/// A choice label, per the current step. The active body is shown once above the options and marked on the
/// table, so it is never repeated per action. Targets are named WITH current health, so two same-named bodies in
/// different states read as the distinct choices they are.
fn describe(phase: Phase, b: &Board, c: &StepChoice) -> String {
    let who = |t: usize| {
        let u = &b.units[t];
        let kind = if u.horde { "bodies" } else { "hp" };
        format!("{} ({} {kind})", u.name, u.health)
    };
    match (phase, c) {
        (Phase::Inner, StepChoice::Strike(Some(t))) => format!("Melee {}", who(*t)),
        (Phase::Early, StepChoice::Strike(Some(t))) => format!("Strike {} (early)", who(*t)),
        (Phase::Volley, StepChoice::Strike(Some(t))) => format!("Volley the crossing {}", who(*t)),
        (Phase::Raid, StepChoice::Strike(Some(t))) => format!("Raid {}", who(*t)),
        (Phase::Late, StepChoice::Strike(Some(t))) => format!("Strike {}", who(*t)),
        (Phase::Advance, StepChoice::Strike(Some(t))) => {
            format!("Advance on the exposed {}", who(*t))
        }
        (_, StepChoice::Strike(Some(t))) => format!("Strike {}", who(*t)),
        (_, StepChoice::Strike(None)) => "Hold (pass this step)".to_string(),
        (Phase::Withdraw, StepChoice::Move(true)) => "Withdraw to your own line".to_string(),
        (Phase::Withdraw, StepChoice::Move(false)) => "Stay loose in their ranks".to_string(),
        (Phase::Cross, StepChoice::Move(true)) => "Cross into their line".to_string(),
        (Phase::Cross, StepChoice::Move(false)) => "Hold the line (do not cross)".to_string(),
        (_, StepChoice::Move(go)) => if *go { "Go" } else { "Stay" }.to_string(),
    }
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

// ---- input ---------------------------------------------------------------------------------------------

#[derive(Component, Clone, Copy)]
enum Hit {
    /// Apply `options[usize]` directly.
    Option(usize),
    /// Step back one decision (pop the undo stack).
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
    match (StepCombat::outcome(&f.state), f.verdict) {
        (Some(o), _) => writeln!(s, "*** {o:?} ***").ok(),
        (None, Some(v)) => writeln!(s, "round {}   position: {v:?}", f.state.round()).ok(),
        (None, None) => writeln!(s, "round {}   position: computing...", f.state.round()).ok(),
    };
    if let Some(i) = f.state.deciding() {
        writeln!(s, "acting: {}", b.units[i].name).ok();
    }
    if StepCombat::outcome(&f.state).is_none() {
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

    // The options - same order and content as the buttons, labelled by the current step.
    writeln!(s, "\nOPTIONS").ok();
    if StepCombat::outcome(&f.state).is_none() {
        let (rs, rd) = f.best_route();
        writeln!(
            s,
            "best route from here: {}  (downed/rounds/hp, minimized in that priority)",
            route_cell(rs, rd)
        )
        .ok();
    }
    let vtag = |v: Option<Verdict>| v.map(|x| format!("{x:?}")).unwrap_or_else(|| "...".into());
    if StepCombat::outcome(&f.state).is_some() {
        writeln!(s, "the fight is over.").ok();
    } else {
        let (k, name) = phase_coord(f.state.phase());
        writeln!(s, "step {k}/8 - {name}").ok();
        for (n, c) in f.options.iter().enumerate() {
            writeln!(
                s,
                "[{n}] {:<28} {:<12} {:<22} {}",
                describe(f.state.phase(), b, c),
                vtag(f.opt_verdict[n]),
                opt_counts(f.opt_paths[n]),
                fmt_route(f.opt_score[n], f.opt_score_done[n])
            )
            .ok();
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
        let status = match (StepCombat::outcome(&f.state), f.verdict) {
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
        if StepCombat::outcome(&f.state).is_none() {
            let (rs, rd) = f.best_route();
            text(
                h,
                format!("best route: {}  (downed/rounds/hp)", route_cell(rs, rd)),
                12.0,
                GOOD,
            );
        }
        // What you are waiting on: a live progress line, so a busy UI is never a silent one.
        if StepCombat::outcome(&f.state).is_none() {
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
    if StepCombat::outcome(&f.state).is_some() {
        text(p, "your options", 16.0, INK);
        text(
            p,
            "the fight is over - Restart or Next encounter.",
            13.0,
            MUTED,
        );
        return;
    }
    // The active hero and the STEP it is deciding, once and prominently - every option below belongs to both,
    // so neither is repeated per row. (The hero is also marked with a > on its row in the unit table.)
    let (k, name) = phase_coord(f.state.phase());
    match f.state.deciding() {
        Some(i) => {
            let u = &f.state.board().units[i];
            text(
                p,
                format!("> {} decides - step {k}/8: {name}", u.name),
                17.0,
                GOOD,
            );
        }
        None => text(p, "your options", 16.0, INK),
    }
    text(
        p,
        "each shows: solver verdict, then winning / losing lines through it (a tie is a loss)",
        11.0,
        MUTED,
    );
    for (n, c) in f.options.iter().enumerate() {
        choice_button(
            p,
            Hit::Option(n),
            describe(f.state.phase(), f.state.board(), c),
            opt_counts(f.opt_paths[n]),
            fmt_route(f.opt_score[n], f.opt_score_done[n]),
            f.opt_verdict[n],
        );
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
