package spec

import "list"

#TransferCentsMax: 10000000
#Int64Min:         -9223372036854775808
#Int64Max:         9223372036854775807

#Cents: int & >=#Int64Min & <=#Int64Max

#Account: {
	id!:                  string & !=""
	overdraft_cents_max!: #Cents & >=0
	opening_cents!:       #Cents & >=-overdraft_cents_max
}

accounts!: [#Account, ...#Account]
account_ids: [for account in accounts {account.id}] & list.UniqueItems()

_opening: {for account in accounts {(account.id): account.opening_cents}}
_floor: {for account in accounts {(account.id): -account.overdraft_cents_max}}
_total: list.Sum([for account in accounts {account.opening_cents}])

// Closed: a balance snapshot may not mention an undeclared account. Every
// declared account is read by the checks below, so none may be missing.
#Balances: {for account in accounts {(account.id): #Cents}}

#Entry: {
	account!:      string
	direction!:    "debit" | "credit"
	amount_cents!: #Cents & >0
}

#Step: {
	transfer_id!:  string & !=""
	from!:         string
	to!:           string
	amount_cents!: #Cents
	before!:       #Balances
	after!:        #Balances
	status!:       "ok" | "non_positive_amount" | "above_transfer_limit" | "unknown_account" | "same_account" | "duplicate" | "overdraft_limit" | "overflow"
	entries!: [...#Entry]
}

steps!: [#Step, ...#Step]
closing!: #Balances

transitions: {
	for i, step in steps {
		"\(i)": {
			let known = list.Contains(account_ids, step.from) && list.Contains(account_ids, step.to)

			// Idempotency: only an accepted transfer consumes its id.
			let replayed = [for j, previous in steps if j < i && previous.status == "ok" && previous.transfer_id == step.transfer_id {j}]

			// When several rules apply, the first one is reported.
			let expected = [
				if step.amount_cents <= 0 {"non_positive_amount"},
				if step.amount_cents > #TransferCentsMax {"above_transfer_limit"},
				if !known {"unknown_account"},
				if step.from == step.to {"same_account"},
				if len(replayed) > 0 {"duplicate"},
				if known if step.before[step.from]-step.amount_cents < _floor[step.from] {"overdraft_limit"},
				if known if step.before[step.to]+step.amount_cents > #Int64Max {"overflow"},
				"ok",
			][0]
			status: true & (step.status == expected)

			conserved: true & (list.Sum([for account in accounts {step.after[account.id]}]) == _total)

			continuity: {
				for account in accounts {
					if i == 0 {
						(account.id): true & (step.before[account.id] == _opening[account.id])
					}
					if i > 0 {
						(account.id): true & (step.before[account.id] == steps[i-1].after[account.id])
					}
				}
			}

			if step.status == "ok" {
				debited:          true & (step.after[step.from] == step.before[step.from]-step.amount_cents)
				credited:         true & (step.after[step.to] == step.before[step.to]+step.amount_cents)
				within_overdraft: true & (step.after[step.from] >= _floor[step.from])
				untouched: {
					for account in accounts if account.id != step.from && account.id != step.to {
						(account.id): true & (step.after[account.id] == step.before[account.id])
					}
				}
				// Double entry: one debit on the source, one credit of the same
				// amount on the destination, nothing else.
				entries: [
					{account: step.from, direction: "debit", amount_cents: step.amount_cents},
					{account: step.to, direction: "credit", amount_cents: step.amount_cents},
				] & step.entries
			}

			if step.status != "ok" {
				unchanged: {
					for account in accounts {
						(account.id): true & (step.after[account.id] == step.before[account.id])
					}
				}
				no_entries: true & (len(step.entries) == 0)
			}
		}
	}
}

final: {
	for account in accounts {
		(account.id): true & (closing[account.id] == steps[len(steps)-1].after[account.id])
	}
}
