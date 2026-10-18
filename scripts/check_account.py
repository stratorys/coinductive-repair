#!/usr/bin/env -S uv run --script
"""Check that CUE rejects several corrupted copies of the bank account trace.

Each mutation breaks exactly one rule of specs/account.cue. If CUE accepts any
of them, the spec is too weak and this script fails.
"""

import copy
import json
import subprocess
import tempfile
from pathlib import Path


def main():
    source = Path("contracts/account.json")
    trace = json.loads(source.read_text())
    invalid = []

    def mutation(name, change):
        candidate = copy.deepcopy(trace)
        change(candidate)
        invalid.append((name, candidate))

    # Step 1 is an accepted withdrawal of 40 from 150: the fee must be charged.
    mutation("withdrawal fee not charged", lambda t: t["steps"][1].update(after=110))
    # Step 2 is rejected for insufficient funds: the balance must not change.
    mutation(
        "balance debited after a rejection", lambda t: t["steps"][2].update(after=106)
    )
    # Step 4 deposits u32::MAX: it must be reported as an overflow.
    mutation("overflow reported as ok", lambda t: t["steps"][4].update(status="ok"))
    # The final balance must equal the balance after the last step.
    mutation("wrong final balance", lambda t: t.update(final=0))
    # Step 3 deposits 0: a zero amount must be rejected.
    mutation("zero amount accepted", lambda t: t["steps"][3].update(status="ok"))
    # A trace must contain at least one step.
    mutation("empty trace", lambda t: t.update(steps=[]))

    def break_continuity(t):
        # This step is still valid on its own, but no longer follows step 0.
        t["steps"][1].update(before=151, after=109)

    mutation("broken continuity between steps", break_continuity)

    with tempfile.TemporaryDirectory(dir=source.parent) as directory:
        for name, candidate in invalid:
            path = Path(directory) / "invalid.json"
            path.write_text(json.dumps(candidate))
            result = subprocess.run(
                ["cue", "vet", "-c", "specs/account.cue", str(path)],
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
