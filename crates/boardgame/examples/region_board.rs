//! **The regions / relations combat model, on a table you can click.**
//!
//! A design spike, not the product. It drives [`deckbound_board::regions`] - the model
//! `examples/v2_regions` proved load-bearing - so you can *judge the consequences of switching* instead of
//! reading about them. Nothing here touches the shipped arena, renderer, or solver.
//!
//! Run: `cargo run --release -p boardgame --example region_board`
//!
//! # What to look at
//!
//! 1. **Regions are just who is standing together.** No map, no coordinates, no ranks. A region with an enemy
//!    in it is a *melee*; one without is **the back** - which is not a rank any more, it is a fact about the
//!    board, and it stops being true the moment somebody walks in.
//!
//! 2. **The screen is a cascade you can see.** Declare `Defend` and your card physically **shingles over your
//!    ward** - depth of shingle is depth of protection, and the card on top is the one an attacker has to go
//!    through. That is not a decoration invented for the renderer: it is literally the rule (a blow aimed at
//!    the ward lands on the head of the chain), drawn.
//!
//! 3. **The doom chart** (right). Every move each hero could make, and whether the position is still winnable
//!    if it makes it. It asks what **forecloses** the win, not what is optimal. On Greywater Ford it says the
//!    thing the current model cannot even express: *the Raider must not charge in round one.*
//!
//! 4. **Round 4 still reads.** Step the rounds and watch the board. That is the whole question this design
//!    exists to answer.
//!
//! # How to drive it
//!
//! Click a **hero** to select it (the chart fills in). Then click:
//! - an **enemy** -> `Press` it (melee crosses to it; ranged stays and shoots)
//! - an **ally**  -> `Defend` it (your body goes in the way - watch the shingle)
//! - **that hero again** -> deselect. Clicking it back off always returns you to where you started.
//! - **a row of the chart** -> declare that aim directly. This is where `Withdraw` lives.
//!
//! **Withdraw** = peel off alone to fresh ground. The retreat: how a cannon escapes a melee that walked in on
//! it, or how a cluster breaks up so one area strike cannot catch it all. It is *not free* - you are crossing,
//! so everything hostile in the region you leave gets a parting blow at you. If you are already alone it does
//! nothing.
//!
//! Declare for every hero, then **Resolve the round**.
//!
//! # A note on the oracle
//!
//! Selecting is **instant**. The oracle never runs on the click: every verdict starts `Evaluating` and is
//! ground down a frame at a time ([`grind`]), so the UI stays live and the chart fills in as each answer
//! *settles*. `Evaluating` is honest - it means "not sure yet", never "probably fine". One memo is shared by
//! every question about a position, which is what makes the second question nearly free.

use bevy::prelude::*;

use deckbound_board::combat::{Combatant, Side};
use deckbound_board::regions::{self, Aim, Board, Oracle, SubPhase, Verdict, legal_aims};
use deckbound_content::catalog::{self, Creature, Encounter};
use deckbound_content::rank::Intention as Rank;

const FELT: Color = Color::srgb(0.08, 0.09, 0.11);
const PANEL: Color = Color::srgb(0.14, 0.15, 0.18);
const SUNK: Color = Color::srgb(0.11, 0.12, 0.14);
const INK: Color = Color::srgb(0.93, 0.94, 0.96);
const MUTED: Color = Color::srgb(0.62, 0.66, 0.72);
const HERO: Color = Color::srgb(0.24, 0.34, 0.46);
const FOE: Color = Color::srgb(0.42, 0.24, 0.26);
const PICK: Color = Color::srgb(0.95, 0.82, 0.35);
const GOOD: Color = Color::srgb(0.45, 0.80, 0.65);
const WARN: Color = Color::srgb(0.85, 0.60, 0.30);
const BAD: Color = Color::srgb(0.88, 0.38, 0.40);

/// How much search one frame is allowed to pay for. A whole encounter measures a few hundred nodes, so this is
/// generous - it exists so that a click is *never* the thing that pays for a search: the answer arrives over
/// the next frame or two while the UI stays live, and says `Evaluating` honestly until it is sure.
const FRAME_NODES: u64 = 3_000;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "regions + relations - combat model spike".into(),
                resolution: (1480u32, 940u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(FELT))
        .init_resource::<Spike>()
        .add_systems(Startup, (setup_camera, rebuild).chain())
        .add_systems(Update, (on_click, grind, rebuild.run_if(is_dirty)).chain())
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

// ---- state ---------------------------------------------------------------------------------------------

#[derive(Resource)]
struct Spike {
    encounter: usize,
    board: Board,
    /// The aim each hero has declared this round; `None` = not yet declared. Foes are scripted.
    aims: Vec<Option<Aim>>,
    selected: Option<usize>,
    round: usize,
    /// The per-move verdicts for the selected hero - the doom chart. An entry stays `Evaluating` until the
    /// oracle has actually settled it; it is never a guess.
    chart: Vec<(Aim, Verdict)>,
    /// The overall verdict for the position at the top of this round.
    verdict: Verdict,
    /// **One oracle per board position, shared by every question asked about it.** This is the whole reason
    /// selection is instant: the memo is what makes the second question cheap, so building a fresh `Oracle` per
    /// chart row (as the first cut did) threw away exactly the thing that makes this affordable and re-walked
    /// the tree seven times over. Rebuilt only when the *board* changes - never when the selection does.
    oracle: Oracle,
    /// How many nodes the next walk may spend. Doubles whenever a walk comes back `Evaluating`, and resets once
    /// a question settles - so an easy position never over-spends and a hard one still converges.
    grant: u64,
    log: Vec<String>,
    dirty: bool,
}

impl Default for Spike {
    fn default() -> Self {
        let mut s = Spike {
            encounter: 5, // Greywater Ford - the one whose chart actually discriminates
            board: Board::opening(vec![]),
            aims: Vec::new(),
            selected: None,
            round: 0,
            chart: Vec::new(),
            verdict: Verdict::Evaluating,
            oracle: Oracle::new(0),
            grant: FRAME_NODES,
            log: Vec::new(),
            dirty: true,
        };
        s.reset();
        s
    }
}

impl Spike {
    fn encounter(&self) -> &'static Encounter {
        &catalog::ENCOUNTERS[self.encounter % catalog::ENCOUNTERS.len()]
    }

    fn reset(&mut self) {
        self.board = setup(self.encounter());
        self.aims = vec![None; self.board.units.len()];
        self.selected = None;
        self.round = 0;
        self.log = vec![format!(
            "{} - {}",
            self.encounter().title,
            self.encounter().flavor
        )];
        self.reposition();
    }

    /// **The board changed** - throw the memo away (it is keyed to the old position) and start asking again.
    /// Nothing is *computed* here: every verdict starts `Evaluating` and is ground down by [`grind`], a frame
    /// at a time, so a click is never the thing that pays for a search.
    fn reposition(&mut self) {
        self.oracle = Oracle::new(0);
        self.grant = FRAME_NODES;
        self.verdict = Verdict::Evaluating;
        self.repick();
    }

    /// **The selection changed** - the chart is about a different hero, but it is the *same board*, so the memo
    /// still applies and is kept. That is why selecting a second hero is nearly free.
    fn repick(&mut self) {
        self.chart = match self.selected {
            Some(h) if !self.board.units[h].fallen => legal_aims(&self.board, h)
                .into_iter()
                .map(|a| (a, Verdict::Evaluating))
                .collect(),
            _ => Vec::new(),
        };
        self.dirty = true;
    }

    /// One frame's worth of search: grant the oracle a slice of nodes and settle **one** pending question -
    /// the position's own verdict first, then the chart rows in order. Returns whether anything settled.
    ///
    /// A question that runs out of its grant stays `Evaluating` and is retried next frame with a **doubled**
    /// grant. That escalation is not optional: a subtree only memoizes once it is fully explored, so a grant too
    /// small to settle *any* new subtree makes no progress however many frames it is repeated for. Doubling
    /// settles anything in a handful of frames, and the memo survives every retry so nothing is re-derived.
    ///
    /// Safety does not depend on any of this: an incomplete subtree is never memoized, so a starved oracle can
    /// only ever be **silent**, never wrong. Pinned by `regions::tests::a_starved_oracle_is_silent_never_wrong`.
    fn grind(&mut self) -> bool {
        self.oracle.grant(self.grant);
        if self.verdict == Verdict::Evaluating {
            // Disjoint fields: `oracle` is borrowed mutably, `board` immutably. Rust permits this.
            let v = self.oracle.verdict(&self.board, self.round);
            if v != Verdict::Evaluating {
                self.verdict = v;
                self.grant = FRAME_NODES;
                return true;
            }
            self.grant = self.grant.saturating_mul(2); // that bite was too small - take a bigger one
            return false; // still chewing - do not start a chart row while the position itself is unknown
        }
        let Some(h) = self.selected else { return false };
        let Some(k) = self
            .chart
            .iter()
            .position(|(_, v)| *v == Verdict::Evaluating)
        else {
            return false; // everything settled
        };
        let aim = self.chart[k].0;
        let v = self.oracle.verdict_for(&self.board, self.round, h, aim);
        if v == Verdict::Evaluating {
            self.grant = self.grant.saturating_mul(2);
        } else {
            self.grant = FRAME_NODES;
        }
        if v != Verdict::Evaluating {
            self.chart[k].1 = v;
            return true;
        }
        false
    }

    fn heroes(&self) -> Vec<usize> {
        (0..self.board.units.len())
            .filter(|&i| self.board.units[i].side == Side::Party && !self.board.units[i].fallen)
            .collect()
    }

    fn all_declared(&self) -> bool {
        self.heroes().iter().all(|&i| self.aims[i].is_some())
    }

    /// Click a card: select a living hero, **deselect it if it is already selected**, or - with one selected -
    /// declare its aim against this card.
    ///
    /// Clicking the selected hero again **puts you back where you started**. It used to declare `Withdraw`,
    /// which was a bad overload of the one gesture a player reaches for to say *"never mind"* - so `Withdraw`
    /// moved to its own row in the chart, where it belongs: it is a **move**, not something you do *to* a card.
    fn click(&mut self, i: usize) {
        if self.board.outcome().is_some() || self.board.units[i].fallen {
            return;
        }
        match self.selected {
            // Toggle back to the original state - the escape hatch.
            Some(h) if h == i => {
                self.selected = None;
                self.repick();
            }
            Some(h) => {
                let aim = if self.board.units[i].side == Side::Party {
                    Aim::Defend(i)
                } else {
                    Aim::Press(i)
                };
                self.declare(h, aim);
            }
            None if self.board.units[i].side == Side::Party => {
                self.selected = Some(i);
                self.repick();
            }
            None => {}
        }
    }

    /// Declare `aim` for hero `h` - gated on what the model actually offers, so the chart and the board can
    /// never disagree about what was legal.
    fn declare(&mut self, h: usize, aim: Aim) {
        if !legal_aims(&self.board, h).contains(&aim) {
            return;
        }
        self.aims[h] = Some(aim);
        self.selected = None;
        self.repick();
    }

    /// Play the round: the declared aims, the foes' script, and the four sub-phases.
    fn resolve(&mut self) {
        if !self.all_declared() || self.board.outcome().is_some() {
            return;
        }
        let mut aims: Vec<Aim> = (0..self.board.units.len())
            .map(|i| self.aims[i].unwrap_or(Aim::Withdraw))
            .collect();
        for (i, a) in regions::foe_aims(&self.board).iter().enumerate() {
            if let Some(a) = a {
                aims[i] = *a;
            }
        }

        self.log.push(format!("--- Round {} ---", self.round + 1));
        for &i in &self.heroes() {
            self.log.push(format!(
                "  {} -> {}",
                self.board.units[i].name,
                aims[i].label(&self.board.units)
            ));
        }

        let logs = regions::play_round(&mut self.board, &aims);
        for (phase, l) in SubPhase::ALL.iter().zip(&logs) {
            let mut notes: Vec<String> = Vec::new();
            for &i in &l.caught {
                notes.push(format!("{} is CAUGHT crossing", self.board.units[i].name));
            }
            for &i in &l.arrived {
                notes.push(format!("{} gets through", self.board.units[i].name));
            }
            for &i in &l.fallen {
                notes.push(format!("{} FALLS", self.board.units[i].name));
            }
            if !notes.is_empty() {
                self.log
                    .push(format!("  {}: {}", phase.label(), notes.join("; ")));
            }
        }

        self.round += 1;
        self.aims = vec![None; self.board.units.len()];
        self.selected = None;
        match self.board.outcome() {
            Some(true) => self.log.push("*** the party wins ***".into()),
            Some(false) => self.log.push("*** the party falls ***".into()),
            None if self.round >= regions::MAX_ROUNDS => {
                self.log.push("*** draw at the round cap ***".into())
            }
            None => {}
        }
        self.reposition();
    }
}

fn setup(e: &Encounter) -> Board {
    let kit = |(name, stats, ability): (&'static str, [u8; 5], &'static str)| {
        let (melee, ranged) = catalog::ability_reach(ability);
        let (_r, aoe) = catalog::ability_shape(ability);
        Combatant::from_stats(name, Side::Party, Rank::Vanguard, stats, 0, melee, ranged)
            .with_aoe(aoe)
    };
    let beast = |c: &Creature| {
        Combatant::from_stats(
            c.name,
            Side::Foe,
            Rank::Vanguard,
            c.stats,
            0,
            c.melee,
            c.ranged,
        )
        .with_aoe(c.aoe)
        .as_horde(c.horde)
    };
    let mut units: Vec<Combatant> = catalog::ROSTER.iter().copied().map(kit).collect();
    for (c, q) in catalog::encounter_foes(e) {
        for _ in 0..q {
            units.push(beast(c));
        }
    }
    Board::opening(units)
}

// ---- input ---------------------------------------------------------------------------------------------

#[derive(Component, Clone, Copy)]
enum Hit {
    Card(usize),
    /// A row of the doom chart: declare this aim for the selected hero. This is where `Withdraw` lives - it is
    /// a move, not something you do *to* another card, so it has no card to click.
    Declare(Aim),
    Resolve,
    Reset,
    NextEncounter,
}

#[derive(Component)]
struct Root;

fn is_dirty(s: Res<Spike>) -> bool {
    s.dirty
}

/// Chip away at whatever the oracle has not settled yet, a frame at a time, and redraw when something lands.
/// The UI never blocks on a search: it shows `Evaluating` and means it.
fn grind(mut s: ResMut<Spike>) {
    if s.grind() {
        s.dirty = true;
    }
}

fn on_click(mut s: ResMut<Spike>, q: Query<(&Interaction, &Hit), Changed<Interaction>>) {
    for (i, hit) in &q {
        if *i != Interaction::Pressed {
            continue;
        }
        match *hit {
            Hit::Card(u) => s.click(u),
            Hit::Declare(a) => {
                if let Some(h) = s.selected {
                    s.declare(h, a);
                }
            }
            Hit::Resolve => s.resolve(),
            Hit::Reset => s.reset(),
            Hit::NextEncounter => {
                s.encounter += 1;
                s.reset();
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

/// Who each unit is shingled *under*: `screen[w] = Some(d)` when `d` declared `Defend(w)` from `w`'s region.
/// This is the same edge [`regions::screen_head`] walks - the picture and the rule are the same object.
fn screens(s: &Spike) -> Vec<Option<usize>> {
    let b = &s.board;
    (0..b.units.len())
        .map(|w| {
            (0..b.units.len()).find(|&d| {
                !b.units[d].fallen
                    && b.units[d].side == b.units[w].side
                    && b.regions[d] == b.regions[w]
                    && s.aims[d] == Some(Aim::Defend(w))
            })
        })
        .collect()
}

/// The render order for one region: each screen chain from its **head** down to its ward, so a defender is
/// drawn above the body it is covering and the ward can tuck under it. Depth of shingle = depth of protection.
fn chains(s: &Spike, region: u8) -> Vec<Vec<usize>> {
    let b = &s.board;
    let screen = screens(s);
    let here = b.in_region(region);
    let mut out = Vec::new();
    for &head in &here {
        if screen[head].is_some() {
            continue; // somebody is covering it - it will be drawn under them
        }
        let mut chain = vec![head];
        let mut at = head;
        // Walk DOWN: what is this body covering?
        while let Some(Aim::Defend(w)) = s.aims[at] {
            if !here.contains(&w) || chain.contains(&w) {
                break; // out of region, or a mutual-defend cycle - legal, just not a line
            }
            chain.push(w);
            at = w;
        }
        out.push(chain);
    }
    out
}

fn rebuild(mut commands: Commands, mut s: ResMut<Spike>, old: Query<Entity, With<Root>>) {
    for e in &old {
        commands.entity(e).despawn();
    }
    s.dirty = false;
    let s = &*s;

    commands
        .spawn((
            Root,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                padding: UiRect::all(Val::Px(14.0)),
                column_gap: Val::Px(14.0),
                ..default()
            },
        ))
        .with_children(|root| {
            // ---- left: the table -----------------------------------------------------------------------
            root.spawn(Node {
                width: Val::Percent(68.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..default()
            })
            .with_children(|left| {
                header(left, s);
                regions_row(left, s);
                controls(left, s);
                log_panel(left, s);
            });

            // ---- right: the doom chart -----------------------------------------------------------------
            root.spawn((
                Node {
                    width: Val::Percent(32.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(8.0),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(PANEL),
            ))
            .with_children(|right| chart_panel(right, s));
        });
}

fn label(p: &mut ChildSpawnerCommands, text: impl Into<String>, size: f32, color: Color) {
    p.spawn((
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    ));
}

fn header(p: &mut ChildSpawnerCommands, s: &Spike) {
    let e = s.encounter();
    p.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(2.0),
        ..default()
    })
    .with_children(|h| {
        label(
            h,
            format!(
                "{}  -  Round {} of {}",
                e.title,
                (s.round + 1).min(regions::MAX_ROUNDS),
                regions::MAX_ROUNDS
            ),
            22.0,
            INK,
        );
        let v = match s.board.outcome() {
            Some(true) => ("the party has won".to_string(), GOOD),
            Some(false) => ("the party has fallen".to_string(), BAD),
            None => (
                format!("the oracle says: {:?}", s.verdict),
                verdict_color(s.verdict),
            ),
        };
        label(h, v.0, 15.0, v.1);
        label(
            h,
            match s.selected {
                Some(i) => format!(
                    "{} is selected - click an enemy to Press, an ally to Defend, or itself to Withdraw",
                    s.board.units[i].name
                ),
                None => "click a hero to select it".to_string(),
            },
            13.0,
            MUTED,
        );
    });
}

/// The table: one panel per occupied region. A region holding both sides is a **melee**; one holding only your
/// own is **the back** - and the badge says which, because that is the whole shielding model.
fn regions_row(p: &mut ChildSpawnerCommands, s: &Spike) {
    p.spawn(Node {
        flex_direction: FlexDirection::Row,
        column_gap: Val::Px(12.0),
        align_items: AlignItems::FlexStart,
        flex_grow: 1.0,
        ..default()
    })
    .with_children(|row| {
        for r in s.board.occupied() {
            let contested = !s.board.is_safe(r, Side::Party) && !s.board.is_safe(r, Side::Foe);
            row.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(6.0),
                    min_width: Val::Px(190.0),
                    border: UiRect::all(Val::Px(if contested { 2.0 } else { 1.0 })),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(SUNK),
                BorderColor::all(if contested {
                    BAD
                } else {
                    MUTED.with_alpha(0.3)
                }),
            ))
            .with_children(|panel| {
                label(
                    panel,
                    format!(
                        "Region {}   {}",
                        (b'A' + r) as char,
                        if contested { "MELEE" } else { "safe ground" }
                    ),
                    13.0,
                    if contested { BAD } else { MUTED },
                );
                for chain in chains(s, r) {
                    for (depth, &i) in chain.iter().enumerate() {
                        card(panel, s, i, depth);
                    }
                }
            });
        }
    });
}

/// One body. `depth > 0` means it is **shingled under its screen** - drawn tucked beneath the card that is
/// covering it, with an inset, so the picture *is* the rule: the card on top is the one you have to go through.
fn card(p: &mut ChildSpawnerCommands, s: &Spike, i: usize, depth: usize) {
    let u = &s.board.units[i];
    let selected = s.selected == Some(i);
    let base = if u.side == Side::Party { HERO } else { FOE };
    let mut tags: Vec<&str> = Vec::new();
    if u.melee {
        tags.push("melee");
    }
    if u.ranged {
        tags.push("ranged");
    }
    if u.aoe {
        tags.push("area");
    }
    if u.horde {
        tags.push("horde");
    }

    p.spawn((
        Button,
        Hit::Card(i),
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(7.0)),
            row_gap: Val::Px(1.0),
            // The shingle: tuck under the screen above, and inset so the chain reads as depth.
            margin: UiRect {
                top: Val::Px(if depth > 0 { -6.0 } else { 0.0 }),
                left: Val::Px(depth as f32 * 12.0),
                ..default()
            },
            border: UiRect::all(Val::Px(if selected { 2.0 } else { 1.0 })),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(base),
        BorderColor::all(if selected {
            PICK
        } else {
            Color::BLACK.with_alpha(0.45)
        }),
    ))
    .with_children(|c| {
        label(
            c,
            format!("{}  {}hp", u.name, u.health),
            14.0,
            if u.fallen { MUTED } else { INK },
        );
        label(
            c,
            format!(
                "might {}  grit {}  cadence {}  finesse {}",
                u.might, u.grit, u.cadence, u.finesse
            ),
            10.0,
            MUTED,
        );
        label(c, tags.join(" / "), 10.0, MUTED);
        if depth > 0 {
            label(c, "guarded - the screen takes the blow", 10.0, GOOD);
        }
        if let Some(a) = s.aims[i] {
            label(c, a.label(&s.board.units), 11.0, PICK);
        } else if u.side == Side::Party {
            label(c, "undeclared", 11.0, WARN);
        }
    });
}

fn controls(p: &mut ChildSpawnerCommands, s: &Spike) {
    let ready = s.all_declared() && s.board.outcome().is_none();
    p.spawn(Node {
        flex_direction: FlexDirection::Row,
        column_gap: Val::Px(8.0),
        ..default()
    })
    .with_children(|row| {
        button(
            row,
            Hit::Resolve,
            if ready {
                "Resolve the round"
            } else {
                "declare for every hero first"
            },
            if ready { GOOD } else { PANEL },
        );
        button(row, Hit::Reset, "Reset", PANEL);
        button(row, Hit::NextEncounter, "Next encounter", PANEL);
    });
}

fn button(p: &mut ChildSpawnerCommands, hit: Hit, text: &str, bg: Color) {
    p.spawn((
        Button,
        hit,
        Node {
            padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor::all(MUTED.with_alpha(0.4)),
    ))
    .with_children(|b| label(b, text, 13.0, INK));
}

fn log_panel(p: &mut ChildSpawnerCommands, s: &Spike) {
    p.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(10.0)),
            height: Val::Px(210.0),
            overflow: Overflow::clip(),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(PANEL),
    ))
    .with_children(|panel| {
        for line in s.log.iter().rev().take(11).collect::<Vec<_>>().iter().rev() {
            let c = if line.starts_with("---") || line.starts_with("***") {
                PICK
            } else if line.contains("FALLS") || line.contains("CAUGHT") {
                BAD
            } else {
                MUTED
            };
            label(panel, (*line).clone(), 12.0, c);
        }
    });
}

/// **The doom chart.** Every move the selected hero could make, and whether the position is still winnable if
/// it makes it. It asks what *forecloses* the win, not what is optimal - which is what a player actually wants
/// to know. On Greywater Ford it says the thing the rank model cannot express: the Raider must not charge.
///
/// **Every row is clickable** - it declares that aim. That is what gives `Withdraw` a home: it is a *move*, not
/// something you do *to* another card, so it has no card to click. And it means the chart is not a read-out
/// bolted onto the UI - it **is** the move list, with the consequence of each move printed next to it.
fn chart_panel(p: &mut ChildSpawnerCommands, s: &Spike) {
    label(p, "The doom oracle", 18.0, INK);
    let Some(h) = s.selected else {
        label(
            p,
            "Select a hero to see every move it could make, and what each one costs.",
            13.0,
            MUTED,
        );
        return;
    };
    label(p, s.board.units[h].name.clone(), 15.0, PICK);

    let pending = s.chart.iter().any(|(_, v)| *v == Verdict::Evaluating);
    let decisive = s.chart.iter().any(|(_, v)| matches!(v, Verdict::Doomed));
    label(
        p,
        if pending {
            "thinking - a verdict appears as soon as the oracle is sure of it"
        } else if decisive {
            "This choice matters - some moves lose the fight outright."
        } else {
            "Every move keeps the win. No decision here."
        },
        12.0,
        if pending || decisive { WARN } else { MUTED },
    );
    label(p, "click a row to declare it", 11.0, MUTED);

    for (a, v) in &s.chart {
        p.spawn((
            Button,
            Hit::Declare(*a),
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(SUNK),
        ))
        .with_children(|row| {
            label(row, a.label(&s.board.units), 13.0, INK);
            label(row, format!("{v:?}"), 13.0, verdict_color(*v));
        });
    }

    // Withdraw is the one aim with no card to click, so say what it is where it lives.
    label(
        p,
        "Withdraw: peel off alone to fresh ground. The retreat - how a cannon escapes a melee \
         that walked in on it, or how a cluster breaks up so one area strike cannot catch it all. \
         Not free: you are crossing, so everything hostile in the region you leave gets a parting \
         blow at you. If you are already alone, it does nothing.",
        11.0,
        MUTED,
    );
}
