package spec

#U32Max:        4294967295
#WithdrawalFee: 2

#Balance: int & >=0 & <=#U32Max

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
			if before+amount <= #U32Max {
				status: "ok"
				after:  before + amount
			}
			if before+amount > #U32Max {
				status: "overflow"
				after:  before
			}
		}
		if operation == "withdraw" {
			// The fee is only charged when the withdrawal is accepted.
			if amount+#WithdrawalFee <= before {
				status: "ok"
				after:  before - amount - #WithdrawalFee
			}
			if amount+#WithdrawalFee > before {
				status: "insufficient_funds"
				after:  before
			}
		}
	}
}

initial!: #Balance
steps!: [#Step, ...#Step]
final!: #Balance & steps[len(steps)-1].after

// The trace must describe a single continuous run: each step starts from the
// balance left by the previous one.
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
