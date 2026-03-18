"""Dataset + queue review example via Python SDK.

Usage:
    python examples/python/eval_example.py
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "sdk" / "python"))

from traceway import Traceway


def main() -> None:
    api_url = os.environ.get("TRACEWAY_URL", "http://localhost:4000")
    api_key = os.environ.get("TRACEWAY_API_KEY")

    with Traceway(url=api_url, api_key=api_key) as tw:
        ds = tw.create_dataset("curated-review-loop", "Simple review queue walkthrough")
        print(f"dataset: {ds.id}")

        dp1 = tw.create_datapoint(ds.id, {
            "type": "generic",
            "input": {"prompt": "What is 2+2?"},
            "expected_output": "4",
        })
        dp2 = tw.create_datapoint(ds.id, {
            "type": "generic",
            "input": {"prompt": "Capital of France?"},
            "expected_output": "Paris",
        })

        queue_items = tw.enqueue_datapoints(ds.id, [dp1.id, dp2.id])
        print(f"enqueued: {len(queue_items)}")

        if queue_items:
            claimed = tw.claim_queue_item(queue_items[0].id, claimed_by="example-reviewer")
            done = tw.submit_queue_item(claimed.id, edited_data={"approved": True, "notes": "Looks good"})
            print(f"reviewed item: {done.id} status={done.status}")

        print(f"open: {os.environ.get('TRACEWAY_UI_URL', 'http://localhost:5173')}/datasets/{ds.id}")


if __name__ == "__main__":
    main()
