package spec

#Balance: int & >=0 & <=4294967295

#Step: {
	operation!: "deposit" | "withdraw"
	amount!:    #Balance
	before!:    #Balance
	after!:     #Balance
	status!:    "ok" | "zero_amount" | "insufficient_funds" | "overflow"

	if amount == 0 {
		status: "zero_amount"
		after:  before
	}
	if amount > 0 {
		if operation == "deposit" {
			if before+amount <= 4294967295 {
				status: "ok"
				after:  before + amount
			}
			if before+amount > 4294967295 {
				status: "overflow"
				after:  before
			}
		}
		if operation == "withdraw" {
			// Frais fixes de deux unités, seulement si le retrait est accepté.
			if amount+2 <= before {
				status: "ok"
				after:  before - amount - 2
			}
			if amount+2 > before {
				status: "insufficient_funds"
				after:  before
			}
		}
	}
}

initial!: #Balance
steps!: [#Step, ...#Step]
final!: #Balance & steps[len(steps)-1].after

// La trace doit décrire une seule exécution continue.
continuity: {
	for i, step in steps {
		"\(i)": {
			if i == 0 {
				valid: true & (step.before == initial)
			}
			if i > 0 {
				valid: true & (step.before == steps[i-1].after)
			}
		}
	}
}
