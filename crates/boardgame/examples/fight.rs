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
use rules::combat::regions::{Act, Answer, Board, Post, foe_acts};
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

/// The MOST work any one frame may do, so a frame never stalls and the window stays responsive - you can drag
/// it, close it, and see progress while the solver thinks. The verdict search restarts from the root each frame
/// reusing its memo, so capping the per-frame grant just spreads a hard question over more frames.
const FRAME_NODES: u64 = 20_000;
/// The win/loss tally is a single bounded pass per option; an incomplete result is an honest ">=" lower bound,
/// good enough to compare routes. (The shared memo makes later options' tallies more complete for free.)
const COUNT_BUDGET: u64 = 40_000;

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
    /// The current per-walk grant, doubled each frame a verdict stays uncertain (capped at [`FRAME_NODES`]).
    grant: u64,
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
            grant: FRAME_NODES,
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
        self.log.clear();
        self.reposition();
    }

    /// **The position changed** - list the new options and arm the lazy compute. Nothing is searched here, so a
    /// click returns instantly and the board is drawn at once; the scores fill in over the next frames.
    fn reposition(&mut self) {
        // Auto-advance a forced single option so the UI only ever shows real decisions.
        loop {
            let opts = Combat::options(&self.state);
            if opts.len() == 1 && Combat::outcome(&self.state).is_none() {
                self.state = Combat::apply(&self.state, &opts[0]);
            } else {
                self.options = opts;
                break;
            }
        }
        self.opt_verdict = vec![None; self.options.len()];
        self.opt_paths = vec![None; self.options.len()];
        self.verdict = None;
        self.solver = Solver::new();
        self.counter = PathCounter::new();
        self.grant = FRAME_NODES;
        self.dirty = true;
    }

    /// **One frame's worth of scoring.** Settle the position's verdict first, then each option's verdict, then
    /// each option's win/loss tally - one small step, capped at [`FRAME_NODES`], so the frame never stalls. The
    /// shared memo means each step is cheaper than the last. Returns whether anything changed (redraw if so).
    fn grind(&mut self) -> bool {
        if Combat::outcome(&self.state).is_some() {
            return false;
        }
        // 1. the position's own verdict
        if self.verdict.is_none() {
            self.solver.grant(self.grant);
            match self.solver.verdict(&self.state) {
                Verdict::Evaluating => {
                    self.grant = (self.grant * 2).min(FRAME_NODES.saturating_mul(8))
                }
                v => {
                    self.verdict = Some(v);
                    self.grant = FRAME_NODES;
                }
            }
            return true;
        }
        // 2. the first option still missing a verdict
        for i in 0..self.options.len() {
            if self.opt_verdict[i].is_none() {
                let next = Combat::apply(&self.state, &self.options[i]);
                self.solver.grant(self.grant);
                match self.solver.verdict(&next) {
                    Verdict::Evaluating => {
                        self.grant = (self.grant * 2).min(FRAME_NODES.saturating_mul(8))
                    }
                    v => {
                        self.opt_verdict[i] = Some(v);
                        self.grant = FRAME_NODES;
                    }
                }
                return true;
            }
        }
        // 3. the first option still missing a tally (a single bounded pass; a partial ">=" is fine)
        for i in 0..self.options.len() {
            if self.opt_paths[i].is_none() {
                let next = Combat::apply(&self.state, &self.options[i]);
                self.counter.grant(COUNT_BUDGET);
                self.opt_paths[i] = Some(self.counter.count(&next));
                return true;
            }
        }
        false
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
        // Snapshot the board so we can narrate what the resolution actually DID, not just what was declared.
        let before = self.state.board().clone();
        let who = self
            .state
            .deciding()
            .map(|k| before.units[k].name.clone())
            .unwrap_or_default();
        let round_before = self.state.round();
        self.log.push(format!("{who}: {}", describe(&before, &c)));

        self.state = Combat::apply(&self.state, &c);

        // Combat resolves in exactly ONE apply per round - the round-closing declaration, where the foe folds in
        // and the round counter advances. A mid-round declaration by a non-last hero records an intent and changes
        // nothing, so it must stay silent. (The setup->round-1 transition also advances the counter but draws no
        // blood; `round_before >= 1` excludes it.)
        let resolved = round_before >= 1 && self.state.round() != round_before;
        if resolved {
            // The foes never appear in the options - they are scripted inside apply - so their attacks are
            // invisible unless we surface them here. Log what each foe went for, so a hero going down has a named
            // attacker above it.
            for line in foe_declarations(&before) {
                self.log.push(line);
            }
            for line in narrate(&before, self.state.board()) {
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
        self.reposition();
    }
}

/// **What each foe declared this round** - one line per foe that did something, marked with `*` like the table.
/// The foes are scripted ([`foe_acts`]) rather than chosen, so without this the player sees a hero fall with no
/// idea who felled it. Computed on the pre-resolution board, which is exactly what `apply` scripted them from.
fn foe_declarations(before: &Board) -> Vec<String> {
    foe_acts(before)
        .into_iter()
        .enumerate()
        .filter_map(|(i, a)| {
            let a = a?;
            if matches!(a, Act::Hold) {
                return None; // a foe that did nothing is not worth a line
            }
            Some(format!(
                "*{}: {}",
                before.units[i].name,
                act_label(before, &a)
            ))
        })
        .collect()
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

/// A choice label. The active hero is shown once above the options and marked on the table, so it is never
/// repeated per action - a placement reads just "stand at region A (front)".
fn describe(b: &Board, c: &Choice) -> String {
    match c {
        Choice::Place { region, post } => format!(
            "stand at region {} ({})",
            region_letter(*region),
            if *post == Post::Front {
                "front"
            } else {
                "back"
            }
        ),
        Choice::Act(a) => act_label(b, a),
    }
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
        let verb = if f.state.placing() {
            "placing"
        } else {
            "acting"
        };
        writeln!(s, "{verb}: {}", b.units[i].name).ok();
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
        // Say WHO is deciding, so "place region A" is never ambiguous about which hero.
        if let Some(i) = f.state.deciding() {
            let who = &f.state.board().units[i].name;
            let verb = if f.state.placing() {
                "placing"
            } else {
                "acting"
            };
            text(h, format!("{verb}: {who}"), 13.0, GOOD);
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
            let verb = if f.state.placing() {
                "is placing"
            } else {
                "is choosing an action"
            };
            text(p, format!("> {} {}", u.name, verb), 17.0, GOOD);
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
