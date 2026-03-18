/**
 * Minimal TypeScript trace example using raw HTTP.
 *
 * Run:
 *   TRACEWAY_URL=http://localhost:4000 TRACEWAY_API_KEY=tw_sk_... npx tsx examples/typescript/quick_trace.ts
 */

const BASE_URL = (process.env.TRACEWAY_URL || 'http://localhost:4000').replace(/\/$/, '');
const API_KEY = process.env.TRACEWAY_API_KEY;

async function api<T>(method: string, path: string, body?: unknown): Promise<T> {
  const headers: Record<string, string> = {};
  if (API_KEY) headers.Authorization = `Bearer ${API_KEY}`;
  if (body !== undefined) headers['Content-Type'] = 'application/json';

  const res = await fetch(`${BASE_URL}${path}`, {
    method,
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    throw new Error(`${method} ${path} failed: ${res.status} ${await res.text()}`);
  }
  const text = await res.text();
  return (text ? JSON.parse(text) : undefined) as T;
}

async function main() {
  const trace = await api<{ id: string }>('POST', '/traces', { name: 'ts-quick-trace' });
  const created = await api<{ id: string; trace_id: string }>('POST', '/spans', {
    trace_id: trace.id,
    name: 'prepare-response',
    kind: { type: 'custom', kind: 'example' },
    input: { prompt: 'hello' },
  });

  await api('POST', `/spans/${created.id}/complete`, {
    output: { message: 'done' },
  });

  console.log(`Trace created: ${trace.id}`);
  console.log(`Open: ${(process.env.TRACEWAY_UI_URL || 'http://localhost:5173')}/traces/${trace.id}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
