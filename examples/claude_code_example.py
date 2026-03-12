"""
Claude Code Observability Example
=================================

Shows a realistic "code agent" trace using Traceway spans + file tracking:
- reads source + tests
- plans and generates a patch (LLM spans)
- writes code (fs_write span)
- runs tests before/after fix
- queries file versions and file->trace links

Usage:
    python examples/claude_code_example.py

Prerequisites:
    pip install traceway python-dotenv
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "sdk", "python"))

from dotenv import load_dotenv

load_dotenv(Path(__file__).resolve().parent.parent / ".env")

from traceway import CustomKind, FsReadKind, FsWriteKind, Traceway


TRACEWAY_URL = os.environ.get("TRACEWAY_URL", "http://localhost:4000")
TRACEWAY_UI_URL = os.environ.get("TRACEWAY_UI_URL", "http://localhost:5173")
TRACEWAY_API_KEY = os.environ.get("TRACEWAY_API_KEY")
MODEL = "claude-sonnet-4-20250514"


BUGGY_SOURCE = """def apply_coupon(subtotal_cents: int, coupon_percent: int) -> int:
    discount = subtotal_cents * (coupon_percent / 100)
    return int(subtotal_cents - discount)
"""


FIXED_SOURCE = """def apply_coupon(subtotal_cents: int, coupon_percent: int) -> int:
    if coupon_percent < 0 or coupon_percent > 100:
        raise ValueError("coupon_percent must be between 0 and 100")

    discount = subtotal_cents * (coupon_percent / 100)
    return max(0, int(round(subtotal_cents - discount)))
"""


TEST_SOURCE = """import unittest

from checkout_pricing import apply_coupon


class TestCheckoutPricing(unittest.TestCase):
    def test_apply_coupon_happy_path(self):
        self.assertEqual(apply_coupon(10000, 15), 8500)

    def test_apply_coupon_invalid_percentage(self):
        with self.assertRaises(ValueError):
            apply_coupon(10000, 150)


if __name__ == "__main__":
    unittest.main()
"""


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def read_file_with_span(t, path: Path, tracked_path: str) -> str:
    content = path.read_text(encoding="utf-8")
    file_hash = sha256_text(content)
    with t.span(
        f"read:{path.name}",
        kind=FsReadKind(path=tracked_path, file_version=file_hash, bytes_read=len(content.encode("utf-8"))),
        input={"path": tracked_path},
    ) as span:
        span.set_output(
            {
                "path": tracked_path,
                "file_version": file_hash,
                "file_content": content,
                "preview": "\n".join(content.splitlines()[:6]),
            }
        )
    return content


def write_file_with_span(t, path: Path, tracked_path: str, content: str, reason: str, parent_id: str | None = None) -> str:
    file_hash = sha256_text(content)
    with t.span(
        f"write:{path.name}",
        parent_id=parent_id,
        kind=FsWriteKind(path=tracked_path, file_version=file_hash, bytes_written=len(content.encode("utf-8"))),
        input={"path": tracked_path, "reason": reason},
    ) as span:
        path.write_text(content, encoding="utf-8")
        span.set_output(
            {
                "path": tracked_path,
                "file_version": file_hash,
                "file_content": content,
                "bytes": len(content.encode("utf-8")),
            }
        )
    return file_hash


def run_tests(workspace_dir: Path) -> tuple[int, str]:
    proc = subprocess.run(
        [sys.executable, "-m", "unittest", "-q"],
        cwd=workspace_dir,
        capture_output=True,
        text=True,
        check=False,
    )
    output = (proc.stdout + "\n" + proc.stderr).strip()
    return proc.returncode, output


def run_dataset_approval_flow(tw: Traceway, trace_id: str) -> None:
    dataset_name = f"claude-code-approvals-{datetime.now(timezone.utc).strftime('%Y%m%d-%H%M%S')}"
    dataset = tw.create_dataset(
        dataset_name,
        description="Approval queue for proposed checkout pricing hotfixes",
    )

    draft_fix = tw.create_datapoint(
        dataset.id,
        {
            "type": "generic",
            "input": {
                "file": "claude_code_example/checkout_pricing.py",
                "proposal": "Validate coupon percent bounds and clamp total >= 0",
            },
            "target": {"approved": True},
            "metadata": {"trace_id": trace_id, "stage": "candidate-fix"},
        },
    )

    regression_case = tw.create_datapoint(
        dataset.id,
        {
            "type": "generic",
            "input": {
                "file": "claude_code_example/test_checkout_pricing.py",
                "proposal": "Add invalid percentage test coverage",
            },
            "target": {"approved": True},
            "metadata": {"trace_id": trace_id, "stage": "test-plan"},
        },
    )

    tw.enqueue_datapoints(dataset.id, [draft_fix.id, regression_case.id])
    queue = tw.list_queue(dataset.id)

    print("\nApproval dataset flow:")
    print(f"Dataset: {dataset.name} ({dataset.id})")
    print(f"Dataset URL: {TRACEWAY_UI_URL}/datasets/{dataset.id}")
    print(f"Datapoints created: {len([draft_fix, regression_case])}")
    print(f"Queue items: {queue.count}")

    if queue.items:
        claimed = tw.claim_queue_item(queue.items[0].id, claimed_by="qa-reviewer")
        submitted = tw.submit_queue_item(
            claimed.id,
            edited_data={
                "approved": True,
                "notes": "Validation + clamping look good; tests pass.",
                "decision_source": "sdk-example",
            },
        )
        print(
            "Submitted review: "
            f"item={submitted.id} status={submitted.status} reviewer={submitted.claimed_by}"
        )

    queue_after = tw.list_queue(dataset.id)
    completed = sum(1 for i in queue_after.items if i.status == "completed")
    pending = sum(1 for i in queue_after.items if i.status == "pending")
    claimed = sum(1 for i in queue_after.items if i.status == "claimed")
    print(f"Queue status -> pending={pending} claimed={claimed} completed={completed}")


def main() -> None:
    workspace_dir = Path(tempfile.mkdtemp(prefix="traceway-claude-code-"))
    source_path = workspace_dir / "checkout_pricing.py"
    test_path = workspace_dir / "test_checkout_pricing.py"
    tracked_source_path = "claude_code_example/checkout_pricing.py"
    tracked_test_path = "claude_code_example/test_checkout_pricing.py"

    print(f"Traceway API: {TRACEWAY_URL}")
    print(f"Workspace: {workspace_dir}")
    print("Creating an observed code-agent run...\n")

    with Traceway(url=TRACEWAY_URL, api_key=TRACEWAY_API_KEY) as tw:
        with tw.trace("claude-code-example-checkout-hotfix") as t:
            with t.span(
                "seed-workspace",
                kind=CustomKind(kind="agent", attributes={"step": "setup"}),
                input={"workspace": str(workspace_dir)},
            ) as span:
                source_path.write_text(BUGGY_SOURCE, encoding="utf-8")
                test_path.write_text(TEST_SOURCE, encoding="utf-8")
                span.set_output({"files": [source_path.name, test_path.name]})

            source_before = read_file_with_span(t, source_path, tracked_source_path)
            tests_before = read_file_with_span(t, test_path, tracked_test_path)

            with t.span(
                "run-tests-before",
                kind=CustomKind(kind="exec", attributes={"tool": "python -m unittest -q"}),
                input={"cwd": str(workspace_dir)},
            ) as span:
                code_before, output_before = run_tests(workspace_dir)
                span.set_output({"exit_code": code_before, "output": output_before[:1200]})

            user_request = (
                "Fix checkout coupon calculation so invalid coupon percentages are rejected "
                "and totals cannot go negative, while keeping normal discount behavior correct."
            )

            with t.llm_call(
                "claude-plan",
                model=MODEL,
                provider="anthropic",
                input={
                    "instruction": user_request,
                    "files": {
                        tracked_source_path: source_before,
                        tracked_test_path: tests_before,
                    },
                },
            ) as span:
                plan = (
                    "1) Validate coupon_percent is between 0 and 100. "
                    "2) Keep standard discount math for valid coupons and clamp total to >= 0. "
                    "3) Re-run checkout tests to confirm the hotfix."
                )
                span.set_output(
                    {
                        "content": plan,
                        "input_tokens": 682,
                        "output_tokens": 93,
                        "model": MODEL,
                    }
                )

            with t.llm_call(
                "claude-generate-patch",
                model=MODEL,
                provider="anthropic",
                input={"instruction": user_request, "plan": plan, "target_file": tracked_source_path},
            ) as span:
                patch = """@@
+    if coupon_percent < 0 or coupon_percent > 100:
+        raise ValueError(\"coupon_percent must be between 0 and 100\")
+
+    discount = subtotal_cents * (coupon_percent / 100)
+    return max(0, int(round(subtotal_cents - discount)))
"""
                span.set_output(
                    {
                        "content": patch,
                        "input_tokens": 421,
                        "output_tokens": 74,
                        "model": MODEL,
                    }
                )
                write_file_with_span(
                    t,
                    source_path,
                    tracked_source_path,
                    FIXED_SOURCE,
                    reason="apply agent patch",
                    parent_id=span.span_id,
                )

            with t.span(
                "run-tests-after",
                kind=CustomKind(kind="exec", attributes={"tool": "python -m unittest -q"}),
                input={"cwd": str(workspace_dir)},
            ) as span:
                code_after, output_after = run_tests(workspace_dir)
                span.set_output({"exit_code": code_after, "output": output_after[:1200]})

            with t.span(
                "agent-summary",
                kind=CustomKind(kind="agent", attributes={"step": "finalize"}),
                input={"request": user_request},
            ) as span:
                span.set_output(
                    {
                        "result": "success" if code_after == 0 else "failed",
                        "tests_before": code_before,
                        "tests_after": code_after,
                        "files_changed": [tracked_source_path],
                    }
                )

            trace_id = t.trace_id

        print("Done.\n")
        print(f"Trace ID: {trace_id}")
        print(f"Trace URL: {TRACEWAY_UI_URL}/traces/{trace_id}")

        try:
            run_dataset_approval_flow(tw, trace_id)
        except Exception as err:
            print("\nCould not run dataset approval flow:")
            print(f"{err}")
            print(
                "Hint: dataset endpoints in this backend require an authenticated session; "
                "trace/span ingestion still works with API key mode."
            )

        try:
            versions = tw.file_versions(tracked_source_path)
            print(f"\nTracked versions for {tracked_source_path}: {len(versions)}")
            if versions:
                latest = versions[0]
                print(
                    "Latest version: "
                    f"hash={latest.hash[:12]} size={latest.size} created_by_span={latest.created_by_span}"
                )
        except Exception as err:
            print(f"\nCould not fetch file versions: {err}")

        try:
            links = tw.file_traces(tracked_source_path)
            print("\nTrace links for the tracked file:")
            print(json.dumps(links, indent=2)[:1200])
        except Exception as err:
            print(f"Could not fetch file trace links: {err}")


if __name__ == "__main__":
    main()
