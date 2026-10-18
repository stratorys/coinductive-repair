#!/usr/bin/env -S uv run --script
"""Extract top-level public functions with primitive types from rustdoc JSON.

This minimal adapter fails on any signature it cannot translate. It does not
represent function bodies, methods or submodules.
"""

import json
import sys
from pathlib import Path


def primitive(rust_type):
    if not isinstance(rust_type, dict) or set(rust_type) != {"primitive"}:
        raise ValueError(f"unsupported type: {rust_type!r}")
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
            raise ValueError(f"unsupported generic function: {item['name']}")
        header = function["header"]
        if header["is_async"] or header["is_unsafe"] or header["abi"] != "Rust":
            raise ValueError(f"unsupported function header: {item['name']}")
        signature = function["sig"]
        if signature["is_c_variadic"]:
            raise ValueError(f"unsupported variadic function: {item['name']}")
        functions[item["name"]] = {
            "visibility": "public",
            "inputs": [primitive(t) for _, t in signature["inputs"]],
            "output": primitive(signature["output"]) if signature["output"] else "()",
        }
    return {"functions": functions}


if __name__ == "__main__":
    try:
        if len(sys.argv) != 2:
            raise ValueError("usage: normalize_rustdoc.py RUSTDOC.json")
        document = json.loads(Path(sys.argv[1]).read_text())
        print(json.dumps(normalize(document), indent=2, sort_keys=True))
    except (ValueError, KeyError, TypeError, OSError) as error:
        sys.exit(f"Cannot extract the API from rustdoc JSON: {error}")
