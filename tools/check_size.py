#!/usr/bin/env python3
"""Fail if a source or doc file exceeds MAX_LINES lines (rule: docs/contributing/file-size-rule.md)."""
import pathlib
import sys

MAX_LINES = 300
EXTENSIONS = {".rs", ".go", ".ts", ".vue", ".js", ".mjs", ".py", ".md", ".toml", ".yml", ".yaml", ".json"}
# gen: Tauri schemas generated at build time; dist-desktop: web build output.
SKIP_DIRS = {".git", "target", "node_modules", "dist", "dist-desktop", "gen", ".nuxt", ".output", "vendor"}
SKIP_NAMES = {"Cargo.lock", "package-lock.json", "go.sum"}
# ADR files may exceed the limit only if listed here (keep empty).
ALLOWED: set[str] = set()


def offenders(root: pathlib.Path):
    for path in root.rglob("*"):
        if not path.is_file() or path.suffix not in EXTENSIONS:
            continue
        if path.name in SKIP_NAMES or SKIP_DIRS & set(path.parts):
            continue
        rel = path.relative_to(root).as_posix()
        if rel in ALLOWED:
            continue
        with path.open("rb") as handle:
            lines = sum(1 for _ in handle)
        if lines > MAX_LINES:
            yield rel, lines


def main() -> int:
    root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
    bad = sorted(offenders(root))
    for rel, lines in bad:
        print(f"{rel}: {lines} lines (max {MAX_LINES})")
    if bad:
        print(f"{len(bad)} file(s) over the limit")
        return 1
    print("check_size: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
