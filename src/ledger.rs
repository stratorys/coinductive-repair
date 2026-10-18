//! Double-entry ledger: transfers between accounts with idempotency keys, a
//! per-transfer limit and a per-account overdraft limit.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

/// Largest amount a single transfer may move, in cents (100,000.00).
pub const TRANSFER_CENTS_MAX: i64 = 10_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AccountId(&'static str);

impl AccountId {
    pub const fn new(id: &'static str) -> Self { Self(id) }

    pub fn as_str(self) -> &'static str { self.0 }
}

/// Idempotency key: a transfer accepted once is never applied again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TransferId(&'static str);

impl TransferId {
    pub const fn new(id: &'static str) -> Self { Self(id) }

    pub fn as_str(self) -> &'static str { self.0 }
}

#[derive(Clone, Copy, Debug)]
pub struct AccountConfig {
    pub id: AccountId,
    pub opening_cents: i64,
    /// How far below zero the balance may go, as a non-negative amount.
    pub overdraft_cents_max: i64,
}

#[derive(Clone, Copy, Debug)]
pub struct Transfer {
    pub id: TransferId,
    pub from: AccountId,
    pub to: AccountId,
    pub amount_cents: i64,
}

/// Side of an entry, from the account holder's point of view: a debit takes
/// money out of the account, a credit puts money in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Debit,
    Credit,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debit => "debit",
            Self::Credit => "credit",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub account: AccountId,
    pub direction: Direction,
    pub amount_cents: i64,
}

/// The two balanced entries recorded for an accepted transfer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Posting {
    pub debit: Entry,
    pub credit: Entry,
}

/// Reason a transfer was refused. When several apply, the first one in
/// declaration order is reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    NonPositiveAmount,
    AboveTransferLimit,
    UnknownAccount,
    SameAccount,
    Duplicate,
    OverdraftLimit,
    Overflow,
}

impl Rejection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NonPositiveAmount => "non_positive_amount",
            Self::AboveTransferLimit => "above_transfer_limit",
            Self::UnknownAccount => "unknown_account",
            Self::SameAccount => "same_account",
            Self::Duplicate => "duplicate",
            Self::OverdraftLimit => "overdraft_limit",
            Self::Overflow => "overflow",
        }
    }
}

impl fmt::Display for Rejection {
    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let message = match self {
            Self::NonPositiveAmount => "transfer amount must be positive",
            Self::AboveTransferLimit => "transfer amount exceeds the per-transfer limit",
            Self::UnknownAccount => "transfer references an unknown account",
            Self::SameAccount => "transfer source and destination are the same account",
            Self::Duplicate => "transfer id was already accepted",
            Self::OverdraftLimit => "transfer would exceed the source overdraft limit",
            Self::Overflow => "transfer would overflow the destination balance",
        };
        formatter.write_str(message)
    }
}

impl Error for Rejection {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgerError {
    DuplicateAccount {
        account: AccountId,
    },
    NegativeOverdraft {
        account: AccountId,
        overdraft_cents_max: i64,
    },
    OpeningBelowOverdraft {
        account: AccountId,
        opening_cents: i64,
        overdraft_cents_max: i64,
    },
}

impl fmt::Display for LedgerError {
    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::DuplicateAccount {
                account,
            } => write!(formatter, "account {} is declared twice", account.as_str()),
            Self::NegativeOverdraft {
                account,
                overdraft_cents_max,
            } => write!(
                formatter,
                "account {} has a negative overdraft limit of {overdraft_cents_max} cents",
                account.as_str()
            ),
            Self::OpeningBelowOverdraft {
                account,
                opening_cents,
                overdraft_cents_max,
            } => write!(
                formatter,
                "account {} opens at {opening_cents} cents, below its overdraft limit of \
                 {overdraft_cents_max} cents",
                account.as_str()
            ),
        }
    }
}

impl Error for LedgerError {}

#[derive(Debug)]
struct AccountState {
    id: AccountId,
    balance_cents: i64,
    overdraft_cents_max: i64,
}

impl AccountState {
    fn floor_cents(&self) -> i64 { self.overdraft_cents_max.saturating_neg() }
}

#[derive(Debug)]
pub struct Ledger {
    accounts: Vec<AccountState>,
    // Grows with every accepted transfer; a production ledger would bound it
    // with a retention window.
    accepted_transfers: HashSet<TransferId>,
}

impl Ledger {
    pub fn new(configs: &[AccountConfig]) -> Result<Self, LedgerError> {
        let accounts = configs.iter().enumerate().try_fold(
            Vec::with_capacity(configs.len()),
            |mut accounts, (position, config)| {
                let account = config.id;
                let declared_before = configs
                    .iter()
                    .take(position)
                    .any(|other| other.id == account);
                if declared_before {
                    return Err(LedgerError::DuplicateAccount {
                        account,
                    });
                }
                if config.overdraft_cents_max < 0 {
                    return Err(LedgerError::NegativeOverdraft {
                        account,
                        overdraft_cents_max: config.overdraft_cents_max,
                    });
                }
                let state = AccountState {
                    id: account,
                    balance_cents: config.opening_cents,
                    overdraft_cents_max: config.overdraft_cents_max,
                };
                if state.balance_cents < state.floor_cents() {
                    return Err(LedgerError::OpeningBelowOverdraft {
                        account,
                        opening_cents: config.opening_cents,
                        overdraft_cents_max: config.overdraft_cents_max,
                    });
                }
                accounts.push(state);
                Ok(accounts)
            },
        )?;
        Ok(Self {
            accounts,
            accepted_transfers: HashSet::new(),
        })
    }

    /// Balances in declaration order.
    pub fn balances(&self) -> impl Iterator<Item = (AccountId, i64)> + '_ {
        self.accounts
            .iter()
            .map(|account| (account.id, account.balance_cents))
    }

    /// Applies a transfer atomically. A rejected transfer changes nothing and
    /// does not consume its id, so it can be retried.
    pub fn transfer(
        &mut self,
        transfer: &Transfer,
    ) -> Result<Posting, Rejection> {
        let amount_cents = transfer.amount_cents;
        if amount_cents <= 0 {
            return Err(Rejection::NonPositiveAmount);
        }
        if amount_cents > TRANSFER_CENTS_MAX {
            return Err(Rejection::AboveTransferLimit);
        }
        let (Some(source), Some(destination)) =
            (self.account(transfer.from), self.account(transfer.to))
        else {
            return Err(Rejection::UnknownAccount);
        };
        if source.id == destination.id {
            return Err(Rejection::SameAccount);
        }
        if self.accepted_transfers.contains(&transfer.id) {
            return Err(Rejection::Duplicate);
        }
        // An i64 underflow is necessarily below any floor.
        let debited_cents = source
            .balance_cents
            .checked_sub(amount_cents)
            .filter(|balance_cents| *balance_cents >= source.floor_cents())
            .ok_or(Rejection::OverdraftLimit)?;
        let credited_cents = destination
            .balance_cents
            .checked_add(amount_cents)
            .ok_or(Rejection::Overflow)?;

        for account in &mut self.accounts {
            if account.id == transfer.from {
                account.balance_cents = debited_cents;
            } else if account.id == transfer.to {
                account.balance_cents = credited_cents;
            }
        }
        self.accepted_transfers.insert(transfer.id);
        Ok(Posting {
            debit: Entry {
                account: transfer.from,
                direction: Direction::Debit,
                amount_cents,
            },
            credit: Entry {
                account: transfer.to,
                direction: Direction::Credit,
                amount_cents,
            },
        })
    }

    fn account(
        &self,
        id: AccountId,
    ) -> Option<&AccountState> {
        self.accounts.iter().find(|account| account.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AccountConfig,
        AccountId,
        Ledger,
        LedgerError,
    };

    const OPERATING: AccountId = AccountId::new("operating");
    const PAYROLL: AccountId = AccountId::new("payroll");

    fn config(
        id: AccountId,
        opening_cents: i64,
        overdraft_cents_max: i64,
    ) -> AccountConfig {
        AccountConfig {
            id,
            opening_cents,
            overdraft_cents_max,
        }
    }

    #[test]
    fn new_keeps_declaration_order() {
        let ledger = Ledger::new(&[config(PAYROLL, 0, 0), config(OPERATING, -500, 1_000)])
            .expect("valid configuration");
        let balances: Vec<_> = ledger.balances().collect();
        assert_eq!(balances, [(PAYROLL, 0), (OPERATING, -500)]);
    }

    #[test]
    fn new_rejects_duplicate_account() {
        let result = Ledger::new(&[config(OPERATING, 0, 0), config(OPERATING, 10, 0)]);
        assert_eq!(
            result.err(),
            Some(LedgerError::DuplicateAccount {
                account: OPERATING
            })
        );
    }

    #[test]
    fn new_rejects_negative_overdraft() {
        let result = Ledger::new(&[config(OPERATING, 0, -1)]);
        assert_eq!(
            result.err(),
            Some(LedgerError::NegativeOverdraft {
                account: OPERATING,
                overdraft_cents_max: -1,
            })
        );
    }

    #[test]
    fn new_rejects_opening_below_overdraft() {
        let result = Ledger::new(&[config(OPERATING, -1_001, 1_000)]);
        assert_eq!(
            result.err(),
            Some(LedgerError::OpeningBelowOverdraft {
                account: OPERATING,
                opening_cents: -1_001,
                overdraft_cents_max: 1_000,
            })
        );
    }

    #[test]
    fn new_accepts_opening_at_overdraft_limit() {
        let ledger = Ledger::new(&[config(OPERATING, -1_000, 1_000)]);
        assert!(ledger.is_ok());
    }
}
