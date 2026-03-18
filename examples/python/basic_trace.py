"""Minimal Python SDK trace example.

Usage:
    python examples/python/basic_trace.py
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "sdk" / "python"))

from traceway import CustomKind, Traceway


def main() -> None:
    api_url = os.environ.get("TRACEWAY_URL", "http://localhost:4000")
    api_key = os.environ.get("TRACEWAY_API_KEY")

    with Traceway(url=api_url, api_key=api_key) as tw:
        with tw.trace("python-quickstart") as t:
            with t.span(
                "load-user-request",
                kind=CustomKind(kind="app", attributes={"stage": "input"}),
                input={"prompt": "Summarize feature request #42"},
            ) as s:
                s.set_output({"ok": True})

            with t.llm_call(
                "draft-summary",
                model="gpt-4o-mini",
                provider="openai",
                input={"ticket_id": 42},
            ) as s:
                s.set_output(
                    {
                        "content": "Request #42 asks for better timeline readability and denser rows.",
                        "input_tokens": 53,
                        "output_tokens": 17,
                        "model": "gpt-4o-mini",
                    }
                )

            print(f"Created trace: {t.trace_id}")
            print(f"Open: {os.environ.get('TRACEWAY_UI_URL', 'http://localhost:5173')}/traces/{t.trace_id}")


if __name__ == "__main__":
    main()
