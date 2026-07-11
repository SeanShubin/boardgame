//! §8 — the playable **Campaign**: a world map you move on, entering locations to fight the §4
//! battle, **assigning the role-card rewards** a clear unlocks (§8.3, no currency), toward a run
//! victory (clear the objective).
//!
//! An `contract::Game` that **embeds** the combat game ([`battle_state`]) and folds its result back
//! into the run. A `suggest()` **guide** walks the reference scenario's scripted path
//! (A → B[p] → C[p] → final), so the UI can highlight the next on-script move.
//!
//! Tokens are general (each holds ≥1 member): this scenario uses **one token = the whole party**
//! (move/fight together); a future scenario can use one token per member (split up) with no rewrite.

use contract::{Accent, Game, GameError, MapTile, MapView, Outcome, PlayerId, TableView};

use crate::actor::Actor;
use crate::currency::Currency;
use crate::encounter::EncounterCard;
use crate::game::{self, Deckbound, battle_state};
use crate::reference::reference_scenario;
use crate::scenarios::{RewardId, build_character, build_encounter_foes};
use crate::solver;
use crate::state::State;
use crate::world::Run;

/// A party member: a clean-slate character specialising in one **track**, plus the **rewards**
/// (§8.3) the party has assigned to it — permanent, accreting (a character *is* its role cards, §8.5).
#[derive(Clone, Debug)]
pub struct Member {
    pub name: String,
    pub base: String,
    /// The role track this member specialises in (the guide assigns matching rewards here).
    pub track: Currency,
    /// Rewards assigned to this member — permanent; only ever grows (§0.1 monotone).
    pub rewards: Vec<RewardId>,
}

impl Member {
    fn actor(&self) -> Actor {
        let mut a = build_character(&self.base, &self.rewards);
        a.name = self.name.clone();
        a
    }
}

/// A movable token — a group of members that travel together. Its position is `run.positions[i]`.
#[derive(Clone, Debug)]
pub struct Token {
    pub members: Vec<usize>,
    /// Used its one move this Day?
    pub moved: bool,
    /// Used its one encounter this Day?
    pub acted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CampPhase {
    World,
    Battle,
}

/// What ends a run (§8.2). The reference lattice clears a single **objective** location; the 25-card
/// **grind base** is scored by acquiring **all treasure** — every reward unlocked (each suit to L5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Goal {
    /// Clear the `run.objective` location at its max level.
    Objective,
    /// Unlock all 25 rewards — the experience-grind base, scored in Days to acquire everything.
    AllTreasure,
}

/// The full campaign state.
#[derive(Clone, Debug)]
pub struct CampaignState {
    pub run: Run,
    pub party: Vec<Member>,
    pub tokens: Vec<Token>,
    pub encounters: Vec<EncounterCard>,
    pub phase: CampPhase,
    pub battle: Option<State>,
    /// (token, location) currently being fought.
    pub pending: Option<(usize, usize)>,
    pub seed: u64,
    pub log: Vec<String>,
    pub outcome: Option<Outcome>,
    /// What ends the run (§8.2).
    pub goal: Goal,
    /// The scripted location order the guide follows (indices into `run.locations`).
    pub script: Vec<usize>,
    /// Rewards unlocked by a clear but **not yet assigned** (§8.3 SD2 — assign at unlock). While
    /// non-empty, the only legal moves are to assign its head to a member. Derived (unlocked −
    /// assigned), so the build stays a §0.1 function of clears + assignment.
    pub unassigned: Vec<RewardId>,
}

/// One step a player can take in the campaign.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CampAction {
    /// Move token `t` to adjacent location `loc` (flips it face-up).
    Move(usize, usize),
    /// Enter location `loc` with token `t` — fight to clear it (at the location's max level).
    Enter(usize, usize),
    /// Assign the just-unlocked reward `(track, level)` permanently to member `m` (§8.3 SD2).
    Assign(usize, Currency, u32),
    /// End the Day — advance the clock; tokens refresh.
    EndDay,
    /// A combat action, delegated to the embedded battle.
    Battle(game::Action),
    /// Deliberately give up the current battle (forfeit): the token has acted, no clear. Distinct
    /// from undoing the `Enter` (which reverses the fight as if it never happened).
    Retreat,
}

/// The campaign ruleset (holds no state).
#[derive(Clone, Copy, Debug, Default)]
pub struct Campaign;

// ---- reward unlock & assignment (§8.3 SD2) --------------------------------

fn fully_cleared(s: &CampaignState, loc: usize) -> bool {
    s.run.cleared[loc] >= s.run.locations[loc].max_level
}

/// The five reward suits × five levels = 25 rewards; the grind goal is to hold them all.
const ALL_TREASURE: usize = 25;

/// Is the run's goal met (§8.2)? `Objective` → the objective location cleared; `AllTreasure` → all 25
/// rewards unlocked (every suit cleared to L5; cumulative, so the L5 clears suffice).
fn goal_met(s: &CampaignState) -> bool {
    match s.goal {
        Goal::Objective => s.run.is_won(),
        Goal::AllTreasure => s.run.unlocked().len() >= ALL_TREASURE,
    }
}

/// Every reward currently assigned to some member (the party's whole pool).
fn assigned_rewards(s: &CampaignState) -> Vec<RewardId> {
    s.party
        .iter()
        .flat_map(|m| m.rewards.iter().copied())
        .collect()
}

/// Recompute the not-yet-assigned queue = (rewards unlocked by clears) − (rewards already assigned).
/// Markovian: a pure function of `run.cleared` + the party's assignments (§0.1).
fn recompute_unassigned(s: &mut CampaignState) {
    let held = assigned_rewards(s);
    s.unassigned = s
        .run
        .unlocked()
        .into_iter()
        .map(|(track, level)| RewardId { track, level })
        .filter(|r| !held.contains(r))
        .collect();
    s.unassigned.sort_by_key(|r| (r.track.label(), r.level));
}

/// The member the guide would assign reward `r` to: the one specialising in its track, else member 0.
fn guide_assignee(s: &CampaignState, r: RewardId) -> usize {
    s.party.iter().position(|m| m.track == r.track).unwrap_or(0)
}

/// One step toward `target` from `from` over the layout (BFS), or `None` if unreachable / already there.
fn step_toward(run: &Run, from: usize, target: usize) -> Option<usize> {
    if from == target {
        return None;
    }
    let n = run.locations.len();
    let mut prev = vec![usize::MAX; n];
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(from);
    prev[from] = from;
    while let Some(i) = queue.pop_front() {
        for j in 0..n {
            if prev[j] == usize::MAX
                && run
                    .layout
                    .adjacent(run.locations[i].coord, run.locations[j].coord)
            {
                prev[j] = i;
                if j == target {
                    // walk back to the first step out of `from`.
                    let mut step = j;
                    while prev[step] != from {
                        step = prev[step];
                    }
                    return Some(step);
                }
                queue.push_back(j);
            }
        }
    }
    None
}

/// Build the foe roster for entering `loc` at its max level (§8.4).
fn foes_at(s: &CampaignState, loc: usize) -> Vec<Actor> {
    let level = s.run.locations[loc].max_level;
    build_encounter_foes(&s.encounters[loc], level)
}

/// The token's party as combat Actors.
fn party_actors(s: &CampaignState, token: usize) -> Vec<Actor> {
    s.tokens[token]
        .members
        .iter()
        .map(|&mi| s.party[mi].actor())
        .collect()
}

impl Campaign {
    /// Fold a finished battle back into the run: carry its play-by-play into the run feed (so a
    /// fight won in a single resolution doesn't flash by before you can read it), then record the
    /// clear or the retreat.
    fn resolve_battle(&self, s: &mut CampaignState, won: bool) {
        if let Some((t, loc)) = s.pending.take() {
            // Persist the battle's combat log into the run feed before the map reclaims the screen.
            if let Some(b) = &s.battle {
                let lines = b.log.clone();
                let name = s.run.locations[loc].name.clone();
                s.log.push(format!("-- {name} --"));
                s.log.extend(lines);
            }
            if won {
                let max = s.run.locations[loc].max_level;
                s.run.record_clear(loc, max);
                s.log
                    .push(format!("Cleared {}.", s.run.locations[loc].name));
                // A clear may unlock new rewards to assign (§8.3 SD2).
                recompute_unassigned(s);
                if !s.unassigned.is_empty() {
                    s.log.push(format!(
                        "{} reward(s) unlocked — assign them.",
                        s.unassigned.len()
                    ));
                }
            } else {
                s.log
                    .push(format!("Retreated from {}.", s.run.locations[loc].name));
            }
            s.tokens[t].acted = true;
        }
        s.battle = None;
        s.phase = CampPhase::World;
        if goal_met(s) {
            s.outcome = Some(Outcome::Win(PlayerId(0)));
            let msg = match s.goal {
                Goal::Objective => "The objective is cleared — the run is won!",
                Goal::AllTreasure => "All treasure acquired — the run is won!",
            };
            s.log.push(msg.into());
        }
    }

    /// The guide's recommended next action (the scripted A → B → C → final path).
    fn guide(&self, s: &CampaignState) -> Option<CampAction> {
        if s.phase == CampPhase::Battle {
            let battle = s.battle.as_ref()?;
            let actions = Deckbound.legal_actions(battle);
            return Some(CampAction::Battle(solver::greedy(battle, &actions)));
        }
        // Assign any just-unlocked reward before anything else (§8.3 SD2): to its track specialist.
        if let Some(&r) = s.unassigned.first() {
            return Some(CampAction::Assign(guide_assignee(s, r), r.track, r.level));
        }
        let t = 0; // single token, this scenario
        let pos = s.run.positions[t];
        let target = *s.script.iter().find(|&&loc| !fully_cleared(s, loc))?;
        if pos == target {
            if !s.tokens[t].acted {
                return Some(CampAction::Enter(t, target));
            }
            return Some(CampAction::EndDay); // acted (e.g. lost) — try again tomorrow
        }
        if !s.tokens[t].moved
            && let Some(step) = step_toward(&s.run, pos, target)
        {
            return Some(CampAction::Move(t, step));
        }
        Some(CampAction::EndDay)
    }
}

impl Game for Campaign {
    type State = CampaignState;
    type Action = CampAction;

    fn new_game(&self, _seed: u64, _players: usize) -> CampaignState {
        reference_campaign()
    }

    fn current_player(&self, state: &CampaignState) -> Option<PlayerId> {
        state.outcome.is_none().then_some(PlayerId(0))
    }

    fn legal_actions(&self, s: &CampaignState) -> Vec<CampAction> {
        if s.outcome.is_some() {
            return Vec::new();
        }
        if s.phase == CampPhase::Battle {
            let Some(battle) = &s.battle else {
                return Vec::new();
            };
            let mut a: Vec<CampAction> = Deckbound
                .legal_actions(battle)
                .into_iter()
                .map(CampAction::Battle)
                .collect();
            // A deliberate forfeit, alongside the combat choices.
            a.push(CampAction::Retreat);
            return a;
        }
        // A just-unlocked reward must be assigned before anything else (§8.3 SD2): one Assign per
        // member, for the head of the queue. No other world action is offered until it is placed.
        if let Some(&r) = s.unassigned.first() {
            return (0..s.party.len())
                .map(|m| CampAction::Assign(m, r.track, r.level))
                .collect();
        }
        let mut a = Vec::new();
        let t = 0;
        let pos = s.run.positions[t];
        // Move to any adjacent location (entering flips it face-up).
        if !s.tokens[t].moved {
            for loc in 0..s.run.locations.len() {
                if s.run.can_move(t, loc) {
                    a.push(CampAction::Move(t, loc));
                }
            }
        }
        // Enter the current location if it isn't fully cleared and we haven't acted.
        if !s.tokens[t].acted && !fully_cleared(s, pos) {
            a.push(CampAction::Enter(t, pos));
        }
        a.push(CampAction::EndDay);
        a
    }

    fn action_label(&self, s: &CampaignState, action: &CampAction) -> String {
        match action {
            CampAction::Move(_, loc) => match &s.run.locations[*loc].name {
                _ if !s.run.revealed[*loc] => "Travel to the unknown".into(),
                name => format!("Travel to {name}"),
            },
            CampAction::Enter(_, loc) => format!("Enter {}", s.run.locations[*loc].name),
            CampAction::Assign(m, track, level) => {
                let role = track.role().unwrap_or_else(|| track.label());
                format!("Assign {role} L{level} → {}", s.party[*m].name)
            }
            CampAction::EndDay => "End the day".into(),
            CampAction::Battle(b) => Deckbound.action_label(s.battle.as_ref().unwrap(), b),
            CampAction::Retreat => "Retreat (forfeit this fight)".into(),
        }
    }

    fn apply(&self, s: &mut CampaignState, action: &CampAction) -> Result<(), GameError> {
        match action {
            CampAction::Move(t, loc) => {
                if !s.run.move_to(*t, *loc) {
                    return Err(GameError::new("can't travel there"));
                }
                s.tokens[*t].moved = true;
                Ok(())
            }
            CampAction::Enter(t, loc) => {
                if s.tokens[*t].acted {
                    return Err(GameError::new("already fought today"));
                }
                let heroes = party_actors(s, *t);
                let foes = foes_at(s, *loc);
                s.battle = Some(battle_state(heroes, foes, false, s.seed));
                s.pending = Some((*t, *loc));
                s.phase = CampPhase::Battle;
                Ok(())
            }
            CampAction::Assign(m, track, level) => {
                let rid = RewardId {
                    track: *track,
                    level: *level,
                };
                if s.unassigned.first() != Some(&rid) {
                    return Err(GameError::new("that reward is not the one to assign now"));
                }
                if *m >= s.party.len() {
                    return Err(GameError::new("no such member"));
                }
                s.party[*m].rewards.push(rid);
                s.unassigned.remove(0);
                s.log.push(format!(
                    "Assigned {} L{level} to {}.",
                    track.role().unwrap_or_else(|| track.label()),
                    s.party[*m].name
                ));
                Ok(())
            }
            CampAction::EndDay => {
                s.run.end_day();
                for tok in &mut s.tokens {
                    tok.moved = false;
                    tok.acted = false;
                }
                Ok(())
            }
            CampAction::Battle(b) => {
                let battle = s.battle.as_mut().ok_or(GameError::new("no battle"))?;
                Deckbound.apply(battle, b)?;
                if let Some(out) = Deckbound.outcome(battle) {
                    let won = matches!(out, Outcome::Win(PlayerId(0)));
                    self.resolve_battle(s, won);
                }
                Ok(())
            }
            CampAction::Retreat => {
                if s.phase != CampPhase::Battle {
                    return Err(GameError::new("not in a battle"));
                }
                self.resolve_battle(s, false);
                Ok(())
            }
        }
    }

    fn outcome(&self, s: &CampaignState) -> Option<Outcome> {
        s.outcome.clone()
    }

    fn suggest(&self, s: &CampaignState) -> Option<CampAction> {
        self.guide(s)
    }

    fn is_suggested(&self, s: &CampaignState, a: &CampAction) -> bool {
        self.guide(s).as_ref() == Some(a)
    }

    fn view(&self, s: &CampaignState, perspective: Option<PlayerId>) -> TableView {
        // In a battle, show the embedded combat view.
        if let (CampPhase::Battle, Some(battle)) = (s.phase, &s.battle) {
            return Deckbound.view(battle, perspective);
        }
        // On the map: a tiled board with the token, fog, clear status, and the guide's next move.
        let suggested = self.guide(s);
        let legal = self.legal_actions(s);
        let mut tiles = Vec::new();
        for (i, loc) in s.run.locations.iter().enumerate() {
            // Tokens standing on this tile (general over any number of tokens).
            let tokens_here: Vec<String> = (0..s.tokens.len())
                .filter(|&ti| s.run.positions[ti] == i)
                .map(|ti| {
                    if s.tokens[ti].members.len() == s.party.len() {
                        "party".to_string()
                    } else {
                        s.tokens[ti]
                            .members
                            .iter()
                            .map(|&mi| s.party[mi].name.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                })
                .collect();
            let here = !tokens_here.is_empty();
            let revealed = s.run.revealed[i];
            let (label, sub) = if revealed {
                let status = if fully_cleared(s, i) {
                    "cleared".to_string()
                } else if s.run.cleared[i] > 0 {
                    format!("lvl {}/{}", s.run.cleared[i], loc.max_level)
                } else {
                    loc.currency
                        .role()
                        .unwrap_or_else(|| loc.currency.label())
                        .to_string()
                };
                (Some(loc.name.clone()), Some(status))
            } else {
                (None, None)
            };
            // Which legal action (if any) targets this tile, and is it the suggested one?
            let action = legal.iter().position(|act| match act {
                CampAction::Move(_, l) | CampAction::Enter(_, l) => *l == i,
                _ => false,
            });
            let is_suggested = matches!(&suggested, Some(CampAction::Move(_, l) | CampAction::Enter(_, l)) if *l == i);
            let accent = if is_suggested {
                Accent::Suggested
            } else if fully_cleared(s, i) {
                Accent::Good
            } else if here {
                Accent::Ally
            } else {
                Accent::Neutral
            };
            tiles.push(MapTile {
                col: loc.coord.col,
                row: loc.coord.row,
                label,
                sub,
                accent,
                action,
                tokens: tokens_here,
            });
        }
        // Per-track reward coverage: how many of each role track's five levels the party holds
        // (the §8.3 pool, replacing the old currency balances).
        let held = assigned_rewards(s);
        let cover: Vec<String> = Currency::TRACKS
            .iter()
            .map(|&t| {
                let n = held.iter().filter(|r| r.track == t).count();
                format!("{} {}/5", t.role().unwrap_or_else(|| t.label()), n)
            })
            .collect();
        // Surface the run's state and its latest event in the caption (the map has no log pane):
        // a clear victory banner when won, otherwise the day/coverage line plus the last log entry
        // (e.g. "Cleared the Iron Mire.") so travelling and fighting give feedback.
        let pending = if s.unassigned.is_empty() {
            String::new()
        } else {
            format!(" — {} reward(s) to assign", s.unassigned.len())
        };
        let day_line = format!("Day {} — {}{pending}", s.run.day + 1, cover.join(" · "));
        let status = match (&s.outcome, s.log.last()) {
            (Some(_), _) => format!(
                "Victory! The run is won in {} days.\n{day_line}",
                s.run.day + 1
            ),
            (None, Some(last)) => format!("{day_line}\n{last}"),
            (None, None) => day_line,
        };
        TableView {
            status,
            zones: Vec::new(),
            prose: Vec::new(),
            map: Some(MapView { hex: false, tiles }),
            // The world feed: clears, reward assignments, victory — the same side panel the battles
            // use, so the run reads as one running record.
            log: s.log.iter().rev().take(200).rev().cloned().collect(),
            focus: None,
        }
    }
}

/// Build the reference scenario as a playable campaign: a single token holding a 5-Novice party
/// (one per path), at the start of the A/B/C/final lattice.
pub fn reference_campaign() -> CampaignState {
    let paths = vec![
        Currency::Iron,
        Currency::Silver,
        Currency::Brass,
        Currency::Bone,
        Currency::Salt,
    ];
    let r = reference_scenario(&paths);
    let party: Vec<Member> = paths
        .iter()
        .map(|&p| Member {
            // Name a recruit by the combat role its track funds (Wall / Infiltrator / …) so the
            // party reads as roles; fall back to the track id for the generic Gold.
            name: format!("{} Recruit", p.role().unwrap_or(p.label())),
            base: "Novice".into(),
            track: p,
            rewards: Vec::new(),
        })
        .collect();
    let tokens = vec![Token {
        members: (0..party.len()).collect(),
        moved: false,
        acted: false,
    }];
    let script: Vec<usize> = (0..r.run.locations.len()).collect();
    CampaignState {
        run: r.run,
        party,
        tokens,
        encounters: r.encounters,
        phase: CampPhase::World,
        battle: None,
        pending: None,
        seed: 1,
        log: vec!["The reference run begins.".into()],
        outcome: None,
        goal: Goal::Objective,
        script,
        unassigned: Vec::new(),
    }
}

/// A **tutorial threat recipe** for a suit's ladder (§8.4): each suit's roster builds the *situation*
/// its powers are the answer to, so a location teaches its suit (§8.3). The location's **level** is the
/// difficulty dial (counts grow with it). One recipe per suit, cloned to that suit's five level cards.
///
/// - **Iron / Wall** — *hold the line*: an aggressive melee charge (`count = level` Husks) to intercept
///   and hold.
/// - **Silver / Infiltrator** — *slip & assassinate*: a dangerous ranged caster behind a thin guard;
///   slip past and kill it (a Seer + a growing Husk front).
/// - **Brass / Artillery** — *thin from range*: a **wide** front best cleared by AoE/ranged fire.
/// - **Bone / Controller** — *debuff the threat*: a fast raider, far easier once slowed / staggered.
/// - **Salt / Support** — *outlast*: an attrition fight (a fear caster + a chipping swarm) survived by
///   healing.
///
/// Counts are kept gentle so the accreting guided party (Iron fought weakest, Salt strongest) wins and
/// yields a par to **measure**; the precise difficulty gating is the balance harness's job to tune
/// (the `deckbound-balance` crate). `level` names the card and drives the per-level counts.
pub fn grind_encounter(suit: Currency, level: u32) -> EncounterCard {
    use crate::encounter::RosterEntry;
    let husks = |base, growth| RosterEntry {
        creature: "Husk".into(),
        from_level: 1,
        base,
        growth,
    };
    let solo = |creature: &str| RosterEntry {
        creature: creature.into(),
        from_level: 1,
        base: 1,
        growth: 0,
    };
    let foes = match suit {
        Currency::Iron => vec![husks(1, 1)], // L1:1 .. L5:5 — the charge to hold
        Currency::Silver => vec![solo("Seer"), husks(0, 1)], // Seer prize + L−1 guard Husks
        Currency::Brass => vec![husks(2, 1)], // L1:2 .. L5:6 — a wide front for AoE
        Currency::Bone => vec![solo("Raider"), husks(0, 1)], // a fast raider + L−1 Husks
        Currency::Salt => vec![solo("Seer"), husks(1, 1)], // fear caster + L chipping Husks
        Currency::Gold => vec![husks(1, 1)],
    };
    EncounterCard {
        name: format!("{} L{level}", suit.label()),
        currency: suit,
        strategy: "aggressor".into(),
        foes,
        scaling: crate::form::StatCard::default(),
    }
}

/// The playable **25-card grind base** (§8.1 / §8.3): the five suits × five levels on a seeded random
/// 5×5 grid, one party member specialising per suit, scored by **Days to acquire all treasure**
/// (`Goal::AllTreasure`). The guide grinds each suit's ladder L1→L5 in turn (cumulative unlock, so the
/// climb accretes that suit's rewards). First-cut difficulty: a per-suit swarm that scales with the
/// tier (`grind_encounter`).
pub fn grind_campaign(seed: u64) -> CampaignState {
    use crate::world::{REWARD_SUITS, base_grind_run, base_locations};

    let locations = base_locations(); // suit-major: Iron L1..L5, Silver L1..L5, …
    let encounters: Vec<EncounterCard> = locations
        .iter()
        .map(|l| grind_encounter(l.currency, l.max_level))
        .collect();

    let party: Vec<Member> = REWARD_SUITS
        .iter()
        .map(|&p| Member {
            name: format!("{} Recruit", p.role().unwrap_or(p.label())),
            base: "Novice".into(),
            track: p,
            rewards: Vec::new(),
        })
        .collect();
    let tokens = vec![Token {
        members: (0..party.len()).collect(),
        moved: false,
        acted: false,
    }];

    // The grind goal needs no single objective; nominate the last card so `run.objective` is valid.
    let objective = locations.len() - 1;
    let run = base_grind_run(seed, objective, 0, party.len());
    // Guide order: grind each suit's ladder in turn (suit-major == base_locations order).
    let script: Vec<usize> = (0..run.locations.len()).collect();

    CampaignState {
        run,
        party,
        tokens,
        encounters,
        phase: CampPhase::World,
        battle: None,
        pending: None,
        seed,
        log: vec!["The grind begins — acquire all treasure.".into()],
        outcome: None,
        goal: Goal::AllTreasure,
        script,
        unassigned: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guide_wins_the_reference_run() {
        // Following the guide's suggested action every step (Clash off) must clear the objective —
        // i.e. the scripted reference run is winnable as built. Counts the Days taken (the par).
        let game = Campaign;
        let mut s = game.new_game(0, 1);
        for _ in 0..10_000 {
            if game.outcome(&s).is_some() {
                break;
            }
            let action = game.suggest(&s).expect("the guide always has a next move");
            game.apply(&mut s, &action)
                .expect("the suggested action is legal");
        }
        assert!(
            matches!(game.outcome(&s), Some(Outcome::Win(PlayerId(0)))),
            "the guide failed to win the reference run; log tail: {:?}",
            &s.log[s.log.len().saturating_sub(6)..]
        );
        println!("reference run par (guided): {} days", s.run.day + 1);
    }

    #[test]
    fn the_grind_base_is_playable_and_winnable() {
        // The 25-card grind world is playable: following the guide acquires all 25 treasures
        // (every suit cleared to L5) and wins on `Goal::AllTreasure`. Reports the guided par (Days),
        // the first thing to measure — the optimal par and the balance properties come next.
        let game = Campaign;
        let mut s = grind_campaign(1);
        for _ in 0..100_000 {
            if game.outcome(&s).is_some() {
                break;
            }
            let action = game.suggest(&s).expect("the guide always has a next move");
            game.apply(&mut s, &action)
                .expect("the suggested action is legal");
        }
        assert!(
            matches!(game.outcome(&s), Some(Outcome::Win(PlayerId(0)))),
            "the guide failed to acquire all treasure; log tail: {:?}",
            &s.log[s.log.len().saturating_sub(8)..]
        );
        assert_eq!(s.run.unlocked().len(), 25, "all 25 rewards acquired");
        println!("grind base par (guided): {} days", s.run.day + 1);
    }

    #[test]
    fn tutorial_encounters_are_suit_specific() {
        // Each suit's roster builds its own lesson, not a generic swarm (§8.3 tutorials).
        let names = |suit| {
            grind_encounter(suit, 5)
                .roster(5)
                .into_iter()
                .map(|(n, _)| n)
                .collect::<Vec<_>>()
        };
        assert!(
            names(Currency::Silver).contains(&"Seer".to_string()),
            "Silver teaches slipping to assassinate a caster"
        );
        assert!(
            names(Currency::Bone).contains(&"Raider".to_string()),
            "Bone teaches debuffing a fast threat"
        );
        let total = |suit, lvl| {
            grind_encounter(suit, lvl)
                .roster(lvl)
                .into_iter()
                .map(|(_, c)| c)
                .sum::<u32>()
        };
        assert!(
            total(Currency::Brass, 5) > total(Currency::Iron, 5),
            "Brass fields a wider front than Iron — the AoE lesson"
        );
    }

    #[test]
    fn retreat_forfeits_the_battle_without_clearing() {
        // Retreat is the deliberate, consequential exit (token has acted, no clear) — distinct from
        // undoing the Enter. Start a fight, then retreat, and check the fold-back.
        let game = Campaign;
        let mut s = reference_campaign();
        let enter = game
            .legal_actions(&s)
            .into_iter()
            .find(|a| matches!(a, CampAction::Enter(..)))
            .or_else(|| {
                // Not standing on an enterable tile — move onto one first.
                let mv = game
                    .legal_actions(&s)
                    .into_iter()
                    .find(|a| matches!(a, CampAction::Move(..)))
                    .expect("a move is available");
                game.apply(&mut s, &mv).unwrap();
                game.legal_actions(&s)
                    .into_iter()
                    .find(|a| matches!(a, CampAction::Enter(..)))
            })
            .expect("a battle can be entered");
        let CampAction::Enter(_, loc) = enter else {
            unreachable!()
        };
        game.apply(&mut s, &enter).unwrap();
        assert_eq!(s.phase, CampPhase::Battle);
        let cleared_before = s.run.cleared[loc];

        // Retreat is offered alongside the combat actions, and folds the battle back as a forfeit.
        assert!(game.legal_actions(&s).contains(&CampAction::Retreat));
        game.apply(&mut s, &CampAction::Retreat).unwrap();
        assert_eq!(s.phase, CampPhase::World);
        assert!(s.battle.is_none());
        assert_eq!(
            s.run.cleared[loc], cleared_before,
            "a retreat clears nothing"
        );
        assert!(
            s.tokens[0].acted,
            "a retreat spends the token's action for the day"
        );
    }
}
