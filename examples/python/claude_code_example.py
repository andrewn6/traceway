"""MCP client example (Python HTTP JSON-RPC).

Usage:
    python examples/python/claude_code_example.py
"""

from __future__ import annotations

import json
import os

import httpx


MCP_URL = os.environ.get("TRACEWAY_MCP_URL", "http://localhost:4000/v1/mcp")
API_KEY = os.environ.get("TRACEWAY_API_KEY")


def rpc(method: str, params: dict | None = None, request_id: int = 1) -> dict:
    headers = {"Content-Type": "application/json"}
    if API_KEY:
        headers["Authorization"] = f"Bearer {API_KEY}"

    payload = {
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": params or {},
    }
    res = httpx.post(MCP_URL, headers=headers, json=payload, timeout=30)
    res.raise_for_status()
    body = res.json()
    if "error" in body:
        raise RuntimeError(str(body["error"]))
    return body["result"]


def main() -> None:
    init = rpc("initialize", {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "example", "version": "0.1.0"}}, 1)
    print("initialized:", init.get("serverInfo", {}))

    tools = rpc("tools/list", {}, 2)
    print("tools:", [t["name"] for t in tools.get("tools", [])])

    search = rpc("tools/call", {"name": "search_traces", "arguments": {"query": "status:failed since:24h", "limit": 5}}, 3)
    print("search result text:\n")
    print(search.get("content", [{}])[0].get("text", ""))

    print("\nstructured sample:\n")
    print(json.dumps(search.get("structuredContent", {}), indent=2)[:1500])


if __name__ == "__main__":
    main()
