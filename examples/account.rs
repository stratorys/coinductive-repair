use coinductive_repair::account::{
    Account,
    Operation,
};

fn main() {
    let initial = 100;
    let mut account = Account::new(initial);
    let operations = [
        (Operation::Deposit, 50),
        (Operation::Withdraw, 40),
        (Operation::Withdraw, 200),
        (Operation::Deposit, 0),
        (Operation::Deposit, u32::MAX),
        (Operation::Withdraw, 108),
        (Operation::Withdraw, 106),
        (Operation::Deposit, u32::MAX),
        (Operation::Withdraw, u32::MAX),
        (Operation::Deposit, 1),
        (Operation::Withdraw, 0),
    ];

    println!("{{\"initial\":{initial},\"steps\":[");
    for (i, (operation, amount)) in operations.into_iter().enumerate() {
        let before = account.balance();
        let outcome = account.apply(operation, amount);
        let after = account.balance();
        let separator = if i == 0 { "" } else { "," };
        // Les chaînes proviennent exclusivement des enums fermées ci-dessus.
        println!(
            "{separator}{{\"operation\":\"{}\",\"amount\":{amount},\"before\":{before},\"after\":\
             {after},\"status\":\"{}\"}}",
            operation.as_str(),
            outcome.as_str()
        );
    }
    println!("],\"final\":{}}}", account.balance());
}
