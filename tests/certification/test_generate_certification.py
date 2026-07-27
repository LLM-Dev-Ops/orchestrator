#!/usr/bin/env python3
"""Unit tests for scripts/generate-test-certification.py.

Run with: python3 -m unittest discover -s tests/certification

The generator is a trusted component -- a bug in it produces confidently wrong output with an
authentic-looking provenance stamp. These tests pin the property that matters most: it reports
bad news accurately.
"""

import importlib.util
import io
import re
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURES = Path(__file__).resolve().parent / "fixtures"

_spec = importlib.util.spec_from_file_location(
    "generate_test_certification",
    REPO_ROOT / "scripts" / "generate-test-certification.py",
)
generator = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(generator)

PROVENANCE_ARGS = [
    "--commit",
    "9766e44a0237db942643870eb444d63b8e6ef3ed",
    "--run-id",
    "1234567890",
    "--run-url",
    "https://github.com/LLM-Dev-Ops/orchestrator/actions/runs/1234567890",
    "--command",
    "cargo nextest run --all --message-format libtest-json",
    "--toolchain",
    "rustc 1.90.0",
]

# Vocabulary the ADR forbids the generated artifact from ever asserting.
FORBIDDEN = re.compile(
    r"production[ -]ready|certified|PLATINUM|\b[A-F][+-]? ?\(\d+/100\)|\d+/100",
    re.IGNORECASE,
)


def run_generator(fixture_name, extra_args=()):
    """Run the generator over a fixture, returning (exit_code, document)."""
    with tempfile.TemporaryDirectory() as tmp:
        output = Path(tmp) / "TEST_CERTIFICATION.md"
        argv = PROVENANCE_ARGS + ["--output", str(output)] + list(extra_args)
        with (FIXTURES / fixture_name).open(encoding="utf-8") as stream:
            code = generator.main(argv, stdin=stream)
        document = output.read_text(encoding="utf-8") if output.exists() else ""
    return code, document


class AllPassFixture(unittest.TestCase):
    def setUp(self):
        self.code, self.document = run_generator("core-all-pass.jsonl")

    def test_exit_code_is_zero(self):
        self.assertEqual(self.code, 0)

    def test_verdict_is_pass(self):
        self.assertIn("**Verdict:** `PASS`", self.document)

    def test_reports_198_passing(self):
        self.assertIn("| **Total** | **198** | **0** | **0** | **198** |", self.document)

    def test_failure_section_is_empty(self):
        self.assertIn("## Failures\n\nNone.", self.document)


class ThreeFailuresFixture(unittest.TestCase):
    """The exact run ADR-0002 was written against: 195 passed, 3 failed, 198 total."""

    def setUp(self):
        self.code, self.document = run_generator("core-three-failures.jsonl")

    def test_exit_code_is_nonzero(self):
        self.assertEqual(self.code, 1)

    def test_verdict_names_the_failure_count(self):
        self.assertIn("**Verdict:** `FAIL (3 failing)`", self.document)

    def test_totals_match_the_measured_run(self):
        self.assertIn("| **Total** | **195** | **3** | **0** | **198** |", self.document)

    def test_every_failing_test_is_named(self):
        for name in (
            "agents::dependency_resolver::tests::test_resolve_success",
            "agents::state_machine_agent::tests::test_no_change_transition",
            "agents::state_machine_agent::tests::test_transition_invalid",
        ):
            self.assertIn(name, self.document)

    def test_assertion_sites_are_extracted(self):
        self.assertIn(
            "crates/llm-orchestrator-core/src/agents/dependency_resolver.rs:1440",
            self.document,
        )
        self.assertIn(
            "crates/llm-orchestrator-core/src/agents/state_machine_agent.rs:904",
            self.document,
        )


class ProvenanceAndVocabulary(unittest.TestCase):
    def test_provenance_is_stamped(self):
        _, document = run_generator("core-all-pass.jsonl")
        self.assertIn("9766e44a0237db942643870eb444d63b8e6ef3ed", document)
        self.assertIn("1234567890", document)
        self.assertIn("cargo nextest run --all --message-format libtest-json", document)
        self.assertIn("rustc 1.90.0", document)

    def test_no_readiness_vocabulary_even_on_a_green_run(self):
        _, document = run_generator("core-all-pass.jsonl")
        offending = [
            line for line in document.splitlines() if FORBIDDEN.search(line)
        ]
        # The disclaimer explicitly disclaims readiness; it is the one permitted mention.
        offending = [line for line in offending if "makes no" not in line]
        self.assertEqual(offending, [])


class CheckMode(unittest.TestCase):
    def _write_certification(self, fixture):
        _, document = run_generator(fixture)
        handle = tempfile.NamedTemporaryFile(
            "w", suffix=".md", delete=False, encoding="utf-8"
        )
        handle.write(document)
        handle.close()
        return Path(handle.name)

    def test_matching_document_passes_check(self):
        path = self._write_certification("core-all-pass.jsonl")
        code, _ = run_generator("core-all-pass.jsonl", ["--check", str(path)])
        self.assertEqual(code, 0)
        path.unlink()

    def test_hand_edited_number_fails_check(self):
        path = self._write_certification("core-three-failures.jsonl")
        tampered = path.read_text(encoding="utf-8").replace(
            "**Verdict:** `FAIL (3 failing)`", "**Verdict:** `PASS`"
        )
        path.write_text(tampered, encoding="utf-8")
        code, _ = run_generator("core-three-failures.jsonl", ["--check", str(path)])
        self.assertEqual(code, 2)
        path.unlink()

    def test_stale_document_fails_check(self):
        """A green document left in place after the suite regressed must be caught."""
        path = self._write_certification("core-all-pass.jsonl")
        code, _ = run_generator("core-three-failures.jsonl", ["--check", str(path)])
        self.assertEqual(code, 2)
        path.unlink()


class NameParsing(unittest.TestCase):
    def test_binary_id_is_stripped_to_crate(self):
        self.assertEqual(
            generator.split_test_name("llm-orchestrator-core$agents::foo::test_bar"),
            ("llm-orchestrator-core", "agents::foo::test_bar"),
        )

    def test_non_lib_targets_attribute_to_their_crate(self):
        self.assertEqual(
            generator.split_test_name("agentics-contracts::integration_tests$tests::x"),
            ("agentics-contracts", "tests::x"),
        )

    def test_names_without_a_binary_id_are_unattributed(self):
        self.assertEqual(
            generator.split_test_name("tests::x"),
            (generator.UNATTRIBUTED, "tests::x"),
        )


class StreamRobustness(unittest.TestCase):
    def test_non_json_noise_is_skipped(self):
        stream = io.StringIO(
            'Starting 198 tests\n'
            '{"type":"test","event":"ok","name":"c$t1"}\n'
            'not json at all\n'
            '\n'
            '{"type":"test","event":"failed","name":"c$t2","stdout":"panicked at src/x.rs:9:1:"}\n'
        )
        results = generator.Results()
        for event in generator.parse_events(stream):
            results.record(event)
        self.assertEqual(results.total("passed"), 1)
        self.assertEqual(results.total("failed"), 1)
        self.assertEqual(results.verdict, "FAIL (1 failing)")
        self.assertEqual(results.failures[0]["site"], "src/x.rs:9")

    def test_started_events_do_not_count(self):
        stream = io.StringIO(
            '{"type":"test","event":"started","name":"c$t1"}\n'
            '{"type":"test","event":"ok","name":"c$t1"}\n'
            '{"type":"suite","event":"ok","passed":1,"failed":0}\n'
        )
        results = generator.Results()
        for event in generator.parse_events(stream):
            results.record(event)
        self.assertEqual(results.total("passed"), 1)
        self.assertEqual(results.verdict, "PASS")


if __name__ == "__main__":
    unittest.main()
