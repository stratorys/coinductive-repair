//! Compte avec frais fixes de retrait et opérations atomiques.

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

    /// Un retrait accepté coûte deux unités supplémentaires.
    /// Une opération refusée conserve le solde, sans frais.
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
                // u64 évite un dépassement lors du calcul montant + frais.
                let total = u64::from(amount) + 2;
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
