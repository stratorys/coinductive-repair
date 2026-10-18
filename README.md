# coinductive-repair

Check what Rust code **exposes** and what it **actually does** against
independent [CUE](https://cuelang.org) specifications, with JSON as the bridge
between them.

Tests usually live next to the code, in the same language, and are often
written by the same person at the same time. This repository explores another
approach: the expected behaviour is written as CUE specs that know nothing
about the implementation. The Rust code is inspected or run, what it exposes
or produces is written out as JSON, and `cue vet` checks that JSON against the
specs. If the code drifts away from the spec, the check fails.

This is a small, runnable proof of concept, not a library.

## How it works

```text
                 inspect / run                    cue vet
  Rust code  ─────────────────►  JSON observations  ◄──────────  CUE specs
  src/, examples/                contracts/*.json                specs/*.cue
```

- **Specs** (`specs/*.cue`) describe what the code must do. They are written
  independently of the implementation.
- **Observations** (`contracts/*.json`) record what the code exposes or what it
  actually produced when it ran.
- **`cue vet`** compares the two and fails on any mismatch.

## Quick start

Prerequisites:

- [Rust](https://rustup.rs) stable **and** nightly (nightly is needed for
  rustdoc's JSON output)
- [uv](https://docs.astral.sh/uv/) to run the Python helper scripts
- [CUE](https://cuelang.org/docs/introduction/installation/) v0.17.1

```sh
rustup toolchain install nightly
bash scripts/check.sh
```

Expected output (Cargo build lines omitted):

```text
==> 1/3 API shape: rustdoc JSON -> contracts/api.json -> specs/api.cue
==> 2/3 Behaviour of add: contracts/observations.json -> specs/behavior.cue
==> 3/3 Bank account trace: contracts/account.json -> specs/account.cue
Rejection confirmed: withdrawal fee not charged
Rejection confirmed: balance debited after a rejection
Rejection confirmed: overflow reported as ok
Rejection confirmed: wrong final balance
Rejection confirmed: zero amount accepted
Rejection confirmed: empty trace
Rejection confirmed: broken continuity between steps
All checks passed: the API and observed behaviour match the CUE specs.
```

How to read it:

- Each `Rejection confirmed` line is **good news**. It means a deliberately
  corrupted copy of the trace was caught by the spec (see
  [mutation testing](#mutation-testing) below).
- A rejected bank operation, such as a withdrawal without enough funds, is a
  normal result that the spec expects. It is not a failure.
- The script stops at the first extraction, run or validation that fails, and
  exits with a non-zero code.

## The three checks

### 1. API shape

```text
cargo rustdoc (JSON) → scripts/normalize_rustdoc.py → contracts/api.json → specs/api.cue
```

Rustdoc describes the crate's public API as JSON. A small Python adapter keeps
only the top-level public functions and their signatures. The spec then
requires `add` to keep exactly this shape:

```cue
functions!: {
	add!: {
		visibility!: "public"
		inputs!: ["u32", "u32"]
		output!: "u32"
	}
}
```

Fields ending in `!` are required, so a missing function fails the check.

### 2. Behaviour of a pure function

```text
src/main.rs → contracts/observations.json → specs/behavior.cue
```

`src/main.rs` really calls `add` on 260 input pairs: every pair from 0 to 15
(256 runs), plus 4 edge cases next to `u32::MAX`. Each output must equal the
sum of its inputs and fit in a `u32`:

```cue
#Run: {
	input!: {
		a!: #U32
		b!: #U32
	}
	// Only covers inputs whose sum fits in a u32.
	output!: #U32 & (input.a + input.b)
}

// At least one observation is required, so an empty file cannot pass.
runs!: [#Run, ...#Run]
```

Inputs whose sum exceeds `u32::MAX` are outside this contract. `add` panics on
them, and that panic is not checked by CUE.

### 3. Behaviour with state: a bank account

```text
examples/account.rs → contracts/account.json → specs/account.cue → scripts/check_account.py
```

The account in `src/account.rs` follows these rules:

- It starts with a balance of 100.
- A deposit adds the amount to the balance.
- An accepted withdrawal removes the amount **plus a fixed fee of 2**.
- A zero amount, a deposit that would go past `u32::MAX`, or a withdrawal
  without enough funds to cover amount + fee is **rejected**. The balance stays
  the same and no fee is charged.

`examples/account.rs` runs 11 operations in a row and records each one
(operation, amount, balance before and after, status). For example:

| Operation | Before | Amount | After | Status |
| --- | ---: | ---: | ---: | --- |
| deposit | 100 | 50 | 150 | `ok` |
| withdraw | 150 | 40 | 108 | `ok` |
| withdraw | 108 | 200 | 108 | `insufficient_funds` |
| deposit | 108 | 0 | 108 | `zero_amount` |
| deposit | 108 | 4294967295 | 108 | `overflow` |

The trace also includes a withdrawal of `u32::MAX`, which checks that
computing amount + fee does not itself overflow.

The spec checks three things:

- **Each step:** the status and resulting balance follow the rules above. For
  example, for withdrawals:

  ```cue
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
  ```

- **Continuity:** each step starts from the balance left by the previous one,
  and the first step starts from the initial balance.
- **Final balance:** it equals the balance after the last step.

#### Mutation testing

A spec that accepts everything would also "pass". To show that this one
actually catches errors, `scripts/check_account.py` makes 7 corrupted copies of
the trace, each breaking exactly one rule, and checks that CUE rejects every
one of them:

| Mutation | Rule it breaks |
| --- | --- |
| withdrawal fee not charged | an accepted withdrawal costs amount + fee |
| balance debited after a rejection | a rejected operation leaves the balance unchanged |
| overflow reported as ok | a deposit past `u32::MAX` must be reported as `overflow` |
| wrong final balance | the final balance equals the last step's balance |
| zero amount accepted | a zero amount must be rejected |
| empty trace | a trace has at least one step |
| broken continuity between steps | each step starts from the previous balance (the step is valid on its own) |

To run only this example:

```sh
cargo run --quiet --example account > contracts/account.json
cue vet -c specs/account.cue contracts/account.json
uv run --no-project scripts/check_account.py
```

## Repository layout

| Path | Role |
| --- | --- |
| `src/lib.rs` | The `add` function under test |
| `src/account.rs` | The bank account under test |
| `src/main.rs` | Calls `add` on many inputs and prints the results as JSON |
| `examples/account.rs` | Runs a sequence of account operations and prints the trace as JSON |
| `specs/*.cue` | The specifications, written independently of the code |
| `contracts/*.json` | Generated observations, checked against the specs |
| `scripts/check.sh` | Runs every step, in order |
| `scripts/normalize_rustdoc.py` | Reduces rustdoc JSON to the top-level public function signatures |
| `scripts/check_account.py` | Mutation test: checks that corrupted traces are rejected |
| `cue.mod/module.cue` | CUE module definition |

### Why the generated JSON is committed

The files in `contracts/` are regenerated on every run of `scripts/check.sh`.
They are committed so that changes in observed behaviour show up in Git
diffs and code review. Editing them by hand does not fix anything: change the
Rust code and run the script again. The raw rustdoc JSON stays in `target/`,
which Git ignores.

## Try breaking it

The quickest way to see what the checks are worth is to break the code on
purpose. Make one of these changes and run `bash scripts/check.sh`:

| Change | Check that fails |
| --- | --- |
| In `src/lib.rs`, make `add` return `a.wrapping_add(b).wrapping_add(1)` | 2, behaviour of `add` |
| In `src/lib.rs`, change `b: u32` to `b: u64` | 1, API shape |
| In `src/account.rs`, set `WITHDRAWAL_FEE` to `3` | 3, bank account trace |
| In `src/account.rs`, remove the `amount == 0` check in `apply` | 3, bank account trace |

Afterwards, restore the code and the regenerated JSON with
`git restore src contracts`.

## Limitations

- **Rustdoc JSON is unstable.** Its format changes between nightly releases.
  Pin a nightly version in CI and update `normalize_rustdoc.py` when you
  upgrade.
- **The API extractor is deliberately small.** It only handles top-level public
  functions that are non-generic, synchronous, safe, use the Rust ABI and take
  primitive types. It ignores methods and submodules, and fails loudly on any
  other signature rather than guessing. As a result, the bank account's API is
  not checked structurally; only its behaviour is.
- **Signatures, not bodies.** Rustdoc JSON does not describe function bodies.
  Checking expressions or branches would need another extractor, for example
  one based on [`syn`](https://crates.io/crates/syn).
- **Observations are not proofs.** CUE validates the observations it is given.
  Passing does not prove the code is correct for every input. To cover more
  ground, add business scenarios, random inputs or longer traces.
- **Keep specs independent.** A spec derived from the implementation only
  checks that the code does what it does. Write specs from the intended
  behaviour.

## License

Licensed under the [Mozilla Public License 2.0](LICENSE).
