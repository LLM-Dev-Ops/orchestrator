#!/usr/bin/env python3
"""Generate docs/TEST_CERTIFICATION.md from a libtest-JSON test event stream.

Reads the newline-delimited JSON emitted by `cargo nextest run --message-format libtest-json`
(or `cargo test -- -Z unstable-options --format json`) on stdin and writes a certification
document stamped with the provenance of the run it came from.

The document reports measurements only. It never asserts production readiness and never
carries a grade -- readiness is a human decision that may cite this artifact.

Exit status mirrors the suite: 0 when nothing failed, 1 when something did.
"""

import argparse
import json
import re
import sys
from datetime import datetime, timezone

UNATTRIBUTED = "(unattributed)"

# `panicked at crates/foo/src/bar.rs:1440:9:`
PANIC_SITE = re.compile(r"panicked at ([^\s:]+):(\d+):(\d+)")

# Machine-specific prefixes that must not reach the document.
REGISTRY_PREFIX = re.compile(r"^.*/registry/src/[^/]+/")
TOOLCHAIN_PREFIX = re.compile(r"^/rustc/[0-9a-f]+/")


def normalize_absolute_site(site):
    """Strip the machine-specific prefix from a dependency or toolchain path."""
    site = REGISTRY_PREFIX.sub("", site)
    return TOOLCHAIN_PREFIX.sub("rustc/", site)


def parse_events(stream):
    """Yield decoded JSON objects, skipping blank and non-JSON lines.

    Runners interleave human-readable progress with the JSON stream, so a line that does
    not decode is noise rather than an error.
    """
    for line in stream:
        line = line.strip()
        if not line or not line.startswith("{"):
            continue
        try:
            yield json.loads(line)
        except json.JSONDecodeError:
            continue


def split_test_name(name):
    """Split a nextest test name into (crate, test path).

    nextest names tests `<binary-id>$<test-path>`, where binary-id is `crate`,
    `crate::bin/name`, or `crate::integration-test-name`. Plain `cargo test` JSON has no
    binary-id, so those tests cannot be attributed to a crate.
    """
    if "$" not in name:
        return UNATTRIBUTED, name
    binary_id, _, test_path = name.partition("$")
    return binary_id.split("::")[0], test_path


class Results:
    def __init__(self):
        self.crates = {}
        self.failures = []

    def _bucket(self, crate):
        return self.crates.setdefault(
            crate, {"passed": 0, "failed": 0, "ignored": 0}
        )

    def record(self, event):
        if event.get("type") != "test":
            return
        outcome = event.get("event")
        if outcome not in ("ok", "failed", "ignored"):
            return  # `started` and timeout/retry chatter carry no verdict

        crate, test_path = split_test_name(event.get("name", ""))
        key = {"ok": "passed", "failed": "failed", "ignored": "ignored"}[outcome]
        self._bucket(crate)[key] += 1

        if outcome == "failed":
            self.failures.append(
                {
                    "crate": crate,
                    "test": test_path,
                    "site": self._panic_site(event),
                }
            )

    @staticmethod
    def _panic_site(event):
        blob = "\n".join(
            str(event.get(field, "")) for field in ("stdout", "stderr", "message")
        )
        sites = [
            f"{match.group(1)}:{match.group(2)}" for match in PANIC_SITE.finditer(blob)
        ]
        if not sites:
            return None
        # A repo-relative path names the assertion a reader can go fix. Absolute paths point
        # into the registry or toolchain and, worse, embed the machine's home directory --
        # which would make the document differ between a developer's box and CI.
        for site in sites:
            if not site.startswith("/"):
                return site
        return normalize_absolute_site(sites[0])

    def total(self, key):
        return sum(bucket[key] for bucket in self.crates.values())

    @property
    def verdict(self):
        failed = self.total("failed")
        return "PASS" if failed == 0 else f"FAIL ({failed} failing)"


MEASURED_HEADING = "## Results by crate"


def render_provenance(args, generated_at):
    return [
        "# Test Certification",
        "",
        "<!-- GENERATED FILE -- DO NOT EDIT BY HAND.",
        "     Produced by scripts/generate-test-certification.py from a test run's JSON output.",
        "     CI regenerates the measured sections below and fails if they differ from this file.",
        "     See docs/adr/ADR-0002-certification-document-integrity.md. -->",
        "",
        "## Provenance",
        "",
        "| Field | Value |",
        "|---|---|",
        f"| Commit | `{args.commit}` |",
        f"| CI run ID | `{args.run_id}` |",
        f"| CI run URL | {args.run_url} |",
        f"| Command | `{args.command}` |",
        f"| Toolchain | `{args.toolchain}` |",
        f"| Generated at (UTC) | `{generated_at}` |",
        "",
    ]


def render_measured(results):
    """Render the sections that are a pure function of the test run.

    Kept separate from provenance so CI can compare them across runs: provenance changes on
    every run by construction, the measurements must not change unless the suite did.
    """
    total_passed = results.total("passed")
    total_failed = results.total("failed")
    total_ignored = results.total("ignored")
    total = total_passed + total_failed + total_ignored

    lines = [
        MEASURED_HEADING,
        "",
        f"**Verdict:** `{results.verdict}`",
        "",
        "| Crate | Passed | Failed | Ignored | Total |",
        "|---|---:|---:|---:|---:|",
    ]

    for crate in sorted(results.crates):
        bucket = results.crates[crate]
        crate_total = bucket["passed"] + bucket["failed"] + bucket["ignored"]
        lines.append(
            f"| `{crate}` | {bucket['passed']} | {bucket['failed']} "
            f"| {bucket['ignored']} | {crate_total} |"
        )

    lines += [
        f"| **Total** | **{total_passed}** | **{total_failed}** "
        f"| **{total_ignored}** | **{total}** |",
        "",
        "## Failures",
        "",
    ]

    if not results.failures:
        lines.append("None.")
    else:
        lines += ["| Test | Assertion site |", "|---|---|"]
        for failure in sorted(
            results.failures, key=lambda f: (f["crate"], f["test"])
        ):
            site = f"`{failure['site']}`" if failure["site"] else "_not reported_"
            lines.append(f"| `{failure['crate']}` :: `{failure['test']}` | {site} |")

    lines += [
        "",
        "---",
        "",
        "This document reports what the suite measured on the commit named above. It makes no",
        "claim about production readiness and carries no grade.",
        "",
    ]

    return "\n".join(lines)


def extract_measured(document):
    """Return the measured section of an existing certification document."""
    index = document.find(MEASURED_HEADING)
    if index == -1:
        return None
    return document[index:]


def main(argv=None, stdin=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--commit", required=True, help="Commit SHA the suite ran against")
    parser.add_argument("--run-id", required=True, help="CI run identifier")
    parser.add_argument("--run-url", required=True, help="URL of the CI run")
    parser.add_argument("--command", required=True, help="Exact command that produced the input")
    parser.add_argument("--toolchain", default="unknown", help="Toolchain version string")
    parser.add_argument(
        "--output",
        default="docs/TEST_CERTIFICATION.md",
        help="Path to write; '-' writes to stdout",
    )
    parser.add_argument(
        "--check",
        metavar="PATH",
        help=(
            "Compare the measured sections against an existing document instead of writing. "
            "Exits 2 if they differ, which means the file was hand-edited or is stale."
        ),
    )
    args = parser.parse_args(argv)

    results = Results()
    for event in parse_events(stdin or sys.stdin):
        results.record(event)

    generated_at = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    document = "\n".join(
        render_provenance(args, generated_at) + [render_measured(results)]
    )

    if args.check:
        with open(args.check, encoding="utf-8") as handle:
            existing = extract_measured(handle.read())
        fresh = render_measured(results)
        if existing is None or existing.strip() != fresh.strip():
            print(
                f"{args.check} does not match this run's measurements.\n"
                "Regenerate it with scripts/generate-test-certification.py rather than "
                "editing it by hand. See docs/adr/ADR-0002-certification-document-integrity.md.",
                file=sys.stderr,
            )
            return 2
        print(f"{args.check} matches: {results.verdict}", file=sys.stderr)
    elif args.output == "-":
        sys.stdout.write(document)
    else:
        with open(args.output, "w", encoding="utf-8") as handle:
            handle.write(document)

    print(results.verdict, file=sys.stderr)
    return 0 if results.total("failed") == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
