"""
Basic Trace Example
====================

Creates a single trace with a few LLM calls to demonstrate:
- Tracing multi-step LLM pipelines
- Automatic token counting and cost estimation
- Nested spans with parent/child relationships

Prerequisites:
    pip install traceway openai anthropic python-dotenv

Usage:
    python examples/basic_trace.py
"""
from __future__ import annotations

import os
import sys
from pathlib import Path

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "sdk", "python"))

from dotenv import load_dotenv
load_dotenv(Path(__file__).resolve().parent.parent / ".env")

from traceway import Traceway, LlmCallKind

# ── Config ────────────────────────────────────────────────────────────

USE_OPENAI = bool(os.environ.get("OPENAI_API_KEY"))
USE_ANTHROPIC = bool(os.environ.get("ANTHROPIC_API_KEY"))

if not USE_OPENAI and not USE_ANTHROPIC:
    print("Set at least one of OPENAI_API_KEY or ANTHROPIC_API_KEY in .env")
    sys.exit(1)


# ── LLM helpers ───────────────────────────────────────────────────────

def call_openai(model: str, messages: list[dict], **kwargs) -> dict:
    from openai import OpenAI
    resp = OpenAI().chat.completions.create(model=model, messages=messages, **kwargs)
    return {
        "content": resp.choices[0].message.content,
        "input_tokens": resp.usage.prompt_tokens,
        "output_tokens": resp.usage.completion_tokens,
        "model": resp.model,
    }


def call_anthropic(model: str, messages: list[dict], **kwargs) -> dict:
    import anthropic
    # Extract system messages into top-level param (Anthropic API requirement)
    system_msgs = [m["content"] for m in messages if m["role"] == "system"]
    non_system = [m for m in messages if m["role"] != "system"]
    if system_msgs:
        kwargs["system"] = "\n".join(system_msgs)
    resp = anthropic.Anthropic().messages.create(
        model=model, messages=non_system, **kwargs
    )
    return {
        "content": resp.content[0].text,
        "input_tokens": resp.usage.input_tokens,
        "output_tokens": resp.usage.output_tokens,
        "model": resp.model,
    }


def call_llm(model: str, messages: list[dict], **kwargs) -> dict:
    if model.startswith("claude"):
        if not USE_ANTHROPIC:
            raise RuntimeError(f"ANTHROPIC_API_KEY not set for {model}")
        return call_anthropic(model, messages, **kwargs)
    if not USE_OPENAI:
        raise RuntimeError(f"OPENAI_API_KEY not set for {model}")
    return call_openai(model, messages, **kwargs)


# ── Main ──────────────────────────────────────────────────────────────

def main():
    tw = Traceway(
        url=os.environ.get("TRACEWAY_URL", "https://api.traceway.ai"),
        api_key=os.environ.get("TRACEWAY_API_KEY"),
    )

    # Pick models based on available keys
    fast_model = "gpt-4o-mini" if USE_OPENAI else "claude-sonnet-4-20250514"
    smart_model = "claude-sonnet-4-20250514" if USE_ANTHROPIC else "gpt-4o"
    fast_provider = "openai" if USE_OPENAI else "anthropic"
    smart_provider = "anthropic" if USE_ANTHROPIC else "openai"

    print(f"Using: {fast_model} (fast), {smart_model} (smart)")

    with tw.trace("summarize-and-critique") as t:
        # Step 1: Generate a summary
        print("[1/3] Generating summary...")
        with t.llm_call("summarize", model=fast_model, provider=fast_provider,
                         input={"task": "summarize article"}) as span:
            result = call_llm(fast_model, [
                {"role": "system", "content": "You are a concise summarizer. Respond in 2-3 sentences."},
                {"role": "user", "content": "Summarize the key ideas behind transformer neural networks and why they revolutionized NLP."},
            ], max_tokens=200, temperature=0.3)
            span.set_output(result)
            summary = result["content"]
            print(f"   Tokens: {result['input_tokens']} in / {result['output_tokens']} out")

        # Step 2: Critique the summary with a stronger model
        print("[2/3] Critiquing summary...")
        with t.llm_call("critique", model=smart_model, provider=smart_provider,
                         input={"summary": summary}) as span:
            result = call_llm(smart_model, [
                {"role": "system", "content": "You are a critical reviewer. Rate the summary 1-10 and explain what could be improved. Be concise."},
                {"role": "user", "content": f"Rate this summary:\n\n{summary}"},
            ], max_tokens=300, temperature=0.4)
            span.set_output(result)
            critique = result["content"]
            print(f"   Tokens: {result['input_tokens']} in / {result['output_tokens']} out")

        # Step 3: Revise based on feedback
        print("[3/3] Revising...")
        with t.llm_call("revise", model=fast_model, provider=fast_provider,
                         input={"summary": summary, "critique": critique}) as span:
            result = call_llm(fast_model, [
                {"role": "system", "content": "You are a concise summarizer. Improve the summary based on the feedback. 2-3 sentences max."},
                {"role": "user", "content": f"Original summary:\n{summary}\n\nFeedback:\n{critique}\n\nWrite an improved version:"},
            ], max_tokens=200, temperature=0.3)
            span.set_output(result)
            print(f"   Tokens: {result['input_tokens']} in / {result['output_tokens']} out")

        print(f"\nTrace ID: {t.trace_id}")
        print(f"View at: https://platform.traceway.ai/traces/{t.trace_id}")

    tw.close()


if __name__ == "__main__":
    main()
