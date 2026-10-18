# Rust → JSON → CUE

Runnable example of validating a Rust API and its observed behaviour.

## How it works end to end

The `.cue` files are specifications: they describe what the code must do. The
`.json` files are observations: they describe what the code exposes or what it
actually produced. `cue vet` compares the two.

`scripts/check.sh` runs these steps in order:

1. Rustdoc analyses the library and writes `target/doc/coinductive_repair.json`.
   This JSON describes the API; it does not contain function bodies.
2. `uv run` executes the adapter that extracts the signature of `add` into
   `contracts/api.json`. CUE compares it with `specs/api.cue`.
3. `cargo run` actually calls `add` with 260 input pairs and writes the results
   to `contracts/observations.json`. CUE compares them with the equations in
   `specs/behavior.cue`.
4. `cargo run --example ledger` posts fifteen transfers between four accounts
   and writes `contracts/ledger.json`. CUE checks every transition against
   `specs/ledger.cue`: status, balances, double entries, conservation of money
   and continuity of the trace.
5. `uv run` executes the test that corrupts seventeen temporary copies of this
   trace. CUE must reject every copy. The copies are then deleted.

A rejected transfer is an expected result, for example when the source account
would exceed its overdraft limit. A rejection by CUE means the JSON contradicts
the specification. The "Rejection confirmed" lines mean that the injected
defects were detected, not that the original trace contains defects.

The three JSON files in `contracts/` are versioned so that observations can be
read and compared in Git. The script regenerates them on every run. Editing them
by hand does not fix Rust: change the code and rerun the script. The raw rustdoc
JSON stays in `target/`, which Git ignores.

## Running

Requirements: stable Rust, nightly Rust, uv and CUE (module at version 0.17.1).
Python scripts are only run through `uv run`.

```sh
rustup toolchain install nightly
bash scripts/check.sh
```

The script regenerates three versioned artefacts in `contracts/`:

- `api.json`: top-level public functions extracted from rustdoc;
- `observations.json`: actual inputs and results of the Rust function `add`;
- `ledger.json`: trace of fifteen transfers posted to a ledger.

The independent contracts are in `specs/`. A command fails as soon as an
extraction, an execution or a validation fails.

## What is checked

The API contract requires `pub fn add(u32, u32) -> u32`. The behavioural
contract requires every result to equal the sum of its inputs and to fit in a
u32. The program observes all 256 pairs of values from 0 to 15, then four u32
boundary cases. Inputs whose sum exceeds u32::MAX are outside the domain of this
contract; the function panics in that case, but CUE does not check this panic.

To observe a failure, change `add` to return a wrong value and rerun the script,
or change its signature to break the API contract. Required fields and the
non-empty list prevent a validation without data.

## Limits and extension

Rustdoc JSON describes the API, not the AST of function bodies. The Python
adapter is deliberately limited to top-level public functions that are not
generic, synchronous, safe and use primitive types. It ignores methods and
submodules, and refuses extracted function signatures it cannot translate. Add
explicit support for them to widen the analysis.

The rustdoc JSON format is unstable: pin a nightly version in CI and adapt the
extractor when upgrading. Expressions and branches need another extractor (for
example one based on `syn`).

CUE validates the observations it is given. This does not prove correctness for
every input. Extend the generator with business cases, random inputs or
transition traces to check more properties. Specs must express intent
independently of the implementation.

## Stateful example: double-entry ledger

The implementation is in `src/ledger.rs`, the scenario in `examples/ledger.rs`
and the independent contract in `specs/ledger.cue`.

```sh
cargo run --quiet --example ledger > contracts/ledger.json
cue vet -c specs/ledger.cue contracts/ledger.json
uv run --no-project scripts/check_ledger.py
```

The ledger holds the accounts of a small company, in cents. `operating` opens at
120,000.00 with an overdraft limit of 20,000.00; `payroll`, `suppliers` and
`tax_reserve` open at zero without overdraft. A transfer moves money from one
account to another and is either applied entirely or rejected without effect.
When several rules apply, the first one in this order is reported:

| Status | Rule |
| --- | --- |
| `non_positive_amount` | The amount is zero or negative. |
| `above_transfer_limit` | The amount exceeds 100,000.00. |
| `unknown_account` | The source or destination is not declared. |
| `same_account` | The source and destination are the same account. |
| `duplicate` | A transfer with the same id was already accepted. |
| `overdraft_limit` | The source would go below its overdraft limit. |
| `overflow` | The destination balance would exceed `i64::MAX`. |
| `ok` | The transfer is applied. |

The transfer id is an idempotency key: retrying an accepted transfer is
rejected as `duplicate`, while a rejected transfer does not consume its id and
can be retried. Excerpt of the trace (amounts in cents):

| Transfer | From → To | Amount | Status | Operating after |
| --- | --- | ---: | --- | ---: |
| PAY-2026-10 | operating → payroll | 4500000 | ok | 7500000 |
| PAY-2026-10 | operating → payroll | 4500000 | duplicate | 7500000 |
| SUP-0043 | operating → suppliers | 10000001 | above_transfer_limit | 2500000 |
| SUP-0043 | operating → suppliers | 4000000 | ok | -1500000 |
| SUP-0044 | operating → suppliers | 600000 | overdraft_limit | -1500000 |
| SUP-0044 | operating → suppliers | 500000 | ok | -2000000 |

For every step, CUE checks:

- the status, recomputed from the rules above and the earlier steps;
- for an accepted transfer: the source is debited and the destination credited
  by the amount, no other account changes, the source stays within its overdraft
  limit, and exactly one debit and one credit entry of that amount are recorded;
- for a rejected transfer: no balance changes and no entry is recorded;
- conservation: the sum of all balances equals the sum of opening balances;
- continuity: each step starts from the balances left by the previous one, and
  the closing balances equal those after the last step;
- every balance snapshot names exactly the declared accounts.

`scripts/check_ledger.py`, run through uv, injects seventeen defects into copies
of the trace and checks that CUE rejects each one: a credit not posted or posted
to the wrong account, a third account changed, unbalanced entries, a duplicate
paid twice, an exceeded overdraft, a rejected transfer that moves money, a
broken continuity, and so on. Defects are propagated through the rest of the
trace, so each copy stays continuous unless continuity is what is being broken.

Constructor errors (duplicate account, negative overdraft limit, opening below
the overdraft limit) are covered by Rust unit tests, since the trace only
contains valid ledgers.

The current rustdoc extraction remains limited to the top-level `add` API: the
types and methods of the ledger module are not normalised. This example extends
behavioural checking without extending the structural API contract.
