//! §4.5 — **Groups**: same-side Actors bound at form-up into one unit (one position, one shared
//! target, **distinct** Health). This module is the **mechanical core** of §4.5 — the asymmetries the
//! spec makes load-bearing, as pure, tested functions over a group's member values:
//!
//! - **Sum to block, weakest-link to slip** ([`block_bid`] / [`group_slips`]) — holding a line *pools*
//!   (shields abreast = one stronger wall), so a block **sums** member Tempo; a slip/evade does **not**
//!   pool (a sentry foils a crowd by catching *any one*), so a group slips only if **every** member
//!   strictly beats the attacker. ⇒ a group is a superb wall and a hopeless slipper.
//! - **Spillover in declared order** ([`spillover`]) — accumulated **single-target** damage applies
//!   point-by-point down the declared order, overflowing to the next member once the current is spent
//!   (a tank soaks for the squishies behind).
//! - **AoE hits every member** ([`aoe`]) — width can't whiff against a crowd and need not pick a
//!   victim; it lands on **all** at full value, bypassing the spillover queue (the price of clustering).
//! - **Acting costs one Tempo per member** ([`group_can_act`]) — a group acts only when **every**
//!   member can spend a Tempo card, so a big group is durable but tempo-hungry.
//! - **Hoard X** ([`hoard_bodies`]) — a swarm *authored as one card* is the group dialed to the
//!   extreme: **X one-Health bodies** (sums to block, can't slip, melts to AoE, loses an attack per
//!   body killed).
//!
//! **Integration status.** These are the §4.5 primitives, additive and order-independent (§1.9). The
//! live six-phase loop (`game`/`combat`) currently resolves each Actor as its own singleton — the
//! **§4.1 count-adaptive** floor (groups only surface once party size makes them meaningful, which the
//! present small/ungrouped scenarios never do). Wiring player-side group declaration + group-aware
//! resolution into the interactive loop (and roster expansion for `Hoard X` authoring) is the
//! remaining §4.5 step; it shifts combat balance, so it is sequenced with the deferred par-solver
//! balance pass. These primitives are what that integration composes.

/// **Sum to block** (§4.5): a group pools its members' Tempo bids into one summed hold. The bids are
/// `cards × Finesse` per member (the §3.1 contest currency).
pub fn block_bid(member_bids: &[u32]) -> u32 {
    member_bids.iter().sum()
}

/// **Weakest-link to slip / evade** (§4.5): a group avoids the strike only if **every** member's bid
/// **strictly exceeds** the attacker's (a tie lands, §3.1). An empty group cannot slip. Never *barred*
/// — if all members out-bid, the whole group slips, just at brutal cost (force-not-fiat).
pub fn group_slips(member_bids: &[u32], attacker_bid: u32) -> bool {
    !member_bids.is_empty() && member_bids.iter().all(|&b| b > attacker_bid)
}

/// **Spillover** (§4.5): apply `damage` to the group's members **point-by-point in declared order**,
/// overflowing to the next member once the current can no longer absorb it. Mutates `members_health`
/// (each a remaining-Health count, in declared order) and returns the **leftover** damage wasted past
/// the last member. This is the single-target queue; AoE uses [`aoe`] instead.
pub fn spillover(mut damage: u32, members_health: &mut [u32]) -> u32 {
    for h in members_health.iter_mut() {
        let absorbed = damage.min(*h);
        *h -= absorbed;
        damage -= absorbed;
        if damage == 0 {
            break;
        }
    }
    damage
}

/// **AoE hits every member** (§4.5): width strikes each body at full `value`, **bypassing** the
/// spillover queue (the standing risk of clustering). Saturating at 0 per body.
pub fn aoe(value: u32, members_health: &mut [u32]) {
    for h in members_health.iter_mut() {
        *h = h.saturating_sub(value);
    }
}

/// **Acting costs one Tempo per member** (§4.5): a group attacks / makes a contested defense only when
/// **every** member can spend a Tempo card (`tempo > 0`, matching the per-Actor strike/contest gate).
/// An empty group cannot act.
pub fn group_can_act(member_tempo: &[i32]) -> bool {
    !member_tempo.is_empty() && member_tempo.iter().all(|&t| t > 0)
}

/// **Hoard X** (§4.5): a swarm authored as **one card** expands to **X one-Health bodies** — a
/// built-in group of X. Returns the per-body remaining-Health vec (all 1). The swarm's group
/// properties then fall out of the functions above: sums to block, never slips (each tiny body must
/// win its own race), melts to AoE (X bodies hit), loses one attack per body killed.
pub fn hoard_bodies(x: u32) -> Vec<u32> {
    vec![1; x as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_sums_but_slip_is_weakest_link() {
        // Sum to block: three shields abreast pool into one wall.
        assert_eq!(block_bid(&[2, 3, 4]), 9);
        // Slip is weakest-link: the whole group slips only if EVERY member strictly beats the attacker.
        assert!(
            group_slips(&[4, 5, 6], 3),
            "all members beat 3 → the group slips"
        );
        assert!(
            !group_slips(&[4, 2, 6], 3),
            "one member (2) fails to beat 3 → the group is caught (weakest-link)"
        );
        assert!(
            !group_slips(&[3, 3, 3], 3),
            "a tie lands the strike (must strictly exceed, §3.1)"
        );
        assert!(!group_slips(&[], 0), "an empty group cannot slip");
        // A lone high-Tempo body is the slipper; a blob is the wall — the §4.5 sum-vs-min asymmetry.
        assert!(group_slips(&[7], 5), "a lone fast body slips");
    }

    #[test]
    fn spillover_overflows_in_declared_order() {
        // A tank (5) soaks for the squishies behind: 7 damage spends the tank, overflows 2 to the next.
        let mut hp = [5, 3, 4];
        let leftover = spillover(7, &mut hp);
        assert_eq!(
            hp,
            [0, 1, 4],
            "tank emptied, 2 overflowed to the second member"
        );
        assert_eq!(leftover, 0);
        // Overflow past the last member is wasted (not wrapped).
        let mut hp = [2, 2];
        assert_eq!(spillover(10, &mut hp), 6, "10 − 4 = 6 leftover");
        assert_eq!(hp, [0, 0]);
    }

    #[test]
    fn aoe_hits_every_member_bypassing_the_queue() {
        // Width lands on ALL at full value (not point-by-point) — the price of clustering.
        let mut hp = [5, 3, 4];
        aoe(2, &mut hp);
        assert_eq!(hp, [3, 1, 2], "every body takes the full 2");
    }

    #[test]
    fn group_acts_only_if_every_member_can_spend() {
        assert!(group_can_act(&[1, 2, 3]), "all have Tempo → the group acts");
        assert!(
            !group_can_act(&[1, 0, 3]),
            "one member is out of Tempo → the group cannot act (one Tempo per member)"
        );
        assert!(!group_can_act(&[]), "an empty group cannot act");
    }

    #[test]
    fn hoard_is_a_group_of_one_health_bodies() {
        // Hoard X authored as one card → X one-Health bodies (a swarm).
        let mut swarm = hoard_bodies(4);
        assert_eq!(swarm, vec![1, 1, 1, 1]);
        // Melts to AoE: one width-2 hit downs every body at once.
        aoe(2, &mut swarm);
        assert_eq!(swarm, vec![0, 0, 0, 0], "one AoE shreds the whole swarm");
        // Can essentially never slip: each tiny one-card body must win its own race.
        assert!(
            !group_slips(&[1, 1, 1, 1], 1),
            "a swarm cannot slip (each body is a hopeless weakest-link)"
        );
    }
}
