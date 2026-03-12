"""
Eval Example
=============

Demonstrates Traceway's eval system:
1. Creates a dataset with test cases
2. Runs evals with different models
3. Compares results

This uses the Traceway API directly — evals run server-side, so you
only need an API key for the LLM provider set on the server or passed
in the eval config.

Prerequisites:
    pip install traceway python-dotenv

Usage:
    python examples/eval_example.py
"""
from __future__ import annotations

import os
import sys
import time
import json
from pathlib import Path

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "sdk", "python"))

from dotenv import load_dotenv
load_dotenv(Path(__file__).resolve().parent.parent / ".env")

import httpx

# ── Config ────────────────────────────────────────────────────────────

BASE_URL = os.environ.get("TRACEWAY_URL", "https://api.traceway.ai")
API_KEY = os.environ.get("TRACEWAY_API_KEY")

if not API_KEY:
    print("Set TRACEWAY_API_KEY in .env or environment")
    sys.exit(1)

HEADERS = {"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"}


def api(method: str, path: str, **kwargs) -> dict:
    resp = httpx.request(method, f"{BASE_URL}/api{path}", headers=HEADERS, **kwargs)
    if resp.status_code >= 400:
        print(f"  API error {resp.status_code}: {resp.text[:200]}")
    resp.raise_for_status()
    return resp.json() if resp.content else {}


# ── Test dataset ──────────────────────────────────────────────────────

EVAL_CASES = [
    {
        "input": "What is the capital of France?",
        "expected": "Paris",
    },
    {
        "input": "What is 15 * 23?",
        "expected": "345",
    },
    {
        "input": "Translate 'hello world' to Spanish.",
        "expected": "hola mundo",
    },
    {
        "input": "What programming language is known for its use in data science and has a snake-themed name?",
        "expected": "Python",
    },
    {
        "input": "What is the chemical symbol for gold?",
        "expected": "Au",
    },
]


# ── Main ──────────────────────────────────────────────────────────────

def main():
    print("Traceway Eval Example")
    print(f"API: {BASE_URL}\n")

    # 1. Create a dataset
    print("[1/4] Creating dataset...")
    dataset = api("POST", "/datasets", json={
        "name": "knowledge-qa",
        "description": "Simple Q&A pairs to test model accuracy",
    })
    dataset_id = dataset["id"]
    print(f"  Dataset: {dataset_id} ({dataset['name']})")

    # 2. Add datapoints
    print(f"[2/4] Adding {len(EVAL_CASES)} test cases...")
    for case in EVAL_CASES:
        dp = api("POST", f"/datasets/{dataset_id}/datapoints", json={
            "kind": {
                "type": "llm_conversation",
                "messages": [{"role": "user", "content": case["input"]}],
                "expected_output": case["expected"],
            }
        })
        print(f"  Added: {case['input'][:50]}...")

    # 3. Run eval with exact_match scoring
    print("\n[3/4] Running eval (exact_match scoring)...")
    print("  This calls the LLM for each test case server-side.")
    print("  The server needs OPENAI_API_KEY set, or pass api_key_env in config.\n")

    run = api("POST", f"/datasets/{dataset_id}/eval", json={
        "name": "gpt-4o-mini-exact",
        "config": {
            "model": "gpt-4o-mini",
            "provider": "openai",
            "system_prompt": "Answer the question directly and concisely. Just give the answer, no explanation.",
            "temperature": 0.0,
            "max_tokens": 100,
        },
        "scoring": "exact_match",
    })
    run_id = run["id"]
    print(f"  Eval run started: {run_id}")

    # Poll until complete
    for _ in range(60):
        time.sleep(2)
        detail = api("GET", f"/eval/{run_id}")
        status = detail["status"]
        completed = detail["results"]["completed"]
        total = detail["results"]["total"]
        print(f"  Status: {status} ({completed}/{total})")
        if status in ("completed", "failed", "cancelled"):
            break

    # 4. Print results
    print(f"\n[4/4] Results:")
    detail = api("GET", f"/eval/{run_id}")
    scores = detail["results"]["scores"]
    print(f"  Pass rate: {scores.get('pass_rate', 0) * 100:.0f}%")
    print(f"  Mean score: {scores.get('mean', 0):.2f}")

    if "result_items" in detail:
        print(f"\n  Per-case results:")
        for r in detail["result_items"]:
            status_icon = "pass" if r["status"] == "passed" else "FAIL"
            output = json.dumps(r["actual_output"]) if isinstance(r["actual_output"], dict) else str(r["actual_output"])
            print(f"    [{status_icon}] {output[:60]}  (score={r.get('score', '-')}, {r['latency_ms']}ms)")

    print(f"\n  View at: https://platform.traceway.ai/datasets/{dataset_id}")
    print(f"\n  Suggested next steps:")
    print(f"  - Run another eval with a different model (e.g. gpt-4o) and compare")
    print(f"  - Try 'contains' scoring instead of 'exact_match' for more lenient matching")
    print(f"  - Try 'llm_judge' scoring for nuanced evaluation")


if __name__ == "__main__":
    main()
