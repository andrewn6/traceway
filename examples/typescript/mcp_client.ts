/**
 * MCP over HTTP example (TypeScript).
 *
 * Run local:
 *   TRACEWAY_MCP_URL=http://localhost:4000/v1/mcp npx tsx examples/typescript/mcp_client.ts
 *
 * Run cloud:
 *   TRACEWAY_MCP_URL=https://api.traceway.ai/v1/mcp TRACEWAY_API_KEY=tw_sk_... npx tsx examples/typescript/mcp_client.ts
 */

const MCP_URL = process.env.TRACEWAY_MCP_URL || 'http://localhost:4000/v1/mcp';
const API_KEY = process.env.TRACEWAY_API_KEY;

async function rpc(method: string, params: unknown, id: number) {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  if (API_KEY) headers.Authorization = `Bearer ${API_KEY}`;

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
  const init = await rpc('initialize', {
    protocolVersion: '2024-11-05',
    capabilities: {},
    clientInfo: { name: 'traceway-ts-example', version: '0.1.0' },
  }, 1);
  console.log('server:', init.serverInfo);

  const tools = await rpc('tools/list', {}, 2);
  console.log('tools:', tools.tools.map((t: { name: string }) => t.name));

  const search = await rpc('tools/call', {
    name: 'search_traces',
    arguments: { query: 'since:24h', limit: 5 },
  }, 3);

  console.log('\nsearch text:\n');
  console.log(search.content?.[0]?.text || '(empty)');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
