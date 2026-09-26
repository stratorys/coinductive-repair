#!/usr/bin/env -S uv run --script
"""Extrait les fonctions publiques de premier niveau, à types primitifs.

Cet adaptateur minimal échoue sur les signatures qu'il ne sait pas traduire.
Il ne représente ni les corps de fonctions, ni les méthodes, ni les sous-modules.
"""

import json
import sys
from pathlib import Path


def primitive(rust_type):
    if not isinstance(rust_type, dict) or set(rust_type) != {"primitive"}:
        raise ValueError(f"Type non pris en charge : {rust_type!r}")
    return rust_type["primitive"]


def normalize(document):
    index = document["index"]
    root = index[str(document["root"])]
    functions = {}
    for item_id in root["inner"]["module"]["items"]:
        item = index[str(item_id)]
        if item["visibility"] != "public" or "function" not in item["inner"]:
            continue
        function = item["inner"]["function"]
        if function["generics"]["params"] or function["generics"]["where_predicates"]:
            raise ValueError(f"Fonction générique non prise en charge : {item['name']}")
        header = function["header"]
        if header["is_async"] or header["is_unsafe"] or header["abi"] != "Rust":
            raise ValueError(f"En-tête non pris en charge : {item['name']}")
        signature = function["sig"]
        if signature["is_c_variadic"]:
            raise ValueError(
                f"Fonction variadique non prise en charge : {item['name']}"
            )
        functions[item["name"]] = {
            "visibility": "public",
            "inputs": [primitive(t) for _, t in signature["inputs"]],
            "output": primitive(signature["output"]) if signature["output"] else "()",
        }
    return {"functions": functions}


if __name__ == "__main__":
    try:
        if len(sys.argv) != 2:
            raise ValueError("Usage : normalize_rustdoc.py RUSTDOC.json")
        document = json.loads(Path(sys.argv[1]).read_text())
        print(json.dumps(normalize(document), indent=2, sort_keys=True))
    except (ValueError, KeyError, TypeError, OSError) as error:
        sys.exit(f"Extraction rustdoc impossible : {error}")
