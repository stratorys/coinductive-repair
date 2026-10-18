use coinductive_repair::ledger::{
    AccountConfig,
    AccountId,
    Entry,
    Ledger,
    LedgerError,
    Transfer,
    TransferId,
};

const OPERATING: AccountId = AccountId::new("operating");
const PAYROLL: AccountId = AccountId::new("payroll");
const SUPPLIERS: AccountId = AccountId::new("suppliers");
const TAX_RESERVE: AccountId = AccountId::new("tax_reserve");
// Never declared in the ledger.
const ESCROW: AccountId = AccountId::new("escrow");

fn transfer(
    id: &'static str,
    from: AccountId,
    to: AccountId,
    amount_cents: i64,
) -> Transfer {
    Transfer {
        id: TransferId::new(id),
        from,
        to,
        amount_cents,
    }
}

fn balances_json(ledger: &Ledger) -> String {
    let fields: Vec<String> = ledger
        .balances()
        .map(|(account, balance_cents)| format!("\"{}\":{balance_cents}", account.as_str()))
        .collect();
    format!("{{{}}}", fields.join(","))
}

fn entry_json(entry: &Entry) -> String {
    format!(
        "{{\"account\":\"{}\",\"direction\":\"{}\",\"amount_cents\":{}}}",
        entry.account.as_str(),
        entry.direction.as_str(),
        entry.amount_cents
    )
}

fn main() -> Result<(), LedgerError> {
    let accounts = [
        AccountConfig {
            id: OPERATING,
            opening_cents: 12_000_000,
            overdraft_cents_max: 2_000_000,
        },
        AccountConfig {
            id: PAYROLL,
            opening_cents: 0,
            overdraft_cents_max: 0,
        },
        AccountConfig {
            id: SUPPLIERS,
            opening_cents: 0,
            overdraft_cents_max: 0,
        },
        AccountConfig {
            id: TAX_RESERVE,
            opening_cents: 0,
            overdraft_cents_max: 0,
        },
    ];
    let mut ledger = Ledger::new(&accounts)?;
    let transfers = [
        transfer("PAY-2026-10", OPERATING, PAYROLL, 4_500_000),
        // Network retry of the payroll run: must not be paid twice.
        transfer("PAY-2026-10", OPERATING, PAYROLL, 4_500_000),
        transfer("SUP-0042", OPERATING, SUPPLIERS, 3_200_000),
        transfer("VAT-2026-Q3", OPERATING, TAX_RESERVE, 1_800_000),
        transfer("SUP-0043", OPERATING, SUPPLIERS, 10_000_001),
        // Same id as the rejected transfer above, now within the limit.
        transfer("SUP-0043", OPERATING, SUPPLIERS, 4_000_000),
        transfer("SUP-0044", OPERATING, SUPPLIERS, 600_000),
        // Brings operating exactly to its overdraft limit.
        transfer("SUP-0044", OPERATING, SUPPLIERS, 500_000),
        transfer("TAX-REFUND", TAX_RESERVE, PAYROLL, 1_800_001),
        transfer("ADJ-0001", OPERATING, OPERATING, 100),
        transfer("ADJ-0002", OPERATING, ESCROW, 100),
        transfer("ADJ-0003", PAYROLL, SUPPLIERS, 0),
        transfer("ADJ-0004", PAYROLL, SUPPLIERS, -500),
        transfer("RET-0001", SUPPLIERS, OPERATING, 2_000_000),
        transfer("TAX-0001", TAX_RESERVE, OPERATING, 1_800_000),
    ];

    // Identifiers only come from the string literals above: nothing to escape.
    let accounts_json: Vec<String> = accounts
        .iter()
        .map(|account| {
            format!(
                "{{\"id\":\"{}\",\"opening_cents\":{},\"overdraft_cents_max\":{}}}",
                account.id.as_str(),
                account.opening_cents,
                account.overdraft_cents_max
            )
        })
        .collect();
    println!("{{\"accounts\":[{}],\"steps\":[", accounts_json.join(","));
    for (i, transfer) in transfers.iter().enumerate() {
        let before = balances_json(&ledger);
        let (status, entries) = match ledger.transfer(transfer) {
            Ok(posting) => (
                "ok",
                [entry_json(&posting.debit), entry_json(&posting.credit)].join(","),
            ),
            Err(rejection) => (rejection.as_str(), String::new()),
        };
        let after = balances_json(&ledger);
        let separator = if i == 0 { "" } else { "," };
        println!(
            "{separator}{{\"transfer_id\":\"{}\",\"from\":\"{}\",\"to\":\"{}\",\"amount_cents\":\
             {},\"before\":{before},\"after\":{after},\"status\":\"{status}\",\"entries\":\
             [{entries}]}}",
            transfer.id.as_str(),
            transfer.from.as_str(),
            transfer.to.as_str(),
            transfer.amount_cents
        );
    }
    println!("],\"closing\":{}}}", balances_json(&ledger));
    Ok(())
}
