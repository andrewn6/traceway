/**
 * Traceway MCP Client Example (TypeScript)
 * 
 * This demonstrates how to use the Traceway MCP tools from code.
 * For use with Claude Code, Cursor, or any MCP-compatible client,
 * configure the MCP server URL in your client config.
 * 
 * Run:
 *   npx tsx examples/typescript/mcp_client.ts
 * 
 * Environment:
 *   TRACEWAY_MCP_URL=http://localhost:4000/v1/mcp
 *   TRACEWAY_API_KEY=your_api_key
 */

const MCP_URL = process.env.TRACEWAY_MCP_URL || 'http://localhost:4000/v1/mcp';
const API_KEY = process.env.TRACEWAY_API_KEY;

async function rpc(method: string, params: unknown, id: number) {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  if (API_KEY) headers['Authorization'] = `Bearer ${API_KEY}`;

  const res = await fetch(MCP_URL, {
    method: 'POST',
    headers,
    body: JSON.stringify({ jsonrpc: '2.0', id, method, params }),
  });

  if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
  const body = await res.json();
  if (body.error) throw new Error(`${body.error.code}: ${body.error.message}`);
  return body.result;
}

async function main() {
  console.log('Connecting to:', MCP_URL);

  // Initialize
  const init = await rpc('initialize', {
    protocolVersion: '2024-11-05',
    capabilities: {},
    clientInfo: { name: 'traceway-example', version: '0.1.0' },
  }, 1);
  console.log('\nServer:', init.serverInfo);

  // List available tools
  const tools = await rpc('tools/list', {}, 2);
  console.log('\nAvailable tools:', tools.tools.map((t: { name: string; description: string }) => 
    `  - ${t.name}: ${t.description.slice(0, 60)}...`
  ).join('\n'));

  // Example 1: Search traces
  console.log('\n--- Search traces (last 24h) ---');
  const search = await rpc('tools/call', {
    name: 'search_traces',
    arguments: { query: 'since:24h', limit: 3 },
  }, 3);
  console.log(search.content?.[0]?.text || '(empty)');

  // Example 2: List sessions
  console.log('\n--- List active sessions ---');
  const sessions = await rpc('tools/call', {
    name: 'list_sessions',
    arguments: { limit: 5 },
  }, 4);
  console.log(sessions.content?.[0]?.text || '(no sessions)');

  // Example 3: Search memory (LLM inputs/outputs)
  console.log('\n--- Search agent memory ---');
  const memory = await rpc('tools/call', {
    name: 'search_memory',
    arguments: { query: 'error', limit: 3, since: '7d' },
  }, 5);
  console.log(memory.content?.[0]?.text || '(no matches)');

  // Example 4: Tag a trace (organize into session)
  // Replace TRACE_ID with actual trace ID
  // const tagResult = await rpc('tools/call', {
  //   name: 'tag_trace',
  //   arguments: { trace_id: 'TRACE_ID', tags: ['session_id:my-session', 'priority:high'] },
  // }, 6);
  // console.log('\nTagged:', tagResult.content);

  console.log('\nDone!');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
