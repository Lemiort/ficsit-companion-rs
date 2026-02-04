#!/usr/bin/env python3
"""Compare two FCS save files (JSON) and print a readable diff summary.

Usage:
    python scripts/compare_saves.py old.fcs new.fcs

This script compares:
 - node counts and per-node field differences (by node index)
 - link sets (start/end pairs)

It prints a concise summary and detailed per-node diffs.
"""
import json
import sys
from typing import Any, Dict, List, Tuple


def load(path: str) -> Dict[str, Any]:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def keys_of(obj: Dict[str, Any]):
    return set(obj.keys())


def diff_values(a: Any, b: Any) -> Tuple[bool, str]:
    """Return (different, description)"""
    if a == b:
        return False, ""
    # Handle dicts
    if isinstance(a, dict) and isinstance(b, dict):
        ak, bk = set(a.keys()), set(b.keys())
        added = bk - ak
        removed = ak - bk
        changed = []
        for k in ak & bk:
            if a[k] != b[k]:
                changed.append(k)
        parts = []
        if added:
            parts.append(f"added keys: {sorted(added)}")
        if removed:
            parts.append(f"removed keys: {sorted(removed)}")
        if changed:
            parts.append(f"changed keys: {sorted(changed)}")
        return True, "; ".join(parts)

    # Lists
    if isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            return True, f"len {len(a)} -> {len(b)}"
        # shallow compare for contents
        for i, (x, y) in enumerate(zip(a, b)):
            if x != y:
                return True, f"index {i} differs ({x} -> {y})"
        return False, ""

    # fallback
    return True, f"{a} -> {b}"


def compare_nodes(old_nodes: List[Dict[str, Any]], new_nodes: List[Dict[str, Any]]):
    print(f"Nodes: old={len(old_nodes)} new={len(new_nodes)}")

    max_n = max(len(old_nodes), len(new_nodes))
    diffs = []
    for i in range(max_n):
        if i >= len(old_nodes):
            diffs.append((i, "added", new_nodes[i]))
            continue
        if i >= len(new_nodes):
            diffs.append((i, "removed", old_nodes[i]))
            continue
        a = old_nodes[i]
        b = new_nodes[i]
        # Compare common fields
        node_diff_lines = []
        # keys union
        for key in sorted(set(a.keys()) | set(b.keys())):
            va = a.get(key)
            vb = b.get(key)
            different, desc = diff_values(va, vb)
            if different:
                node_diff_lines.append((key, va, vb, desc))
        if node_diff_lines:
            diffs.append((i, "changed", node_diff_lines))

    # Output summary
    print(f"Nodes changed: {len([d for d in diffs if d[1]=='changed'])}")
    print(f"Nodes added:   {len([d for d in diffs if d[1]=='added'])}")
    print(f"Nodes removed: {len([d for d in diffs if d[1]=='removed'])}")
    print()

    # Detailed
    for idx, typ, payload in diffs:
        if typ == "added":
            print(f"Node #{idx}: ADDED")
            print(json.dumps(payload, indent=2))
            print()
        elif typ == "removed":
            print(f"Node #{idx}: REMOVED")
            print(json.dumps(payload, indent=2))
            print()
        else:
            print(f"Node #{idx}: CHANGED")
            for (k, va, vb, desc) in payload:
                print(f"  - {k}: {desc}")
                print(f"      old: {va}")
                print(f"      new: {vb}")
            print()


def link_tuple(l: Dict[str, Any]) -> Tuple[int, int, int, int]:
    # (start.node, start.pin, end.node, end.pin)
    s = l.get("start", {})
    e = l.get("end", {})
    return (s.get("node", -1), s.get("pin", -1), e.get("node", -1), e.get("pin", -1))


def compare_links(old_links: List[Dict[str, Any]], new_links: List[Dict[str, Any]]):
    old_set = set(link_tuple(l) for l in old_links)
    new_set = set(link_tuple(l) for l in new_links)
    removed = old_set - new_set
    added = new_set - old_set
    print(f"Links: old={len(old_links)} new={len(new_links)}")
    print(f"Links added: {len(added)}")
    for a in sorted(added):
        print(f"  + {a}")
    print(f"Links removed: {len(removed)}")
    for r in sorted(removed):
        print(f"  - {r}")
    print()


def main():
    if len(sys.argv) != 3:
        print("Usage: compare_saves.py old.fcs new.fcs")
        sys.exit(2)

    oldp = sys.argv[1]
    newp = sys.argv[2]

    old = load(oldp)
    new = load(newp)

    old_nodes = old.get("nodes", [])
    new_nodes = new.get("nodes", [])

    old_links = old.get("links", [])
    new_links = new.get("links", [])

    print(f"Comparing {oldp} -> {newp}")
    print("=" * 60)
    compare_nodes(old_nodes, new_nodes)
    compare_links(old_links, new_links)

    # Quick summary: check top-level differences
    top_keys = sorted(set(old.keys()) | set(new.keys()))
    print("Top-level keys differences:")
    for k in top_keys:
        if old.get(k) != new.get(k):
            print(f"  - {k}: {old.get(k)} -> {new.get(k)}")


if __name__ == "__main__":
    main()
