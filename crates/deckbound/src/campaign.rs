//! §8 — the playable **Campaign**: a world map you move on, entering locations to fight the §4
//! battle, buying Upgrades with earned currency, toward a run victory (clear the objective).
//!
//! An `engine::Game` that **embeds** the combat game ([`battle_state`]) and folds its result back
//! into the run. A `suggest()` **guide** walks the reference scenario's scripted path
//! (A → B[p] → C[p] → final), so the UI can highlight the next on-script move and detect deviation.
//!
//! Tokens are general (each holds ≥1 member): this scenario uses **one token = the whole party**
//! (move/fight together); a future scenario can use one token per member (split up) with no rewrite.

use engine::{Accent, Game, GameError, MapTile, MapView, Outcome, PlayerId, TableView};

use crate::actor::Actor;
use crate::currency::{Coins, Currency, balance};
use crate::encounter::EncounterCard;
use crate::game::{self, Deckbound, battle_state};
use crate::reference::reference_scenario;
use crate::scenarios::{build_character, build_encounter_foes, upgrade_price, upgrades_for};
use crate::solver;
use crate::state::State;
use crate::world::Run;

/// A party member: a clean-slate character specialising in one path, plus its bought Upgrades.
#[derive(Clone, Debug)]
pub struct Member {
    pub name: String,
    pub base: String,
    pub path: Currency,
    pub upgrades: Vec<String>,
}

impl Member {
    fn actor(&self) -> Actor {
        let mut a = build_character(&self.base, &self.upgrades);
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
    /// The scripted location order the guide follows (indices into `run.locations`).
    pub script: Vec<usize>,
}

/// One step a player can take in the campaign.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CampAction {
    /// Move token `t` to adjacent location `loc` (flips it face-up).
    Move(usize, usize),
    /// Enter location `loc` with token `t` — fight to clear it (at the location's max level).
    Enter(usize, usize),
    /// Buy Upgrade `up` for member `m`.
    Buy(usize, String),
    /// End the Day — advance the clock; tokens refresh.
    EndDay,
    /// A combat action, delegated to the embedded battle.
    Battle(game::Action),
}

/// The campaign ruleset (holds no state).
#[derive(Clone, Copy, Debug, Default)]
pub struct Campaign;

// ---- economy (the §8.3 recompute, on demand) -----------------------------

fn earned(s: &CampaignState) -> Vec<Coins> {
    s.run.treasure()
}

fn spent(s: &CampaignState) -> Vec<Coins> {
    s.party
        .iter()
        .flat_map(|m| m.upgrades.iter().map(|u| upgrade_price(u)))
        .collect()
}

fn affordable(s: &CampaignState, price: Coins) -> bool {
    balance(price.currency, &earned(s), &spent(s)) >= price.amount as i64
}

fn fully_cleared(s: &CampaignState, loc: usize) -> bool {
    s.run.cleared[loc] >= s.run.locations[loc].max_level
}

/// The next affordable, unowned Upgrade for some member (matched to its path's currency).
fn next_upgrade(s: &CampaignState) -> Option<(usize, String)> {
    for (mi, m) in s.party.iter().enumerate() {
        for up in upgrades_for(m.path) {
            if !m.upgrades.contains(&up) && affordable(s, upgrade_price(&up)) {
                return Some((mi, up));
            }
        }
    }
    None
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
    /// Fold a finished battle back into the run: record the clear or the retreat.
    fn resolve_battle(&self, s: &mut CampaignState, won: bool) {
        if let Some((t, loc)) = s.pending.take() {
            if won {
                let max = s.run.locations[loc].max_level;
                s.run.record_clear(loc, max);
                s.log
                    .push(format!("Cleared {}.", s.run.locations[loc].name));
            } else {
                s.log
                    .push(format!("Retreated from {}.", s.run.locations[loc].name));
            }
            s.tokens[t].acted = true;
        }
        s.battle = None;
        s.phase = CampPhase::World;
        if s.run.is_won() {
            s.outcome = Some(Outcome::Win(PlayerId(0)));
            s.log
                .push("The objective is cleared — the run is won!".into());
        }
    }

    /// The guide's recommended next action (the scripted A → B → C → final path).
    fn guide(&self, s: &CampaignState) -> Option<CampAction> {
        if s.phase == CampPhase::Battle {
            let battle = s.battle.as_ref()?;
            let actions = Deckbound.legal_actions(battle);
            return Some(CampAction::Battle(solver::greedy(battle, &actions)));
        }
        // Buy what we can afford first (so a freshly-cleared path funds its Upgrades).
        if let Some((m, up)) = next_upgrade(s) {
            return Some(CampAction::Buy(m, up));
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
            return Deckbound
                .legal_actions(battle)
                .into_iter()
                .map(CampAction::Battle)
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
        // Buy any affordable, unowned Upgrade.
        for (mi, m) in s.party.iter().enumerate() {
            for up in upgrades_for(m.path) {
                if !m.upgrades.contains(&up) && affordable(s, upgrade_price(&up)) {
                    a.push(CampAction::Buy(mi, up));
                }
            }
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
            CampAction::Buy(m, up) => format!("{}: buy {up}", s.party[*m].name),
            CampAction::EndDay => "End the day".into(),
            CampAction::Battle(b) => Deckbound.action_label(s.battle.as_ref().unwrap(), b),
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
            CampAction::Buy(m, up) => {
                let price = upgrade_price(up);
                if !affordable(s, price) {
                    return Err(GameError::new("can't afford that"));
                }
                s.party[*m].upgrades.push(up.clone());
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
                    loc.currency.label().to_string()
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
        let bal: Vec<String> = Currency::ALL
            .iter()
            .map(|&c| format!("{} {}", balance(c, &earned(s), &spent(s)), c.label()))
            .collect();
        TableView {
            status: format!("Day {} — {}", s.run.day + 1, bal.join(" · ")),
            zones: Vec::new(),
            prose: Vec::new(),
            map: Some(MapView { hex: false, tiles }),
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
            name: format!("{} Recruit", p.label()),
            base: "Novice".into(),
            path: p,
            upgrades: Vec::new(),
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
        script,
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
}
