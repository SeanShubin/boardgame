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
//! 1. **A region is a formation, not a bag.** Within each region every side has a **front** and a **back**. The
//!    front is drawn on top; the back is shingled beneath it, behind a "behind the line" marker. Kill the front
//!    and the marker turns red: the back is exposed, and there is nobody left paying for it.
//!
//! 2. **The three old roles are still here - as consequences, not as ranks.**
//!    - **Vanguard** = a body at the **front**. It can be shot and clashed with, and it *catches slippers*.
//!    - **Rearguard** = a body at the **back** with a ranged strike. Deals from safety somebody else is paying
//!      for. Ranged fire cannot even pick it out while a front stands.
//!    - **Outrider** = a **melee strike at an enemy in the back**. Not a role, not even a separate declaration:
//!      it is simply *what pressing a screened target means*. Any front can catch you. Get through and you are
//!      on the cannon; get caught and your tempo went to the body that caught you.
//!
//!    `Slip > Cannon > Front > Slip` - the old triangle, inside a region, with no ranks anywhere.
//!
//! 3. **The screen is a price, never an immunity.** Enough Tempo always gets past a front. (The previous cut
//!    silently *redirected* every blow onto the screen, which made a screened body untouchable until its guard
//!    died - that was fiat, and it had quietly deleted the Outrider.)
//!
//! 4. **An area strike nukes the whole region**, both tiers, bypassing the screen entirely. That is the
//!    anti-cluster counter, and it is what stops a deep formation from being free.
//!
//! 5. **The doom chart** (right). Every move each hero could make, and whether the position is still winnable if
//!    it makes it. It asks what **forecloses** the win, not what is optimal.
//!
//! # How to drive it
//!
//! Click a **hero** to select it. **Every legal declaration is listed on the right, with the oracle's verdict
//! beside it - click a row to declare it.** That list is the whole move set; card clicks are just shortcuts into
//! it. Nothing is ever a dead end:
//!
//! - click the **selected hero again** -> deselect (back to where you started)
//! - the **Undo** row at the top of the chart -> un-declare that hero
//! - **Clear all** -> put every hero back to undeclared
//!
//! Card shortcut: with a hero selected, click an **enemy** to strike it.
//!
//! # What a body declares
//!
//! A **post** (front or back - where it stands in its own line) and an **act**:
//!
//! | | |
//! |---|---|
//! | **Strike X** | a melee body crosses to X and pays the crossing gauntlet; an arrow does not walk. If X is at their **back**, this is a *slip* - their front will try to catch you. |
//! | **Hold the front / back** | take your post and wait. |
//! | **Fall in with X** | cross to that region without attacking. |
//! | **Peel away alone** | retreat to empty ground. Not free: everything hostile in the region you leave gets a parting blow at you. |
//!
//! A body that **crosses arrives at the front** - you charged in, you are at the sharp end by definition - so a
//! crossing act carries no post choice.
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
use deckbound_board::regions::{
    self, Act, Aim, Board, Oracle, Post, SubPhase, Verdict, legal_aims,
};
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
    /// which stole the one gesture a player reaches for to say *"never mind"*. The chart is now the full move
    /// list, so a card click is only ever a *shortcut* into it - and every state is leavable.
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
            // A card click is a SHORTCUT into the chart, never the only way in: clicking an enemy declares the
            // strike, keeping whatever post this hero is already committed to (or the front, by default).
            Some(h) if self.board.units[i].side != self.board.units[h].side => {
                let post = self.aims[h].map_or(Post::Front, |a| a.post);
                let want = Aim::new(post, Act::Strike(i));
                let legal = legal_aims(&self.board, h);
                let pick = if legal.contains(&want) {
                    Some(want)
                } else {
                    // The post it wanted is not on offer for this strike (a crosser always arrives at the
                    // front) - so take whichever post the model does offer for it.
                    legal.into_iter().find(|a| a.act == Act::Strike(i))
                };
                if let Some(a) = pick {
                    self.declare(h, a);
                }
            }
            Some(_) => {} // clicking an ally does nothing now - there is no "guard X" to declare
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
            .map(|i| self.aims[i].unwrap_or(Aim::WAIT))
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
                aims[i].label(&self.board)
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
    /// A row of the doom chart: declare this aim for the selected hero. The chart IS the move list, so every
    /// aim has a home here - including the ones (Hold, Peel) that have no card to click.
    Declare(Aim),
    /// Take a hero's declaration back. Every state must be reachable *and* leavable.
    Undeclare(usize),
    /// Put every hero back to undeclared - the big escape hatch.
    ClearAll,
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
            Hit::Undeclare(h) => {
                s.aims[h] = None;
                s.selected = Some(h); // leave it selected, so you can immediately pick something else
                s.repick();
            }
            Hit::ClearAll => {
                s.aims = vec![None; s.board.units.len()];
                s.selected = None;
                s.repick();
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

/// The render order for one region: **the front line first, then the back line, shingled beneath it** - per
/// side, so a mixed region (a melee) shows both formations.
///
/// The shingle *is* the rule, drawn: a body at the back is tucked under the bodies that are screening it, and
/// the cards on top are the ones an attacker has to go through. It is not a decoration - `front_line` is the
/// same function the resolver uses to decide who can catch a slipper.
fn tiers(s: &Spike, region: u8, side: Side) -> (Vec<usize>, Vec<usize>) {
    let here: Vec<usize> = s
        .board
        .in_region(region)
        .into_iter()
        .filter(|&i| s.board.units[i].side == side)
        .collect();
    let post = |i: usize| s.aims[i].map_or(Post::Front, |a| a.post);
    let front: Vec<usize> = here
        .iter()
        .copied()
        .filter(|&i| post(i) == Post::Front)
        .collect();
    let back: Vec<usize> = here
        .iter()
        .copied()
        .filter(|&i| post(i) == Post::Back)
        .collect();
    (front, back)
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
                    "{} selected - pick any row on the right, or click an enemy to press / an ally to guard. Click it again to deselect.",
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
                // Each side's formation in this region: its front line, then its back line shingled beneath.
                for side in [Side::Party, Side::Foe] {
                    let (front, back) = tiers(s, r, side);
                    if front.is_empty() && back.is_empty() {
                        continue;
                    }
                    for &i in &front {
                        card(panel, s, i, 0);
                    }
                    if !back.is_empty() {
                        let screened = !front.is_empty();
                        label(
                            panel,
                            if screened {
                                "-- behind the line --"
                            } else {
                                "-- no line left: the back is exposed --"
                            },
                            10.0,
                            if screened { GOOD } else { BAD },
                        );
                        for &i in &back {
                            card(panel, s, i, 1);
                        }
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
            label(c, a.label(&s.board), 11.0, PICK);
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
        button(row, Hit::ClearAll, "Clear all", PANEL);
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
/// **The chart IS the move list.** Every row is clickable and declares that aim, so every state is reachable
/// from here - including the ones with no card to click (`Hold`, `Peel`). A card click is only ever a
/// *shortcut* into this list, never the only way in. And every state is **leavable**: the hero's current pick
/// is highlighted, and the first row takes it back.
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

    // Leaving a state must be exactly as easy as entering it.
    match s.aims[h] {
        Some(a) => row(
            p,
            Hit::Undeclare(h),
            &format!("Undo: {}", a.label(&s.board)),
            "take it back",
            PICK,
            PANEL,
        ),
        None => label(p, "click a row to declare it", 11.0, MUTED),
    }

    for (a, v) in &s.chart {
        let chosen = s.aims[h] == Some(*a);
        row(
            p,
            Hit::Declare(*a),
            &a.label(&s.board),
            &format!("{v:?}"),
            verdict_color(*v),
            if chosen { HERO } else { SUNK },
        );
    }
}

/// One clickable line of the chart: a label on the left, a verdict on the right.
fn row(
    p: &mut ChildSpawnerCommands,
    hit: Hit,
    text: &str,
    right: &str,
    right_color: Color,
    bg: Color,
) {
    p.spawn((
        Button,
        hit,
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(bg),
    ))
    .with_children(|r| {
        label(r, text, 13.0, INK);
        label(r, right, 12.0, right_color);
    });
}
