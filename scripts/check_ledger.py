#!/usr/bin/env -S uv run --script
"""Check that CUE rejects corrupted copies of the ledger trace.

Each mutation introduces one defect a faulty ledger could produce. If CUE
accepts any of them, specs/ledger.cue is too weak and this script fails.
"""

import copy
import json
import subprocess
import tempfile
from pathlib import Path


def shift(trace, index, **deltas):
    """Add deltas to the balances after step `index` and keep the rest of the
    trace continuous, so that only the step itself is defective."""
    snapshots = [trace["steps"][index]["after"], trace["closing"]]
    for step in trace["steps"][index + 1 :]:
        snapshots += [step["before"], step["after"]]
    for balances in snapshots:
        for account, delta in deltas.items():
            balances[account] += delta


def accept(trace, index):
    """Rewrite a rejected step as if the ledger had accepted it."""
    step = trace["steps"][index]
    amount = step["amount_cents"]
    step.update(
        status="ok",
        entries=[
            {"account": step["from"], "direction": "debit", "amount_cents": amount},
            {"account": step["to"], "direction": "credit", "amount_cents": amount},
        ],
    )
    if step["from"] == step["to"]:
        return
    for account, delta in ((step["from"], -amount), (step["to"], amount)):
        if account in step["after"]:
            shift(trace, index, **{account: delta})
        else:
            # An undeclared account can only appear in this snapshot.
            step["after"][account] = delta


def main():
    source = Path("contracts/ledger.json")
    trace = json.loads(source.read_text())
    invalid = []

    def mutation(name, change):
        candidate = copy.deepcopy(trace)
        change(candidate)
        invalid.append((name, candidate))

    # Step 14, the last one, moves 18,000.00 from tax_reserve to operating.
    mutation("credit not posted", lambda t: shift(t, 14, operating=-1800000))
    mutation(
        "credit posted to the wrong account",
        lambda t: shift(t, 14, operating=-1800000, payroll=1800000),
    )
    # Step 3 provisions VAT: payroll and suppliers are not part of it. The
    # total is preserved.
    mutation(
        "transfer changes a third account",
        lambda t: shift(t, 3, payroll=-100, suppliers=100),
    )
    mutation(
        "unbalanced entries",
        lambda t: t["steps"][14]["entries"][1].update(amount_cents=1799999),
    )
    mutation("accepted transfer without entries", lambda t: t["steps"][14].update(entries=[]))
    # Step 1 retries the payroll transfer with the same id.
    mutation("duplicate transfer paid twice", lambda t: accept(t, 1))
    # Step 4 exceeds the per-transfer limit.
    mutation("transfer above limit accepted", lambda t: accept(t, 4))
    mutation(
        "rejected transfer moves money",
        lambda t: shift(t, 4, operating=1, suppliers=-1),
    )
    # Step 6 would take operating below its overdraft limit.
    mutation("overdraft limit exceeded", lambda t: accept(t, 6))
    # Step 9 transfers from operating to itself.
    mutation("transfer to the same account accepted", lambda t: accept(t, 9))
    # Step 10 targets an account that is not declared.
    mutation("transfer to an unknown account accepted", lambda t: accept(t, 10))
    # Step 11 transfers a zero amount.
    mutation("zero amount accepted", lambda t: accept(t, 11))

    def break_continuity(t):
        # Still valid on its own and conserves the total, but no longer starts
        # from the balances left by step 0.
        for balances in (t["steps"][1]["before"], t["steps"][1]["after"]):
            balances.update(operating=7499999, payroll=4500001)

    mutation("broken continuity between steps", break_continuity)
    mutation(
        "undeclared account in a snapshot",
        lambda t: t["steps"][2]["before"].update(escrow=0),
    )
    mutation(
        "account missing from a snapshot",
        lambda t: t["steps"][2]["before"].pop("tax_reserve"),
    )
    mutation("wrong closing balance", lambda t: t["closing"].update(operating=0))
    mutation("empty trace", lambda t: t.update(steps=[]))

    with tempfile.TemporaryDirectory(dir=source.parent) as directory:
        for name, candidate in invalid:
            path = Path(directory) / "invalid.json"
            path.write_text(json.dumps(candidate))
            result = subprocess.run(
                ["cue", "vet", "-c", "specs/ledger.cue", str(path)],
                capture_output=True,
                text=True,
            )
            if result.returncode != 1:
                raise SystemExit(
                    f"Expected CUE to reject '{name}', got exit code "
                    f"{result.returncode}: {result.stderr}"
                )
            print(f"Rejection confirmed: {name}")


if __name__ == "__main__":
    main()
