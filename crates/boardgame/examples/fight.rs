//! **Fight** - a clickable UI to play a combat by hand, driving the pure [`rules::combat`] game through the
//! generic interface. Pick an option; the board updates; repeat to a verdict.
//!
//! Every option button shows, for the route it opens, **how many complete lines end in a win and how many in a
//! loss** (a tie counts as a loss), plus the solver's verdict. That is the "how forgiving is this move" number:
//! a route with many winning lines and few losing ones is safe; one Doomed line is a trap the button says so.
//!
//! Space-efficient by intent: a compact unit table with stat abbreviations (M/V/G/C/F = Might / Vitality /
//! Grit / Cadence / Finesse), region letters, and F/b posts.
//!
//! Run: `cargo run --release -p boardgame --example fight -- [encounter#]`

use bevy::prelude::*;

use deckbound_board::units::{beast, kit};
use deckbound_content::catalog::{self, Encounter};
use rules::combat::game::{Choice, Combat, State};
use rules::combat::regions::{Act, Answer, Board, Post};
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
    let idx: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or_else(|| {
            catalog::ENCOUNTERS
                .iter()
                .position(|e| e.party)
                .unwrap_or(0)
        });

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
        .insert_resource(Fight::new(idx))
        .add_systems(Startup, (camera, rebuild).chain())
        .add_systems(Update, (on_click, grind, rebuild.run_if(is_dirty)).chain())
        .run();
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

// ---- state ---------------------------------------------------------------------------------------------

#[derive(Resource)]
struct Fight {
    encounter: usize,
    state: State,
    /// The legal options right now. Their scores are computed lazily, a slice per frame - never on the click.
    options: Vec<Choice>,
    /// Each option's verdict and win/loss tally; `None` means "still computing" (shown as `...`).
    opt_verdict: Vec<Option<Verdict>>,
    opt_paths: Vec<Option<Paths>>,
    /// The position's own verdict; `None` while thinking.
    verdict: Option<Verdict>,
    /// A solver and counter kept ACROSS frames, so their memo survives and the work converges instead of
    /// restarting. Rebuilt only when the position changes.
    solver: Solver<Combat>,
    counter: PathCounter<Combat>,
    /// A round-robin cursor over the options, so each option's win/loss tally gets a fair share of refinement
    /// each frame - no single hard tally starves the others.
    refine: usize,
    /// Each unit's Vitality (its full health), snapshotted at the start so the table can show max **and** current
    /// health - the live `health` field only ever holds the current value, so the maximum has to be kept here.
    max_health: Vec<u32>,
    log: Vec<String>,
    dirty: bool,
}

impl Fight {
    fn new(encounter: usize) -> Self {
        let mut f = Fight {
            encounter,
            state: build(encounter),
            options: Vec::new(),
            opt_verdict: Vec::new(),
            opt_paths: Vec::new(),
            verdict: None,
            solver: Solver::new(),
            counter: PathCounter::new(),
            refine: 0,
            max_health: Vec::new(),
            log: Vec::new(),
            dirty: true,
        };
        f.snapshot_max();
        f.reposition();
        f
    }

    /// Record every unit's full health for this encounter, so the table can show max vs current.
    fn snapshot_max(&mut self) {
        self.max_health = self.state.board().units.iter().map(|u| u.health).collect();
    }

    fn enc(&self) -> &'static Encounter {
        &catalog::ENCOUNTERS[self.encounter % catalog::ENCOUNTERS.len()]
    }

    fn reset(&mut self) {
        self.state = build(self.encounter);
        self.snapshot_max();
        // A new roster (Restart or Next encounter) is the ONE thing that invalidates the memos: the state key
        // omits unit stats, so a key from one encounter must never answer for another. Start fresh here - the one
        // place the units change. Ordinary moves keep the memo (see `reposition`).
        self.solver = Solver::new();
        self.counter = PathCounter::new();
        self.log.clear();
        self.reposition();
    }

    /// **The position changed** - list the new options and arm the lazy compute. Nothing is searched here, so a
    /// click returns instantly and the board is drawn at once; the scores fill in over the next frames.
    fn reposition(&mut self) {
        // Auto-advance a forced single option so the UI only ever shows real decisions. A FOE reaches the
        // declaration cursor exactly here - its turn has one legal act - so this is where creature declarations
        // (and any forced hero move) get applied and, crucially, LOGGED, through the same path a click takes.
        loop {
            let opts = Combat::options(&self.state);
            if opts.len() == 1 && Combat::outcome(&self.state).is_none() {
                let c = opts[0].clone();
                self.apply_choice(&c);
            } else {
                self.options = opts;
                break;
            }
        }
        self.opt_verdict = vec![None; self.options.len()];
        self.opt_paths = vec![None; self.options.len()];
        self.verdict = None;
        // KEEP the solver/counter memos across a move - do NOT re-new them here. The game is deterministic and the
        // memo key is a pure function of the position, so any subtree already explored while scoring the parent's
        // options is reused now: the child we stepped into was walked to produce the win/loss tally we just showed
        // on its button, so its verdict and counts are already in the memo and land instantly. Only a *roster*
        // change invalidates them, and that is handled in `reset`. So stepping through a fight re-computes only the
        // genuinely new frontier, never a node already seen.
        self.refine = 0;
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

        // 3. the option tallies, refined ROUND-ROBIN so no one hard tally starves the rest. The first still-
        //    incomplete option this frame gets whatever frame budget the verdicts left - a fuller pass than a
        //    verdict step, so counts fill decently - and the cursor moves on so the next frame refines the next
        //    option. Each pass keeps the latest count (a partial is an honest ">="); the shared memo makes it more
        //    complete each visit.
        let n = self.options.len();
        let mut looked = 0;
        while spent < FRAME_BUDGET && looked < n {
            let i = self.refine % n;
            self.refine = (self.refine + 1) % n;
            looked += 1;
            if !self.opt_paths[i].is_some_and(|p| p.complete) {
                let next = Combat::apply(&self.state, &self.options[i]);
                self.counter.grant(FRAME_BUDGET - spent);
                self.opt_paths[i] = Some(self.counter.count(&next));
                spent = FRAME_BUDGET; // that one pass used the rest of this frame's budget
                changed = true;
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
        let c = self.options[i].clone();
        self.apply_choice(&c);
        self.reposition();
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
            for line in crossings(before, &acts) {
                self.log.push(line);
            }
            for line in narrate(before, self.state.board()) {
                self.log.push(line);
            }
        }
        if self.state.round() != round_before {
            match Combat::outcome(&self.state) {
                Some(o) => self.log.push(format!("=== {o:?} ===")),
                None => self
                    .log
                    .push(format!("--- round {} ---", self.state.round())),
            }
        }
    }
}

/// The region an act carries its actor into, if it moves at all - the renderer's copy of the private
/// `Act::destination`, so the UI can tell a slipper from a stander without reaching into the rules.
fn act_destination(before: &Board, i: usize, a: &Act) -> Option<u8> {
    let here = before.regions[i];
    match a {
        Act::Raid(t, _) => (before.regions[*t] != here).then(|| before.regions[*t]),
        Act::Slip(r, _) => (*r != here).then_some(*r),
        _ => None,
    }
}

fn act_answer(a: &Act) -> Option<Answer> {
    match a {
        Act::Raid(_, x) | Act::Slip(_, x) => Some(*x),
        _ => None,
    }
}

/// **The slip contests** - who reached for each body that tried to cross. A slipper is caught by every enemy
/// standing in the region it LEAVES and the region it ENTERS (it is outside its own screen the moment it moves) -
/// which is where a pushed slipper's damage comes from, damage no *declared* attack would explain. Reconstructed
/// from the same acts the round resolved with, so it names the exact bodies that reached for it.
fn crossings(before: &Board, acts: &[Act]) -> Vec<String> {
    // A body mid-crossing cannot also hold a line, so it is not a catcher.
    let transit: Vec<bool> = (0..before.units.len())
        .map(|j| !before.units[j].fallen && act_destination(before, j, &acts[j]).is_some())
        .collect();

    let mut out = Vec::new();
    for i in 0..before.units.len() {
        if before.units[i].fallen {
            continue;
        }
        let Some(dest) = act_destination(before, i, &acts[i]) else {
            continue;
        };
        let foe_side = if before.units[i].side == Side::Party {
            Side::Foe
        } else {
            Side::Party
        };
        // Every enemy in the region left and the region entered reaches for the slipper.
        let mut names: Vec<String> = Vec::new();
        for region in [before.regions[i], dest] {
            for j in before.in_region(region) {
                if before.units[j].side == foe_side && !transit[j] {
                    names.push(before.units[j].name.clone());
                }
            }
        }
        if names.is_empty() {
            continue; // an unopposed crossing - nobody to catch it
        }
        let verb = match act_answer(&acts[i]) {
            Some(Answer::Evade) => "slips past",
            Some(Answer::Push) => "pushes past",
            Some(Answer::Abort) => "turns and fights",
            None => "crosses past",
        };
        out.push(format!(
            "  {} {} {} (crossing to region {})",
            before.units[i].name,
            verb,
            join_counts(&names),
            region_letter(dest)
        ));
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

/// **What the last round actually did to the board**, read from a before/after diff - the events the player
/// never sees otherwise, because the whole round (foes included) resolves inside one `apply`. One line per thing
/// that changed: a body that moved, took damage, or fell. Indented so the history can colour them apart from the
/// declarations that caused them.
fn narrate(before: &Board, after: &Board) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..after.units.len() {
        let (bu, au) = (&before.units[i], &after.units[i]);
        let name = &au.name;
        // Moved: a slip landed it in a new region, or its post changed under it (a charge-in, a promotion).
        if !au.fallen && before.regions[i] != after.regions[i] {
            out.push(format!(
                "  {name} slips to region {}",
                region_letter(after.regions[i])
            ));
        } else if !au.fallen && before.posts[i] != after.posts[i] {
            let where_to = if after.posts[i] == Post::Front {
                "charges to the front"
            } else {
                "falls back"
            };
            out.push(format!("  {name} {where_to}"));
        }
        // Bled: a horde loses whole bodies; anyone else loses hp.
        if au.health < bu.health {
            let d = bu.health - au.health;
            if au.horde {
                out.push(format!(
                    "  {name} loses {d} ({} -> {} bodies)",
                    bu.health, au.health
                ));
            } else {
                out.push(format!(
                    "  {name} takes {d} ({} -> {} hp)",
                    bu.health, au.health
                ));
            }
        }
        // Fell.
        if au.fallen && !bu.fallen {
            out.push(format!("  {name} is down"));
        }
    }
    if out.is_empty() {
        out.push("  (no blood drawn)".into());
    }
    out
}

/// The tempo a body actually has to spend in a round: its Cadence pool - except a **horde**, which swarms with one
/// tempo per living body (`refresh_round`). This is why a Swarm's `C` stat reads low but it still floods the round.
fn round_tempo(u: &Combatant) -> u32 {
    if u.horde { u.health.max(1) } else { u.cadence }
}

fn build(encounter: usize) -> State {
    let e = &catalog::ENCOUNTERS[encounter % catalog::ENCOUNTERS.len()];
    let mut units: Vec<Combatant> = catalog::ROSTER.iter().copied().map(kit).collect();
    for (c, q) in catalog::encounter_foes(e) {
        for _ in 0..q {
            units.push(beast(c));
        }
    }
    State::new(units)
}

// ---- choice / board formatting -------------------------------------------------------------------------

fn region_letter(r: u8) -> char {
    (b'A' + r) as char
}

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
    match a {
        Act::Clash(t) => format!("Clash {}", b.units[*t].name),
        Act::Raid(t, x) => format!("Raid {} / {}", b.units[*t].name, ans(x)),
        Act::Melee(t) => format!("Melee {}", b.units[*t].name),
        Act::Slip(r, x) => format!("Slip -> {} / {}", region_letter(*r), ans(x)),
        Act::Hold => "Hold".into(),
    }
}

// ---- input ---------------------------------------------------------------------------------------------

#[derive(Component, Clone, Copy)]
enum Hit {
    Option(usize),
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
    let cols = ["unit", "rg", "M", "V", "G", "C", "F", "hp", "tp", "kind"];
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
        let place = format!(
            "{}{}",
            region_letter(b.regions[i]),
            if b.posts[i] == Post::Front { "F" } else { "b" }
        );
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
        "(M might  V vitality  G grit  C cadence  F finesse  hp current health  tp tempo/round, horde=bodies;  rg = region+post, F front / b back)"
    )
    .ok();

    // The options - same order and content as the buttons.
    writeln!(s, "\nOPTIONS").ok();
    if Combat::outcome(&f.state).is_some() {
        writeln!(s, "the fight is over.").ok();
    } else {
        for i in 0..f.options.len() {
            let v = f.opt_verdict[i]
                .map(|x| format!("{x:?}"))
                .unwrap_or_else(|| "...".into());
            let counts = match f.opt_paths[i] {
                Some(p) => {
                    let ge = if p.complete { "" } else { ">=" };
                    format!("{ge}{} win / {ge}{} lose", abbrev(p.wins), abbrev(p.losses))
                }
                None => "counting lines...".into(),
            };
            writeln!(
                s,
                "[{i}] {:<28} {:<12} {counts}",
                describe(b, &f.options[i]),
                v
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
            &["unit", "rg", "M", "V", "G", "C", "F", "hp", "tp", "kind"],
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
            let place = format!("{}{}", region_letter(b.regions[i]), if b.posts[i] == Post::Front { "F" } else { "b" });
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
            "M might  V vitality  G grit  C cadence  F finesse  hp current health  tp tempo/round (horde = bodies)   rg = region+post",
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
    text(
        p,
        "each shows: solver verdict, then winning / losing lines through it (a tie is a loss)",
        11.0,
        MUTED,
    );

    for i in 0..f.options.len() {
        let c = &f.options[i];
        let v = f.opt_verdict[i];
        let counts = match f.opt_paths[i] {
            Some(paths) => {
                let ge = if paths.complete { "" } else { ">=" };
                format!(
                    "{ge}{} win / {ge}{} lose",
                    abbrev(paths.wins),
                    abbrev(paths.losses)
                )
            }
            None => "counting lines...".to_string(),
        };
        let border = v.map(verdict_color).unwrap_or(WARN); // amber while still evaluating
        let vtag = v
            .map(|x| format!("{x:?}"))
            .unwrap_or_else(|| "...".to_string());
        p.spawn((
            Button,
            Hit::Option(i),
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
                text(left, describe(f.state.board(), c), 14.0, INK);
                text(left, counts, 11.0, MUTED);
            });
            text(row, vtag, 12.0, border);
        });
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
