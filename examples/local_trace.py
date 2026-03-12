"""
Create a local demo of a Claude-powered customer support agent.

This script is intentionally business-readable for non-technical demos:
- each trace is one customer conversation
- spans show the end-to-end agent workflow
- LLM steps use the Anthropic API (real token usage)

Usage:
    python examples/local_trace.py
    python examples/local_trace.py --runs 15

Prerequisites:
    pip install python-dotenv anthropic
"""

from __future__ import annotations

import argparse
import os
import random
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "sdk", "python"))

from dotenv import load_dotenv

load_dotenv(Path(__file__).resolve().parent.parent / ".env")
load_dotenv(Path(__file__).resolve().parent.parent / "backend" / "app" / ".env")

from traceway import CustomKind, Traceway  # type: ignore[import-not-found]


TRACEWAY_URL = os.environ.get("TRACEWAY_URL", "http://localhost:4000")
TRACEWAY_UI_URL = os.environ.get("TRACEWAY_UI_URL", "http://localhost:5173")
TRACEWAY_API_KEY = os.environ.get("TRACEWAY_API_KEY")
ANTHROPIC_API_KEY = os.environ.get("ANTHROPIC_API_KEY")
CLAUDE_MODEL = os.environ.get("CLAUDE_MODEL", "claude-sonnet-4-20250514")


@dataclass
class SupportScenario:
    customer_name: str
    issue: str
    urgency: str
    plan: str


SCENARIOS: list[SupportScenario] = [
    SupportScenario("Maya", "My shipment is 3 days late and this is a birthday gift.", "high", "expedite-and-credit"),
    SupportScenario("Chris", "I was charged twice for the same order.", "high", "refund-duplicate"),
    SupportScenario("Ava", "I need to change my delivery address after checkout.", "medium", "address-change"),
    SupportScenario("Noah", "The product arrived damaged and I need a replacement.", "high", "replacement"),
    SupportScenario("Liam", "I want to return this item but lost the original box.", "low", "return-exception"),
    SupportScenario("Emma", "Can you pause my subscription for two months?", "medium", "subscription-pause"),
]


def call_claude(messages: list[dict[str, str]], system: str) -> dict[str, Any]:
    import anthropic

    client = anthropic.Anthropic(api_key=ANTHROPIC_API_KEY)
    resp: Any = client.messages.create(  # type: ignore[call-overload]
        model=CLAUDE_MODEL,
        system=system,
        messages=messages,  # type: ignore[arg-type]
        max_tokens=260,
        temperature=0.3,
    )
    text = ""
    if resp.content and len(resp.content) > 0:
        text = getattr(resp.content[0], "text", "")
    return {
        "content": text,
        "input_tokens": resp.usage.input_tokens,
        "output_tokens": resp.usage.output_tokens,
        "model": resp.model,
    }


def mock_account_lookup() -> dict[str, Any]:
    order_total = random.choice([39.99, 72.40, 119.00, 14.50])
    delay_days = random.choice([0, 1, 2, 3, 4])
    return {
        "customer_tier": random.choice(["standard", "plus", "vip"]),
        "order_total": order_total,
        "delay_days": delay_days,
        "last_csat": random.choice(["good", "neutral", "poor"]),
    }


def run_one_trace(tw: Traceway, idx: int) -> str:
    scenario = random.choice(SCENARIOS)
    trace_name = f"claude-support-agent-{idx:03d}"

    with tw.trace(trace_name) as t:
        with t.span(
            "receive-customer-request",
            kind=CustomKind(kind="support", attributes={"stage": "intake"}),
            input={
                "customer": scenario.customer_name,
                "issue": scenario.issue,
                "urgency": scenario.urgency,
            },
        ) as span:
            span.set_output({"ticket_id": f"T-{10000 + idx}", "channel": random.choice(["chat", "email"])})

        with t.llm_call(
            "claude-understand-intent",
            model=CLAUDE_MODEL,
            provider="anthropic",
            input={"issue": scenario.issue, "urgency": scenario.urgency},
        ) as span:
            analysis = call_claude(
                messages=[
                    {
                        "role": "user",
                        "content": (
                            "Classify this customer support issue and return: "
                            "category, risk level, and a plain-English summary in 2 bullets.\n\n"
                            f"Customer message: {scenario.issue}"
                        ),
                    }
                ],
                system="You are a senior customer support operations assistant.",
            )
            span.set_output(analysis)

        with t.span(
            "lookup-account-context",
            kind=CustomKind(kind="tool", attributes={"tool": "crm_lookup"}),
            input={"customer": scenario.customer_name},
        ) as span:
            account = mock_account_lookup()
            span.set_output(account)

        with t.llm_call(
            "claude-decide-resolution",
            model=CLAUDE_MODEL,
            provider="anthropic",
            input={
                "issue": scenario.issue,
                "urgency": scenario.urgency,
                "customer_context": account,
                "policy_hint": scenario.plan,
            },
        ) as span:
            decision = call_claude(
                messages=[
                    {
                        "role": "user",
                        "content": (
                            "Given this support case, propose a resolution with: "
                            "(1) action, (2) customer impact, (3) estimated business cost.\n\n"
                            f"Issue: {scenario.issue}\n"
                            f"Urgency: {scenario.urgency}\n"
                            f"Account context: {account}"
                        ),
                    }
                ],
                system="You balance customer happiness with operational cost.",
            )
            span.set_output(decision)

        with t.llm_call(
            "claude-draft-customer-reply",
            model=CLAUDE_MODEL,
            provider="anthropic",
            input={"issue": scenario.issue, "decision_summary": decision["content"][:500]},
        ) as span:
            draft_reply = call_claude(
                messages=[
                    {
                        "role": "user",
                        "content": (
                            "Write a short, empathetic customer message in plain English. "
                            "Keep it under 120 words and include next steps.\n\n"
                            f"Issue: {scenario.issue}\n"
                            f"Resolution: {decision['content']}"
                        ),
                    }
                ],
                system="You write clear and friendly support responses.",
            )
            span.set_output(draft_reply)

        approved = random.random() > 0.14
        with t.span(
            "human-approval-check",
            kind=CustomKind(kind="review", attributes={"stage": "qa"}),
            input={"draft_preview": draft_reply["content"][:250]},
        ) as span:
            span.set_output(
                {
                    "approved": approved,
                    "reviewer": random.choice(["ops-lead", "qa-manager", "support-supervisor"]),
                    "reason": "ready" if approved else "needs_policy_clarification",
                }
            )

        with t.span(
            "send-customer-response",
            kind=CustomKind(kind="support", attributes={"stage": "delivery"}),
            input={"approved": approved},
        ) as span:
            span.set_output(
                {
                    "status": "sent" if approved else "held",
                    "follow_up_required": not approved,
                    "estimated_csat": random.choice(["high", "medium", "high", "medium", "low"]),
                }
            )

        return t.trace_id


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Generate local Claude support-agent traces")
    parser.add_argument("--runs", type=int, default=12, help="Number of customer-support traces to generate")
    return parser.parse_args()


def main() -> None:
    args = parse_args()

    if not TRACEWAY_API_KEY:
        print("Missing TRACEWAY_API_KEY. Create one in Traceway settings.")
        sys.exit(1)
    if not ANTHROPIC_API_KEY:
        print("Missing ANTHROPIC_API_KEY. Set it in your .env file.")
        sys.exit(1)

    print(f"Traceway API: {TRACEWAY_URL}")
    print(f"Claude model: {CLAUDE_MODEL}")
    print(f"Generating {args.runs} support-agent traces...\n")

    started = time.time()
    trace_ids: list[str] = []

    with Traceway(url=TRACEWAY_URL, api_key=TRACEWAY_API_KEY) as tw:
        for i in range(args.runs):
            trace_id = run_one_trace(tw, i)
            trace_ids.append(trace_id)
            if i > 0 and i % 4 == 0:
                print(f"  created {i + 1}/{args.runs} traces...")

    elapsed = time.time() - started
    print("\nDone.")
    print(f"  traces created: {len(trace_ids)}")
    print(f"  elapsed:        {elapsed:.1f}s")
    if trace_ids:
        print(f"  latest trace:   {TRACEWAY_UI_URL}/traces/{trace_ids[-1]}")
        print(f"  traces list:    {TRACEWAY_UI_URL}/traces")


if __name__ == "__main__":
    main()
