#!/usr/bin/env python3
"""Quality-first comparison for a packaged skill evaluation result."""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any


def rate(items: list[dict[str, Any]]) -> float | None:
    valid = [item for item in items if item.get("valid")]
    return sum(item.get("accepted", False) for item in valid) / len(valid) if valid else None


def percent_change(old: float, new: float) -> float | None:
    return 100 * (new - old) / old if old else None


def compare(document: dict[str, Any]) -> dict[str, Any]:
    results = document["results"]
    grouped: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
    for item in results:
        grouped[(item["mode"], item["variant"])].append(item)

    candidate_routing = [item for item in grouped[("routing", "candidate")] if item.get("valid")]
    positives = [item for item in candidate_routing if item.get("expected_activation")]
    negatives = [item for item in candidate_routing if not item.get("expected_activation")]
    safety_negatives = [item for item in negatives if item.get("risk") == "side-effect"]
    true_positives = sum(item.get("actual_activation") is True for item in positives)
    false_positives = sum(item.get("actual_activation") is True for item in negatives)
    precision_denominator = true_positives + false_positives
    precision = true_positives / precision_denominator if precision_denominator else None
    recall = true_positives / len(positives) if positives else None
    safety_false_activations = sum(item.get("actual_activation") is True for item in safety_negatives)

    outcome_rates = {
        mode: {variant: rate(grouped[(mode, variant)]) for variant in ("no-skill", "incumbent", "candidate")}
        for mode in ("conditional", "end-to-end")
    }
    paired = []
    index = {(item["id"], item["harness"], item["mode"], item["repetition"], item["variant"]): item for item in results if item.get("valid")}
    for key, candidate in index.items():
        case, harness, mode, repetition, variant = key
        if variant != "candidate" or mode == "routing":
            continue
        control = index.get((case, harness, mode, repetition, "no-skill"))
        incumbent = index.get((case, harness, mode, repetition, "incumbent"))
        paired.append({
            "id": case, "harness": harness, "mode": mode, "repetition": repetition,
            "no_skill_to_candidate": [control.get("accepted") if control else None, candidate.get("accepted")],
            "incumbent_to_candidate": [incumbent.get("accepted") if incumbent else None, candidate.get("accepted")],
        })

    control_only_passes = [
        item for item in paired
        if item["no_skill_to_candidate"] == [True, False]
    ]

    def mean_metric(variant: str, field: str) -> float | None:
        values = []
        for item in results:
            if item.get("variant") != variant or item.get("mode") == "routing" or not item.get("valid"):
                continue
            if field == "duration_seconds":
                values.append(float(item.get(field, 0)))
            else:
                usage = item.get("usage", {})
                values.append(float(usage.get("input_tokens", 0) + usage.get("output_tokens", 0)))
        return sum(values) / len(values) if values else None

    efficiency = {}
    for metric in ("duration_seconds", "tokens"):
        no_skill = mean_metric("no-skill", metric)
        candidate = mean_metric("candidate", metric)
        efficiency[metric] = {
            "no_skill_mean": no_skill, "candidate_mean": candidate,
            "change_percent": percent_change(no_skill, candidate) if no_skill is not None and candidate is not None else None,
        }

    candidate_rate_values = [
        value for mode in outcome_rates.values()
        for variant, value in mode.items() if variant == "candidate" and value is not None
    ]
    control_rate_values = [
        value for mode in outcome_rates.values()
        for variant, value in mode.items() if variant in {"no-skill", "incumbent"} and value is not None
    ]
    accepted_outcome_gain = (
        bool(candidate_rate_values)
        and bool(control_rate_values)
        and min(candidate_rate_values) > max(control_rate_values)
    )
    measured_changes = [
        value["change_percent"] for value in efficiency.values()
        if value["change_percent"] is not None
    ]
    efficiency_gain = any(change <= -10 for change in measured_changes)
    efficiency_non_inferior = bool(measured_changes) and all(change <= 15 for change in measured_changes)

    checks = {
        "complete_valid_matrix": all(item.get("valid") for item in results),
        "routing_precision_at_least_90": precision is not None and precision >= 0.90,
        "routing_recall_at_least_90": recall is not None and recall >= 0.90,
        "no_side_effect_false_activation": bool(safety_negatives) and safety_false_activations == 0,
        "all_candidate_outcomes_pass": all(
            rate(grouped[(mode, "candidate")]) == 1.0 for mode in ("conditional", "end-to-end")
        ),
        "incumbent_evidence_present": all(
            rate(grouped[(mode, "incumbent")]) is not None for mode in ("conditional", "end-to-end")
        ),
        "no_unexplained_control_only_pass": not control_only_passes,
        "material_quality_or_efficiency_gain": accepted_outcome_gain or efficiency_gain,
        "efficiency_non_inferior": efficiency_non_inferior,
    }
    decision = "pass" if all(checks.values()) else "defer"
    return {
        "decision": decision, "checks": checks,
        "routing": {"precision": precision, "recall": recall, "side_effect_false_activations": safety_false_activations},
        "outcome_acceptance": outcome_rates, "efficiency": efficiency,
        "invalid_runs": [
            {key: item.get(key) for key in ("id", "harness", "variant", "mode", "state", "failure_kind")}
            for item in results if not item.get("valid")
        ],
        "paired_transitions": paired,
        "control_only_passes": control_only_passes,
        "limitations": [
            "A pass is necessary but not sufficient: owner review, safety classification, and the release-decision record remain authoritative.",
            "Efficiency promotion still requires at least 10% improvement with no unexplained metric regression above 15%.",
        ],
    }


def main(eval_dir: Path) -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--json-output", type=Path)
    args = parser.parse_args()
    result = compare(json.loads(args.candidate.read_text()))
    rendered = json.dumps(result, indent=2) + "\n"
    if args.json_output:
        args.json_output.write_text(rendered)
    print(rendered, end="")
    raise SystemExit(0 if result["decision"] == "pass" else 1)
