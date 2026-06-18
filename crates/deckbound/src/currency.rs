//! §8.3 — the **currency economy**: six typed currencies, and the **recompute invariant** — a
//! balance is *read off the table*, never a maintained meter (§2.1):
//!
//! > `balance(C) = (C on reachable treasure cards) − (C spent on owned Upgrades)`
//!
//! Earned sits on treasure cards; spent sits on the Upgrades you bought. Nothing is tracked.

use serde::Deserialize;

/// The six currencies (§8.3): one per combat role (the §4 triangle's five splits), plus a generic
/// for role-independent utility.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
pub enum Currency {
    /// Wall — hold the line.
    Iron,
    /// Infiltrator — slip and assassinate.
    Silver,
    /// Artillery — ranged damage.
    Brass,
    /// Controller — strip foes / Fear.
    Bone,
    /// Support — heal / ward / aid.
    Salt,
    /// Generic — role-independent utility.
    Gold,
}

impl Currency {
    /// All six, in canonical order (Gold last — the generic).
    pub const ALL: [Currency; 6] = [
        Currency::Iron,
        Currency::Silver,
        Currency::Brass,
        Currency::Bone,
        Currency::Salt,
        Currency::Gold,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Currency::Iron => "Iron",
            Currency::Silver => "Silver",
            Currency::Brass => "Brass",
            Currency::Bone => "Bone",
            Currency::Salt => "Salt",
            Currency::Gold => "Gold",
        }
    }

    /// The combat role this currency funds — `None` for the generic Gold (§8.5).
    pub fn role(self) -> Option<&'static str> {
        Some(match self {
            Currency::Iron => "Wall",
            Currency::Silver => "Infiltrator",
            Currency::Brass => "Artillery",
            Currency::Bone => "Controller",
            Currency::Salt => "Support",
            Currency::Gold => return None,
        })
    }
}

/// A typed amount of currency: a treasure card's payout, or an Upgrade's price.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct Coins {
    pub currency: Currency,
    pub amount: u32,
}

impl Coins {
    pub fn new(currency: Currency, amount: u32) -> Self {
        Self { currency, amount }
    }
}

/// The §8.3 recompute: a balance is `earned − spent` for one currency, summed off the table.
/// Signed because a build may have committed Upgrades whose treasure is not currently reachable
/// (co-location, §8.3) — a negative reads as "you owe more than you can currently reach."
pub fn balance(currency: Currency, earned: &[Coins], spent: &[Coins]) -> i64 {
    let sum = |v: &[Coins]| {
        v.iter()
            .filter(|c| c.currency == currency)
            .map(|c| c.amount as i64)
            .sum::<i64>()
    };
    sum(earned) - sum(spent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_currencies_five_roles_plus_generic() {
        assert_eq!(Currency::ALL.len(), 6);
        let roles: Vec<_> = Currency::ALL.iter().filter_map(|c| c.role()).collect();
        assert_eq!(roles.len(), 5); // five role currencies
        assert_eq!(Currency::Gold.role(), None); // the generic
    }

    #[test]
    fn balance_is_earned_minus_spent_per_currency() {
        let earned = [
            Coins::new(Currency::Iron, 3),
            Coins::new(Currency::Iron, 2),
            Coins::new(Currency::Salt, 4),
        ];
        let spent = [Coins::new(Currency::Iron, 4)];
        assert_eq!(balance(Currency::Iron, &earned, &spent), 1); // (3+2) − 4
        assert_eq!(balance(Currency::Salt, &earned, &spent), 4); // untouched
        assert_eq!(balance(Currency::Gold, &earned, &spent), 0); // none earned or spent
    }

    #[test]
    fn currencies_are_isolated() {
        // Spending Iron never touches Salt (hard per-role, §8.3).
        let earned = [Coins::new(Currency::Salt, 5)];
        let spent = [Coins::new(Currency::Iron, 5)];
        assert_eq!(balance(Currency::Salt, &earned, &spent), 5);
        assert_eq!(balance(Currency::Iron, &earned, &spent), -5);
    }
}
