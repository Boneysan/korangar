#!/usr/bin/env python3
"""Run the encyclopedia review loop one unit at a time.

The tool finds, packages, validates, records, and commits. The model only reads
one packet and writes one draft. Every command prints the next command to run.
A line starting with STOP means end the session.

    status                     queue counts and the active unit
    queue [--package E1 ...]   rebuild queue units from loaded source (keeps state)
    e0                         check the evidence-label rules (the E0 unit)
    next [--tier strong]       activate the next unit and write its packet
    packet [ID]                rewrite the active unit's packet
    validate [ID]              mechanical checks on draft.json
    verify [ID]                write the verifier packet (run in a fresh session)
    verify [ID] --record       read verdict.json and pass or reject the draft
    accept [ID]                merge the draft, run exporters and checks
    commit [ID]                commit exactly the accepted unit's files
    block [ID] --reason TEXT   give up on a unit with a reason
    audit [ID] --pass|--fail   record a stronger model's spot audit
    citations                  report existing reviews whose quotes are not on their cited lines
    model TAG                  (a person) record the exact model tag used for attribution

Staging lives in korangar/.encyclopedia-staging/<unit-id>/ and is git-ignored.
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import importlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

from encyclopedia_checks import REVIEW_FILES, check_draft, check_sources
from encyclopedia_common import (
    DISPOSITIONS,
    EVIDENCE_STATES,
    JOURNAL,
    LEDGER,
    QUEUE,
    REVIEWED_STATES,
    ROOT,
    STAGING,
    read_json,
    source_lines,
    write_json,
)
from encyclopedia_packets import PACKET_LINE_LIMIT, TOOL_DRAFTED, build_packet, claims_of, tool_draft, verify_packet

PYTHON = sys.executable
OPEN_STATES = ("active", "validated", "verified")
MAX_ATTEMPTS = 2
AUDIT_EVERY = 10
VERIFY_NOT_REQUIRED = TOOL_DRAFTED | {"e0_check"}
TOOL_OWNED = {
    QUEUE.relative_to(ROOT).as_posix(),
    LEDGER.relative_to(ROOT).as_posix(),
    JOURNAL.relative_to(ROOT).as_posix(),
}
RUST_TEST = "versioned_reference_data_loads_and_reconciles_all_links"


def stop(message: str, code: int = 1) -> int:
    print(message)
    print("STOP. Do not run another command. Report the lines above to the person.")
    return code


def today() -> str:
    return dt.date.today().isoformat()


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


# Queue and ledger ----------------------------------------------------------

def load_queue() -> dict[str, Any]:
    if not QUEUE.is_file():
        raise SystemExit(stop("No queue yet. A person runs: python3 tools/encyclopedia_unit.py queue"))
    return read_json(QUEUE)


def save_queue(queue: dict[str, Any]) -> None:
    write_json(QUEUE, queue)


def find_unit(queue: dict[str, Any], unit_id: str | None) -> dict[str, Any]:
    if unit_id:
        for unit in queue["units"]:
            if unit["id"] == unit_id:
                return unit
        raise SystemExit(stop(f"No unit {unit_id!r} in the queue."))
    for unit in queue["units"]:
        if unit["state"] in OPEN_STATES or (unit["state"] == "done" and not unit.get("commit")):
            return unit
    raise SystemExit(stop("No active unit. Run: python3 tools/encyclopedia_unit.py next"))


def log(unit: dict[str, Any], event: str) -> None:
    unit.setdefault("history", []).append(f"{now()} {event}")


def ledger_update(queue: dict[str, Any], unit: dict[str, Any] | None, next_action: str, checks: list[str] | None = None) -> None:
    ledger = read_json(LEDGER)
    ledger["unit_queue"] = QUEUE.relative_to(ROOT).as_posix()
    ledger["active_unit"] = unit["id"] if unit and unit["state"] in OPEN_STATES else None
    ledger["next_action"] = next_action
    ledger["last_checkpoint_utc"] = now()
    counts: dict[str, dict[str, int]] = {}
    for item in queue["units"]:
        counts.setdefault(item["package"], {}).setdefault(item["state"], 0)
        counts[item["package"]][item["state"]] += 1
    ledger["queue_summary"] = counts
    if checks is not None:
        ledger["last_checks"] = checks
    write_json(LEDGER, ledger)


def staging(unit: dict[str, Any]) -> Path:
    path = STAGING / unit["id"]
    path.mkdir(parents=True, exist_ok=True)
    return path


def rel(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def git(*args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", "-C", str(ROOT), *args], capture_output=True, text=True, check=False)


def dirty_paths() -> set[str]:
    out = git("status", "--porcelain", "--untracked-files=all").stdout
    return {line[3:].split(" -> ")[-1] for line in out.splitlines() if line.strip()}


def model_name() -> str:
    model = read_json(LEDGER).get("model")
    if isinstance(model, dict):
        if model.get("actual_tag"):
            return str(model["actual_tag"])
        return f"{model.get('requested_family') or 'unknown model'}, exact tag not recorded"
    return str(model or "unknown model")


# Commands ------------------------------------------------------------------

def cmd_status(_: argparse.Namespace) -> int:
    queue = load_queue()
    table: dict[str, dict[str, int]] = {}
    for unit in queue["units"]:
        key = f"{unit['package']} {unit['tier']}"
        table.setdefault(key, {}).setdefault(unit["state"], 0)
        table[key][unit["state"]] += 1
    for key in sorted(table):
        print(f"{key:9} " + ", ".join(f"{state} {count}" for state, count in sorted(table[key].items())))
    open_units = [u for u in queue["units"] if u["state"] in OPEN_STATES or (u["state"] == "done" and not u.get("commit"))]
    audits = [u["id"] for u in queue["units"] if u.get("audit") == "due"]
    if open_units:
        print(f"Open unit: {open_units[0]['id']} ({open_units[0]['state']})")
    if audits:
        print(f"Audits due for a stronger model: {', '.join(audits[:10])}")
    print(f"Next action: {read_json(LEDGER).get('next_action')}")
    return 0


def cmd_queue(args: argparse.Namespace) -> int:
    from encyclopedia_queue import PACKAGE_ORDER, rebuild

    packages = args.package or list(PACKAGE_ORDER)
    queue = rebuild(packages)
    ledger_update(queue, None, "Run python3 tools/encyclopedia_unit.py next")
    print(f"Queue rebuilt for {', '.join(packages)}: {len(queue['units'])} units in {rel(QUEUE)}")
    return cmd_status(args)


def e0_checks() -> list[tuple[bool, str]]:
    results: list[tuple[bool, str]] = []

    def check(ok: bool, text: str) -> None:
        results.append((bool(ok), text))

    coverage = read_json(ROOT / "docs/encyclopedia-coverage.v1.json")
    states = coverage.get("evidence_states", {})
    check(set(states) == set(EVIDENCE_STATES), f"coverage report defines exactly the evidence states {', '.join(EVIDENCE_STATES)}")
    check(states.get("unknown") and states.get("not_reviewed") and states["unknown"] != states["not_reviewed"],
          "\"unknown\" (searched, no answer) and \"not reviewed\" (not searched) have different descriptions")
    rust = (ROOT / "korangar/src/dm/reference_data.rs").read_text(encoding="utf-8")
    block = re.search(r"pub enum EvidenceState \{(.*?)\}", rust, re.S)
    variants = {re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower() for name in re.findall(r"^\s*([A-Z][A-Za-z]+),", block.group(1), re.M)} if block else set()
    check(variants == set(EVIDENCE_STATES), "the client's EvidenceState enum has the same six states")
    items = {entry["id"]: entry for entry in read_json(ROOT / "docs/items.v1.json")["entries"]}
    check(items[501]["effect_status"] == "translated" and items[501].get("effect_summary"), "item 501 (Red Potion) is translated and has a summary")
    untranslated = [e for e in items.values() if e["effect_status"] == "scripted_not_translated"]
    check(untranslated and not any(e.get("effect_summary") for e in untranslated), "no untranslated item carries a summary")
    quest = next(e for e in read_json(ROOT / "docs/quests.v1.json")["entries"] if e["id"] == 12106)
    flow = quest.get("flow_review") or {}
    check(flow.get("evidence_state") in REVIEWED_STATES and flow.get("sources"), "quest 12106's reviewed flow has a reviewed state and sources")
    npc = next(e for e in read_json(ROOT / "docs/npcs.v1.json")["entries"] if e.get("name") == "Tool Dealer#alb")
    sells = True
    for offer in npc.get("offers", []):
        path, line = offer["source"].rsplit(":", 1)
        text = source_lines(path)[int(line) - 1]
        sells &= "sellitem" in text
    check(npc.get("offers") and sells, f"NPC {npc['id']} (Tool Dealer#alb): every listed offer points at a literal sellitem line")
    flags = read_json(ROOT / "docs/map-flags.v1.json")
    check(not any(c.get("evidence_state") in REVIEWED_STATES for c in flags.get("runtime_clues", [])), "no runtime map-flag clue is labeled as reviewed")
    spawn = next(e for e in read_json(ROOT / "docs/scripted-spawn-reviews.v1.json")["entries"] if e["id"] == "bakonawalake_onmobspawn_1_at_ma_b")
    check(spawn.get("evidence_state") in REVIEWED_STATES, "scripted spawn bakonawalake_onmobspawn_1_at_ma_b is reviewed, not a clue")
    formula = next(e for e in read_json(ROOT / "docs/skill-formula-reviews.v1.json")["entries"] if e["id"] == "al_heal_renewal_formula")
    check(formula.get("evidence_state") in REVIEWED_STATES and formula.get("sources"), "formula al_heal_renewal_formula is reviewed and cites sources")
    exchanges = read_json(ROOT / "docs/item-exchanges.v1.json")["entries"]
    check(all(e.get("evidence_state") in REVIEWED_STATES for e in exchanges), "every exported exchange carries a reviewed state")
    return results


def cmd_e0(_: argparse.Namespace) -> int:
    results = e0_checks()
    for ok, text in results:
        print(f"{'PASS' if ok else 'FAIL'} {text}")
    print("Not verified here: that the Guide window shows these labels on screen. That is part of the E7 live pass.")
    queue = load_queue()
    unit = next((u for u in queue["units"] if u["kind"] == "e0_check"), None)
    passed = all(ok for ok, _ in results)
    if unit:
        unit["state"] = "done" if passed else "blocked"
        unit["commit"] = "not needed"
        if not passed:
            unit["blocker"] = "E0 rule check failed; fix the label or data, not the check."
        log(unit, "e0 passed" if passed else "e0 failed")
    ledger_update(queue, None, "Run python3 tools/encyclopedia_unit.py next", [f"{'PASS' if ok else 'FAIL'} {t}" for ok, t in results])
    save_queue(queue)
    if not passed:
        return stop("E0 failed. Do not change the check to make it pass.")
    print("E0 passed. Next: python3 tools/encyclopedia_unit.py next")
    return 0


def cmd_next(args: argparse.Namespace) -> int:
    queue = load_queue()
    for unit in queue["units"]:
        folder = staging(unit) if unit["state"] in OPEN_STATES else None
        if unit["state"] == "active":
            feedback = sorted(folder.glob("feedback-*.md"))
            extra = f" and {rel(feedback[-1])} (the verifier's objections; fix or downgrade each to unknown)" if feedback else ""
            print(f"Resume unit {unit['id']}: read {rel(folder / 'packet.md')}{extra}. "
                  f"Write {rel(folder / 'draft.json')}. Then run: python3 tools/encyclopedia_unit.py validate")
            return 0
        if unit["state"] == "validated":
            return stop(f"Unit {unit['id']} is waiting for a verifier. A NEW session runs: python3 tools/encyclopedia_unit.py verify", 0)
        if unit["state"] == "verified":
            print(f"Unit {unit['id']} is verified. Run: python3 tools/encyclopedia_unit.py accept")
            return 0
        if unit["state"] == "done" and not unit.get("commit"):
            print(f"Unit {unit['id']} is accepted but not committed. Run: python3 tools/encyclopedia_unit.py commit")
            return 0
    tiers = {"local"} if args.tier == "local" else {"local", "strong"}
    for unit in queue["units"]:
        if unit["state"] != "todo" or unit["tier"] not in tiers or (args.package and unit["package"] not in args.package):
            continue
        if unit["kind"] == "e0_check":
            print(f"Next unit is {unit['id']}. Run: python3 tools/encyclopedia_unit.py e0")
            return 0
        unit["state"] = "active"
        unit["baseline_dirty"] = sorted(dirty_paths())
        log(unit, "activated")
        folder = staging(unit)
        draft_path = folder / "draft.json"
        packet, extra = build_packet(unit, rel(draft_path))
        if extra:
            unit["scope"] = [p for p in unit["scope"] if not p.get("helper")] + extra
        if unit["tier"] == "local" and unit["kind"] not in TOOL_DRAFTED and packet.count("\n") > PACKET_LINE_LIMIT:
            unit["state"] = "todo"
            unit["tier"] = "strong"
            log(unit, f"packet is {packet.count(chr(10))} lines; moved to strong tier")
            save_queue(queue)
            print(f"{unit['id']}: packet too large for a local model; moved to the strong tier.")
            return cmd_next(args)
        (folder / "packet.md").write_text(packet, encoding="utf-8")
        if unit["kind"] in TOOL_DRAFTED:
            write_json(draft_path, tool_draft(unit, model_name()))
            instruction = "The tool drafted this unit. Run: python3 tools/encyclopedia_unit.py validate"
        else:
            instruction = f"Read {rel(folder / 'packet.md')} and nothing else. Write {rel(draft_path)}. Then run: python3 tools/encyclopedia_unit.py validate"
        ledger_update(queue, unit, instruction)
        save_queue(queue)
        print(f"Active unit: {unit['id']} ({unit['kind']}, {unit['size_lines']} source lines)")
        print(instruction)
        return 0
    return stop("No runnable unit for this tier. Strong-tier and blocked units remain; see `status`.", 0)


def cmd_packet(args: argparse.Namespace) -> int:
    queue = load_queue()
    unit = find_unit(queue, args.id)
    folder = staging(unit)
    packet, extra = build_packet(unit, rel(folder / "draft.json"))
    if extra:
        unit["scope"] = [p for p in unit["scope"] if not p.get("helper")] + extra
        save_queue(queue)
    (folder / "packet.md").write_text(packet, encoding="utf-8")
    print(f"Wrote {rel(folder / 'packet.md')} ({packet.count(chr(10))} lines)")
    return 0


def load_draft(unit: dict[str, Any]) -> Any:
    path = staging(unit) / "draft.json"
    if not path.is_file():
        raise SystemExit(stop(f"No draft at {rel(path)}. Write it from the packet first."))
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        print(f"draft.json is not valid JSON: {error}")
        raise SystemExit(1)


def check_translator(draft: dict[str, Any], unit: dict[str, Any]) -> list[str]:
    spec = draft.get("translator")
    if draft.get("unit") != unit["id"] or not isinstance(spec, dict):
        return [f"draft needs `unit` = {unit['id']!r} and a `translator` object"]
    errors = check_sources("translator.doc_source", [spec.get("doc_source")], unit["scope"])
    import export_item_reference

    translator = importlib.reload(export_item_reference).translate_simple_effect
    positive, negative = spec.get("positive_script", ""), spec.get("negative_script", "")
    got = translator(positive) if positive else None
    if got != spec.get("expected_summary"):
        errors.append(f"translate_simple_effect(positive_script) returned {got!r}, not expected_summary {spec.get('expected_summary')!r}")
    if not negative or translator(negative) is not None:
        errors.append("negative_script must be present and must still return None")
    tests = (ROOT / "tools/tests/test_export_item_effect_phrases.py").read_text(encoding="utf-8")
    if not negative or negative not in tests:
        errors.append("tools/tests/test_export_item_effect_phrases.py must contain the negative_script text in a test")
    ids = spec.get("item_ids")
    if not isinstance(ids, list) or not ids or not set(ids) <= set(unit["subject"]["item_ids"]):
        errors.append("item_ids must be a non-empty subset of this unit's items")
    return errors


def cmd_validate(args: argparse.Namespace) -> int:
    queue = load_queue()
    unit = find_unit(queue, args.id)
    if unit["state"] not in OPEN_STATES:
        return stop(f"Unit {unit['id']} is {unit['state']}; nothing to validate.")
    draft = load_draft(unit)
    errors = check_translator(draft, unit) if unit["kind"] == "e3_translator" else check_draft(draft, unit)
    if errors:
        unit["state"] = "active"
        log(unit, f"validate failed ({len(errors)} errors)")
        save_queue(queue)
        print(f"Validation failed with {len(errors)} error(s). Fix draft.json and run validate again:")
        for error in errors[:40]:
            print(f"- {error}")
        return 1
    unit["state"] = "validated"
    unit["draft_sha256"] = hashlib.sha256((staging(unit) / "draft.json").read_bytes()).hexdigest()
    log(unit, "validated")
    if unit["kind"] in VERIFY_NOT_REQUIRED:
        unit["state"] = "verified"
        log(unit, "verification not required: tool-drafted, no behavior claim")
        nxt = "Run: python3 tools/encyclopedia_unit.py accept"
    else:
        nxt = ("Validation passed. End this session. In a NEW session (the verifier), run: "
               "python3 tools/encyclopedia_unit.py verify")
    ledger_update(queue, unit, nxt)
    save_queue(queue)
    print(nxt)
    return 0 if unit["kind"] in VERIFY_NOT_REQUIRED else stop("Hand off to the verifier session.", 0)


def cmd_verify(args: argparse.Namespace) -> int:
    queue = load_queue()
    unit = find_unit(queue, args.id)
    folder = staging(unit)
    if unit["state"] != "validated":
        return stop(f"Unit {unit['id']} is {unit['state']}; only a validated unit is verified.")
    draft = load_draft(unit)
    if hashlib.sha256((folder / "draft.json").read_bytes()).hexdigest() != unit.get("draft_sha256"):
        unit["state"] = "active"
        save_queue(queue)
        return stop("draft.json changed after validation. Run validate again.")
    if unit["kind"] == "e3_translator":
        spec = draft["translator"]
        claims = [f"The documentation defines `{unit['subject']['key']}` so that `{spec['positive_script']}` means: {spec['expected_summary']}"]
    else:
        claims = claims_of(draft)
    if not args.record:
        old_verdict = folder / "verdict.json"
        if old_verdict.is_file():
            old_verdict.rename(folder / f"verdict-{now().replace(':', '')}.json")
        write_json(folder / "claims.json", claims)
        (folder / "verify.md").write_text(verify_packet(unit, claims), encoding="utf-8")
        print(f"Read {rel(folder / 'verify.md')} and nothing else. Write {rel(folder / 'verdict.json')}.")
        print("Then run: python3 tools/encyclopedia_unit.py verify --record")
        return 0
    verdict_path = folder / "verdict.json"
    try:
        verdict = json.loads(verdict_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"verdict.json missing or invalid: {error}")
        return 1
    rows = verdict.get("claims", []) if isinstance(verdict, dict) else []
    judged = {row.get("n"): row for row in rows if isinstance(row, dict)}
    missing = [n for n in range(1, len(claims) + 1) if n not in judged]
    if missing:
        print(f"verdict.json must judge every claim; missing: {missing[:20]}")
        return 1
    bad = [row for row in judged.values() if row.get("verdict") != "supported"]
    omissions = [o for o in verdict.get("omissions", []) if str(o).strip()]
    if not bad and not omissions:
        unit["state"] = "verified"
        log(unit, f"verified: {len(claims)} claims supported")
        nxt = "Run: python3 tools/encyclopedia_unit.py accept"
        ledger_update(queue, unit, nxt)
        save_queue(queue)
        print(nxt)
        return 0
    unit["attempts"] = unit.get("attempts", 0) + 1
    feedback = [f"claim {row.get('n')}: {row.get('verdict')} — {claims[row['n'] - 1][:160]} — {row.get('note', '')}" for row in bad]
    feedback += [f"omission: {o}" for o in omissions]
    (folder / f"feedback-{unit['attempts']}.md").write_text("\n".join(feedback) + "\n", encoding="utf-8")
    log(unit, f"verifier rejected attempt {unit['attempts']}: {len(bad)} claims, {len(omissions)} omissions")
    if unit["attempts"] >= MAX_ATTEMPTS:
        unit["state"] = "blocked"
        unit["blocker"] = f"Verifier rejected {MAX_ATTEMPTS} drafts; see {rel(folder)}/feedback-*.md"
        ledger_update(queue, unit, "Run python3 tools/encyclopedia_unit.py next")
        save_queue(queue)
        return stop(f"Unit {unit['id']} is blocked after {MAX_ATTEMPTS} rejected drafts. The next session runs `next`.", 0)
    unit["state"] = "active"
    feedback_file = folder / f"feedback-{unit['attempts']}.md"
    nxt = (f"Verifier rejected the draft. The drafting session reads {rel(folder / 'packet.md')} and "
           f"{rel(feedback_file)}, fixes or downgrades each listed claim to unknown, then runs validate.")
    ledger_update(queue, unit, nxt)
    save_queue(queue)
    print("\n".join(feedback))
    return stop(nxt, 0)


def snapshot(paths: list[Path]) -> dict[str, str]:
    return {rel(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths if p.is_file()}


def generated_files() -> list[Path]:
    return sorted((ROOT / "docs").glob("*.v1.json")) + [ROOT / "korangar/data/navigation_graph.json"]


def run(command: list[str], label: str, cwd: Path = ROOT) -> tuple[bool, str]:
    done = subprocess.run(command, cwd=cwd, capture_output=True, text=True, check=False)
    tail = "\n".join((done.stdout + done.stderr).strip().splitlines()[-6:])
    return done.returncode == 0, f"{label}: {'passed' if done.returncode == 0 else 'FAILED'}\n{tail if done.returncode else ''}".strip()


def cmd_accept(args: argparse.Namespace) -> int:
    queue = load_queue()
    unit = find_unit(queue, args.id)
    if unit["state"] != "verified":
        return stop(f"Unit {unit['id']} is {unit['state']}; accept needs a verified unit.")
    draft = load_draft(unit)
    if hashlib.sha256((staging(unit) / "draft.json").read_bytes()).hexdigest() != unit.get("draft_sha256"):
        unit["state"] = "active"
        save_queue(queue)
        return stop("draft.json changed after validation. Run validate again.")
    errors = check_translator(draft, unit) if unit["kind"] == "e3_translator" else check_draft(draft, unit)
    if errors:
        unit["state"] = "active"
        save_queue(queue)
        return stop("The draft no longer validates (the source or review files moved):\n- " + "\n- ".join(errors[:20]))
    stamp = {"reviewed_by": f"Model-assisted source review ({model_name()})", "reviewed_on": today()}
    touched: list[Path] = []
    backups: dict[Path, str] = {}
    if unit["kind"] == "e3_translator":
        touched = [ROOT / "tools/export_item_reference.py", ROOT / "tools/tests/test_export_item_effect_phrases.py"]
    else:
        if draft.get("reviews"):
            review_file = ROOT / REVIEW_FILES[unit["kind"]]
            backups[review_file] = review_file.read_text(encoding="utf-8")
            data = read_json(review_file)
            for review in draft["reviews"]:
                data["entries"].append({**review, **stamp})
            write_json(review_file, data)
            touched.append(review_file)
        if draft.get("dispositions"):
            backups[DISPOSITIONS] = DISPOSITIONS.read_text(encoding="utf-8")
            data = read_json(DISPOSITIONS)
            for record in draft["dispositions"]:
                data["entries"].append({**record, "package": unit["package"], "unit": unit["id"], **stamp})
            write_json(DISPOSITIONS, data)
            touched.append(DISPOSITIONS)
    before = snapshot(generated_files())
    checks = []
    ok, text = run([PYTHON, "tools/export_supported_data.py"], "export_supported_data.py")
    checks.append(text)
    if ok:
        ok, text = run([PYTHON, "tools/export_supported_data.py", "--check"], "export_supported_data.py --check")
        checks.append(text)
    if ok and unit["kind"] == "e3_translator":
        ok, text = run([PYTHON, "-m", "unittest", "tests.test_export_item_effect_phrases"], "item effect phrase tests", ROOT / "tools")
        checks.append(text)
        items = {e["id"]: e for e in read_json(ROOT / "docs/items.v1.json")["entries"]}
        still = [i for i in draft["translator"]["item_ids"] if items.get(i, {}).get("effect_status") != "translated"]
        if still:
            ok = False
            checks.append(f"items still untranslated after the new rule: {still[:10]}")
    after = snapshot(generated_files())
    changed = sorted(path for path in after if before.get(path) != after[path])
    rust_inputs = [p for p in changed if p.startswith("docs/")]
    if ok and rust_inputs and not args.skip_rust:
        ok, text = run(["cargo", "test", "-p", "korangar", "--lib", RUST_TEST, "--quiet"], f"cargo test {RUST_TEST}")
        checks.append(text)
    if not ok:
        for path, text in backups.items():
            path.write_text(text, encoding="utf-8")
        subprocess.run([PYTHON, "tools/export_supported_data.py"], cwd=ROOT, capture_output=True, check=False)
        unit["attempts"] = unit.get("attempts", 0) + 1
        unit["state"] = "blocked" if unit["attempts"] >= MAX_ATTEMPTS else "active"
        if unit["state"] == "blocked":
            unit["blocker"] = "Accept failed twice: " + checks[-1][:300]
        log(unit, f"accept failed, review files restored (attempt {unit['attempts']})")
        ledger_update(queue, unit, "Fix the draft and run validate again" if unit["state"] == "active" else "Run python3 tools/encyclopedia_unit.py next", checks)
        save_queue(queue)
        return stop("Accept failed; the review files were restored.\n" + "\n".join(checks))
    accepted = sorted({rel(p) for p in touched} | set(changed) | TOOL_OWNED)
    unit["state"] = "done"
    unit["accepted_files"] = accepted
    unit["accepted_on"] = today()
    done_count = sum(1 for u in queue["units"] if u["state"] == "done" and u["kind"] not in VERIFY_NOT_REQUIRED)
    if unit["kind"] not in VERIFY_NOT_REQUIRED and done_count % AUDIT_EVERY == 0:
        unit["audit"] = "due"
    log(unit, "accepted")
    reviews = [r["id"] for r in draft.get("reviews", [])]
    dispositions = [d["id"] for d in draft.get("dispositions", [])]
    with JOURNAL.open("a", encoding="utf-8") as journal:
        journal.write(
            f"\n## {today()} {unit['id']} — accepted\n\n"
            f"- Kind {unit['kind']}, package {unit['package']}, model {model_name()}, attempts {unit.get('attempts', 0)}.\n"
            f"- Reviews: {', '.join(reviews) or 'none'}. Dispositions: {len(dispositions)}"
            f"{' (' + ', '.join(dispositions[:5]) + (' ...' if len(dispositions) > 5 else '') + ')' if dispositions else ''}.\n"
            f"- Checks: validate; {'verifier: all claims supported' if unit['kind'] not in VERIFY_NOT_REQUIRED else 'verifier not required'}; "
            + "; ".join(c.splitlines()[0] for c in checks) + ".\n"
            f"- Files: {', '.join(accepted)}.\n"
            f"- Not verified: the Guide screen for these records (E7 live pass).\n"
        )
    nxt = "Run: python3 tools/encyclopedia_unit.py commit"
    ledger_update(queue, unit, nxt, checks)
    save_queue(queue)
    print("\n".join(checks))
    print(f"Accepted {unit['id']}. {nxt}")
    return 0


def cmd_commit(args: argparse.Namespace) -> int:
    queue = load_queue()
    unit = find_unit(queue, args.id)
    if unit["state"] != "done" or unit.get("commit"):
        return stop(f"Unit {unit['id']} is {unit['state']}{' and committed' if unit.get('commit') else ''}; nothing to commit.")
    staged = git("diff", "--cached", "--name-only").stdout.split()
    if staged:
        return stop(f"Other files are already staged: {', '.join(staged[:10])}. A person must unstage them.")
    inherited = sorted(set(unit.get("accepted_files", [])) & set(unit.get("baseline_dirty", [])) - TOOL_OWNED)
    if inherited:
        return stop("These files had uncommitted changes before this unit started, so a commit would include "
                    f"someone else's work: {', '.join(inherited)}. A person must commit or discard those changes first.")
    draft = json.loads((staging(unit) / "draft.json").read_text(encoding="utf-8"))
    if "translator" in draft:
        counts = "translator rule"
    else:
        counts = f"{len(draft.get('reviews', []))} review(s), {len(draft.get('dispositions', []))} disposition(s)"
    checked = "verified" if unit["kind"] not in VERIFY_NOT_REQUIRED else "tool-drafted"
    message = f"Encyclopedia {unit['id']}: {counts}\n\nModel-assisted ({model_name()}); validated and {checked}.\n"
    unit["commit"] = "committed"
    log(unit, "committed")
    nxt = "Run: python3 tools/encyclopedia_unit.py next"
    ledger_update(queue, None, nxt)
    save_queue(queue)
    files = [f for f in unit["accepted_files"] if (ROOT / f).exists()]
    added = git("add", "--", *files)
    done = git("commit", "-m", message) if added.returncode == 0 else added
    if done.returncode:
        git("reset", "-q", "--", *files)
        unit["commit"] = None
        save_queue(queue)
        return stop(f"git commit failed: {(done.stdout + done.stderr).strip()[:400]}")
    print(f"Committed {unit['id']} as {git('rev-parse', '--short', 'HEAD').stdout.strip()}. {nxt}")
    return 0


def cmd_block(args: argparse.Namespace) -> int:
    queue = load_queue()
    unit = find_unit(queue, args.id)
    if not args.reason or len(args.reason.strip()) < 12:
        return stop("block needs --reason with the specific input that is missing.")
    unit["state"] = "blocked"
    unit["blocker"] = args.reason.strip()
    log(unit, f"blocked: {args.reason.strip()}")
    ledger_update(queue, unit, "Run python3 tools/encyclopedia_unit.py next")
    save_queue(queue)
    print(f"Blocked {unit['id']}. Next session: python3 tools/encyclopedia_unit.py next")
    return 0


def cmd_audit(args: argparse.Namespace) -> int:
    queue = load_queue()
    if args.id is None:
        due = [u for u in queue["units"] if u.get("audit") == "due"]
        if not due:
            print("No audit is due.")
            return 0
        unit = due[0]
    else:
        unit = find_unit(queue, args.id)
    if not args.passed and not args.failed:
        print(f"Audit {unit['id']}: reread {rel(staging(unit) / 'verify.md')} against the accepted records "
              f"(commit {unit.get('commit')}), then run audit {unit['id']} --pass or --fail \"reason\".")
        return 0
    unit["audit"] = "passed" if args.passed else f"failed: {args.failed}"
    log(unit, f"audit {unit['audit']}")
    if args.failed:
        unit["blocker"] = f"Audit failed: {args.failed}. The accepted records need correction in a separate unit."
    save_queue(queue)
    print(f"Recorded audit for {unit['id']}: {unit['audit']}")
    return 0


def cmd_model(args: argparse.Namespace) -> int:
    ledger = read_json(LEDGER)
    model = ledger.get("model") if isinstance(ledger.get("model"), dict) else {}
    model["actual_tag"] = args.tag
    ledger["model"] = model
    write_json(LEDGER, ledger)
    print(f"Accepted records will be attributed to: Model-assisted source review ({args.tag})")
    return 0


def cmd_citations(_: argparse.Namespace) -> int:
    files = list(REVIEW_FILES.values()) + ["tools/npc_service_reviews.json", "tools/map_runtime_flag_reviews.json"]
    total = 0
    for name in files:
        for entry in read_json(ROOT / name)["entries"]:
            sources = entry.get("sources") or ([dict(entry["source"], required_source_literals=entry.get("required_source_literals"))] if "source" in entry else [])
            if not sources:
                continue
            misplaced = [e for e in check_sources(entry["id"], sources) if "literal not found" in e]
            if misplaced:
                total += 1
                print(f"{name} {entry['id']}: {misplaced[0].split(': ', 1)[1]}")
    print(f"{total} reviewed record(s) quote text that is not on any of their cited lines.")
    return 1 if total else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("status").set_defaults(func=cmd_status)
    q = sub.add_parser("queue")
    q.add_argument("--package", action="append", choices=["E0", "E1", "E2", "E3", "E4", "E5", "E6", "E7"])
    q.set_defaults(func=cmd_queue)
    sub.add_parser("e0").set_defaults(func=cmd_e0)
    n = sub.add_parser("next")
    n.add_argument("--tier", choices=["local", "strong"], default="local")
    n.add_argument("--package", action="append")
    n.set_defaults(func=cmd_next)
    for name, func in (("packet", cmd_packet), ("validate", cmd_validate), ("commit", cmd_commit)):
        p = sub.add_parser(name)
        p.add_argument("id", nargs="?")
        p.set_defaults(func=func)
    v = sub.add_parser("verify")
    v.add_argument("id", nargs="?")
    v.add_argument("--record", action="store_true")
    v.set_defaults(func=cmd_verify)
    a = sub.add_parser("accept")
    a.add_argument("id", nargs="?")
    a.add_argument("--skip-rust", action="store_true", help="for a person only; the loop never skips it")
    a.set_defaults(func=cmd_accept)
    b = sub.add_parser("block")
    b.add_argument("id", nargs="?")
    b.add_argument("--reason", required=True)
    b.set_defaults(func=cmd_block)
    au = sub.add_parser("audit")
    au.add_argument("id", nargs="?")
    au.add_argument("--pass", dest="passed", action="store_true")
    au.add_argument("--fail", dest="failed")
    au.set_defaults(func=cmd_audit)
    sub.add_parser("citations").set_defaults(func=cmd_citations)
    m = sub.add_parser("model", help="a person records the exact model tag used for attribution")
    m.add_argument("tag")
    m.set_defaults(func=cmd_model)
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
