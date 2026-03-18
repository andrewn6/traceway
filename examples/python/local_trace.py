"""Generate many synthetic traces quickly for local demos.

Usage:
    python examples/python/local_trace.py --runs 25
"""

from __future__ import annotations

import argparse
import os
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "sdk" / "python"))

from traceway import CustomKind, Traceway


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=int, default=10)
    args = parser.parse_args()

    api_url = os.environ.get("TRACEWAY_URL", "http://localhost:4000")
    api_key = os.environ.get("TRACEWAY_API_KEY")

    with Traceway(url=api_url, api_key=api_key) as tw:
        for i in range(args.runs):
            with tw.trace(f"support-flow-{i:03d}") as t:
                with t.span("receive", kind=CustomKind(kind="support"), input={"ticket": i}) as s:
                    s.set_output({"channel": random.choice(["chat", "email"])})

                latency_ms = random.randint(100, 2400)
                with t.llm_call("classify", model="gpt-4o-mini", provider="openai") as s:
                    s.set_output(
                        {
                            "content": "classified",
                            "input_tokens": random.randint(20, 120),
                            "output_tokens": random.randint(10, 50),
                            "latency_ms": latency_ms,
                            "model": "gpt-4o-mini",
                        }
                    )

                with t.span("send-response", kind=CustomKind(kind="support")) as s:
                    s.set_output({"status": "sent"})

            if (i + 1) % 5 == 0:
                print(f"created {i + 1}/{args.runs} traces")

    print("done")


if __name__ == "__main__":
    main()
