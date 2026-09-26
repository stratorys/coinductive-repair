#!/usr/bin/env -S uv run --script
"""Vérifie que CUE rejette plusieurs traces bancaires corrompues."""

import copy
import json
import subprocess
import tempfile
from pathlib import Path


def main():
    source = Path("target/contracts/account.json")
    trace = json.loads(source.read_text())
    invalid = []

    def mutation(name, change):
        candidate = copy.deepcopy(trace)
        change(candidate)
        invalid.append((name, candidate))

    mutation("frais oubliés", lambda t: t["steps"][1].update(after=110))
    mutation("débit après refus", lambda t: t["steps"][2].update(after=106))
    mutation("mauvais statut", lambda t: t["steps"][4].update(status="ok"))
    mutation("solde final incorrect", lambda t: t.update(final=0))
    mutation("montant nul accepté", lambda t: t["steps"][3].update(status="ok"))
    mutation("trace vide", lambda t: t.update(steps=[]))

    def break_continuity(t):
        # Cette transition reste valide isolément, mais ne suit plus la précédente.
        t["steps"][1].update(before=151, after=109)

    mutation("rupture de continuité", break_continuity)

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
                    f"Rejet attendu pour {name}, code reçu {result.returncode}: "
                    f"{result.stderr}"
                )
            print(f"Rejet confirmé : {name}")


if __name__ == "__main__":
    main()
