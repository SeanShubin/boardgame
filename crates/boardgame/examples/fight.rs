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

use deckbound_content::catalog::{self, Creature, Encounter};
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

/// How many nodes the win/loss counter may spend per option, per rebuild. A fight's tree is large, so this is a
/// cap: an incomplete tally is shown with a `>=` rather than hangs the UI.
const COUNT_BUDGET: u64 = 60_000;

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
        .add_systems(Update, (on_click, rebuild.run_if(is_dirty)).chain())
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
    /// Per current option: its verdict and win/loss path tally. Computed on rebuild.
    scored: Vec<(Choice, Verdict, Paths)>,
    verdict: Verdict,
    log: Vec<String>,
    dirty: bool,
}

impl Fight {
    fn new(encounter: usize) -> Self {
        let mut f = Fight {
            encounter,
            state: build(encounter),
            scored: Vec::new(),
            verdict: Verdict::Evaluating,
            log: Vec::new(),
            dirty: true,
        };
        f.rescore();
        f
    }

    fn enc(&self) -> &'static Encounter {
        &catalog::ENCOUNTERS[self.encounter % catalog::ENCOUNTERS.len()]
    }

    fn reset(&mut self) {
        self.state = build(self.encounter);
        self.log.clear();
        self.rescore();
    }

    /// Ask the solver for this position's verdict, then score every option: its own verdict, and how many lines
    /// through it win vs lose. One `Solver` and one `PathCounter` are reused across the options so the shared
    /// tree is walked once.
    fn rescore(&mut self) {
        let opts = Combat::options(&self.state);
        // Auto-advance a forced single option so the UI only ever shows real decisions.
        if opts.len() == 1 {
            self.state = Combat::apply(&self.state, &opts[0]);
            if Combat::outcome(&self.state).is_none() {
                return self.rescore();
            }
        }
        // ONE solver and ONE counter across the whole rescore: the options lead to heavily overlapping subtrees,
        // so a shared memo settles the second option almost for free. (A fresh solver per option re-walked the
        // shared tree every time - that was the tens-of-seconds startup on a party encounter.)
        let mut solver = Solver::<Combat>::new();
        let mut counter = PathCounter::<Combat>::new();
        self.verdict = settle_with(&mut solver, &self.state);
        self.scored = opts
            .into_iter()
            .map(|c| {
                let next = Combat::apply(&self.state, &c);
                let v = settle_with(&mut solver, &next);
                counter.grant(COUNT_BUDGET);
                let p = counter.count(&next);
                (c, v, p)
            })
            .collect();
        self.dirty = true;
    }

    fn choose(&mut self, i: usize) {
        if i >= self.scored.len() || Combat::outcome(&self.state).is_some() {
            return;
        }
        let (c, _, _) = self.scored[i].clone();
        self.log.push(describe(self.state.board(), &c));
        self.state = Combat::apply(&self.state, &c);
        self.rescore();
    }
}

/// Grind a SHARED solver to a certain verdict (escalating grant) - reuses its memo across calls.
fn settle_with(o: &mut Solver<Combat>, s: &State) -> Verdict {
    let mut grant = 1u64 << 12;
    loop {
        o.grant(grant);
        let v = o.verdict(s);
        if v != Verdict::Evaluating {
            return v;
        }
        grant = grant.saturating_mul(2);
    }
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

fn kit(spec: (&'static str, [u8; 5], &'static str)) -> Combatant {
    let (name, stats, ability) = spec;
    let (melee, ranged) = catalog::ability_reach(ability);
    let (_r, aoe) = catalog::ability_shape(ability);
    Combatant::from_stats(name, Side::Party, stats, 0, melee, ranged).with_aoe(aoe)
}

fn beast(c: &Creature) -> Combatant {
    Combatant::from_stats(c.name, Side::Foe, c.stats, 0, c.melee, c.ranged)
        .with_aoe(c.aoe)
        .as_horde(c.horde)
}

// ---- choice / board formatting -------------------------------------------------------------------------

fn region_letter(r: u8) -> char {
    (b'A' + r) as char
}

fn describe(b: &Board, c: &Choice) -> String {
    match c {
        Choice::Place { region, post } => format!(
            "{} -> region {} ({})",
            "place",
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
        let status = match Combat::outcome(&f.state) {
            Some(o) => (format!("*** {o:?} ***"), verdict_color(f.verdict)),
            None => (
                format!("round {}   position: {:?}", f.state.round(), f.verdict),
                verdict_color(f.verdict),
            ),
        };
        text(h, status.0, 14.0, status.1);
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
            &["unit", "rg", "M", "V", "G", "C", "F", "hp", "kind"],
            MUTED,
            12.0,
        );
        for i in 0..b.units.len() {
            let u = &b.units[i];
            let colour = if u.fallen {
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
            row_cells(
                t,
                &[
                    &format!("{side}{}", u.name),
                    &place,
                    &u.might.to_string(),
                    &u.health.to_string(), // Vitality shown as current HP; full at start
                    &u.grit.to_string(),
                    &u.cadence.to_string(),
                    &u.finesse.to_string(),
                    &u.health.to_string(),
                    kind.trim(),
                ],
                colour,
                12.5,
            );
        }
        text(
            t,
            "M might  V vitality  G grit  C cadence  F finesse   rg = region+post (F front / b back)",
            10.0,
            MUTED,
        );
    });
}

fn row_cells(p: &mut ChildSpawnerCommands, cells: &[&str], colour: Color, size: f32) {
    // fixed widths so columns line up without a real grid
    let widths = [120.0, 34.0, 26.0, 26.0, 26.0, 26.0, 26.0, 30.0, 90.0];
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
        text(panel, "history", 11.0, MUTED);
        for line in f.log.iter().rev().take(16).collect::<Vec<_>>().iter().rev() {
            text(panel, (*line).clone(), 12.0, MUTED);
        }
    });
}

/// The options, each a clickable button carrying its verdict and win/loss line counts.
fn options_panel(p: &mut ChildSpawnerCommands, f: &Fight) {
    text(p, "your options", 16.0, INK);
    if Combat::outcome(&f.state).is_some() {
        text(
            p,
            "the fight is over - Restart or Next encounter.",
            13.0,
            MUTED,
        );
        return;
    }
    text(
        p,
        "each shows: solver verdict, then winning / losing lines through it (a tie is a loss)",
        11.0,
        MUTED,
    );

    for (i, (c, v, paths)) in f.scored.iter().enumerate() {
        let ge = if paths.complete { "" } else { ">=" };
        let counts = format!(
            "{ge}{} win / {ge}{} lose",
            abbrev(paths.wins),
            abbrev(paths.losses)
        );
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
            BorderColor::all(verdict_color(*v)),
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
            text(row, format!("{v:?}"), 12.0, verdict_color(*v));
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
