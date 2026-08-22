#!/usr/bin/env python3
import argparse
import json
import re
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CATALOG = ROOT / "docs/offline-capabilities.json"
OUTPUT = ROOT / "docs/OFFLINE_CAPABILITIES.md"
ID = re.compile(r"^[a-z0-9]+(?:[.-][a-z0-9]+)*$")
STATUSES = {"complete", "partial", "planned", "excluded"}


def load_and_validate():
    data = json.loads(CATALOG.read_text(encoding="utf-8"))
    if not re.fullmatch(r"[0-9a-f]{40}", data.get("evidence_revision", "")):
        raise SystemExit("evidence_revision must be a full Git SHA")
    if not data.get("evidence_ci", "").startswith("https://github.com/"):
        raise SystemExit("evidence_ci must identify a GitHub Actions run")
    current_tip_ci = data.get("current_tip_ci")
    if current_tip_ci is not None:
        if not re.fullmatch(r"[0-9a-f]{40}", current_tip_ci.get("revision", "")):
            raise SystemExit("current_tip_ci revision must be a full Git SHA")
        if not current_tip_ci.get("url", "").startswith("https://github.com/"):
            raise SystemExit("current_tip_ci url must identify a GitHub Actions run")
        if current_tip_ci.get("conclusion") not in {"success", "failure", "cancelled"}:
            raise SystemExit("current_tip_ci conclusion must be explicit")
    rows = data["capabilities"]
    ids = [row["id"] for row in rows]
    if len(ids) != len(set(ids)):
        raise SystemExit("capability IDs must be unique")
    sources = list(ROOT.glob("src/*.rs")) + [ROOT / "tests/deployment_contract.rs"]
    evidence_text = "\n".join(path.read_text(encoding="utf-8") for path in sources)
    for row in rows:
        if not ID.fullmatch(row["id"]):
            raise SystemExit(f"unstable capability ID: {row['id']}")
        if row["status"] not in STATUSES:
            raise SystemExit(f"unknown status for {row['id']}")
        if not row.get("limits"):
            raise SystemExit(f"missing limits for {row['id']}")
        for relative in row["paths"]:
            if not (ROOT / relative).is_file():
                raise SystemExit(f"missing referenced path for {row['id']}: {relative}")
        for test in row["tests"]:
            if f"fn {test}" not in evidence_text:
                raise SystemExit(f"missing referenced test for {row['id']}: {test}")
        if row["status"] == "complete" and not row["tests"]:
            raise SystemExit(f"complete row lacks tests: {row['id']}")
    return data, rows


def render(data, rows):
    counts = Counter(row["status"] for row in rows)
    current_tip_ci = data.get("current_tip_ci")
    current_tip_line = "Current remediation exact-tip CI: not yet recorded."
    if current_tip_ci is not None:
        current_tip_line = (
            "Current remediation exact-tip CI: "
            f"`{current_tip_ci['revision']}` at {current_tip_ci['url']} "
            f"({current_tip_ci['conclusion']})."
        )
    lines = ["# Offline capability catalog", "", "Generated from `docs/offline-capabilities.json`; do not edit by hand.", "No parity score is claimed. Statuses describe evidence, not UI presence.", f"Baseline evidence revision: `{data['evidence_revision']}`.", f"Baseline CI: {data['evidence_ci']}.", current_tip_line, "", "| Status | Count |", "| --- | ---: |"]
    for status in ("complete", "partial", "planned", "excluded"):
        lines.append(f"| {status.title()} | {counts[status]} |")
    lines += ["", "| ID | Capability | Status | Known limit |", "| --- | --- | --- | --- |"]
    for row in rows:
        limit = row["limits"].replace("|", "\\|")
        lines.append(f"| `{row['id']}` | {row['title']} | {row['status'].title()} | {limit} |")
    lines += ["", "## Status definitions", ""]
    for status, definition in data["status_definitions"].items():
        lines.append(f"- **{status.title()}:** {definition}")
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data, rows = load_and_validate()
    generated = render(data, rows)
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text(encoding="utf-8") != generated:
            raise SystemExit("generated capability document is stale")
    else:
        OUTPUT.write_text(generated, encoding="utf-8")


if __name__ == "__main__":
    main()
