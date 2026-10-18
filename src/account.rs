//! Account with a fixed withdrawal fee and all-or-nothing operations.

/// Fee charged on every accepted withdrawal, on top of the amount.
pub const WITHDRAWAL_FEE: u32 = 2;

/// Kind of operation applied to an [`Account`].
#[derive(Clone, Copy, Debug)]
pub enum Operation {
    Deposit,
    Withdraw,
}

impl Operation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Deposit => "deposit",
            Self::Withdraw => "withdraw",
        }
    }
}

/// Result of [`Account::apply`]. Only [`Outcome::Ok`] changes the balance.
#[derive(Clone, Copy, Debug)]
pub enum Outcome {
    Ok,
    ZeroAmount,
    InsufficientFunds,
    Overflow,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::ZeroAmount => "zero_amount",
            Self::InsufficientFunds => "insufficient_funds",
            Self::Overflow => "overflow",
        }
    }
}

/// Account holding a balance that never goes below zero or above `u32::MAX`.
pub struct Account {
    balance: u32,
}

impl Account {
    pub fn new(balance: u32) -> Self {
        Self {
            balance,
        }
    }

    pub fn balance(&self) -> u32 { self.balance }

    /// Applies an operation and reports its outcome.
    ///
    /// An accepted withdrawal costs [`WITHDRAWAL_FEE`] extra units.
    /// A rejected operation leaves the balance unchanged and charges no fee.
    pub fn apply(
        &mut self,
        operation: Operation,
        amount: u32,
    ) -> Outcome {
        if amount == 0 {
            return Outcome::ZeroAmount;
        }
        let next = match operation {
            Operation::Deposit => match self.balance.checked_add(amount) {
                Some(next) => next,
                None => return Outcome::Overflow,
            },
            Operation::Withdraw => {
                // u64 keeps amount + fee from overflowing.
                let total = u64::from(amount) + u64::from(WITHDRAWAL_FEE);
                if total > u64::from(self.balance) {
                    return Outcome::InsufficientFunds;
                }
                self.balance - total as u32
            }
        };
        self.balance = next;
        Outcome::Ok
    }
}
