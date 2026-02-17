/**
 * Complex SDK test: multi-span traces, nested spans, filtering, analytics, export.
 *
 * Run: npx tsx tests/test_complex.ts
 * (from sdk/typescript/, no build step needed)
 */

import { LLMTrace, type SpanKind, type Span } from '../src/index.js';
import { statusKind } from '../src/types.js';

const BASE = process.env.LLMTRACE_URL ?? 'http://localhost:3000';
const client = new LLMTrace({ url: BASE });

function assert(cond: boolean, msg: string) {
  if (!cond) throw new Error(`ASSERT FAILED: ${msg}`);
}

async function testMultiSpanTraceWithKinds() {
  await client.clearAll();

  const traceId = await client.trace('agent-run', async (t) => {
    // 1. Read config
    await t.span('read-config', async (s) => {
      s.setOutput({ config: { model: 'gpt-4', temperature: 0.7 } });
    }, {
      kind: { type: 'fs_read', path: '/config.yaml', bytes_read: 1024 },
    });

    // 2. Read source
    await t.span('read-source', async (s) => {
      s.setOutput({ lines: 150 });
    }, {
      kind: { type: 'fs_read', path: '/src/main.py', bytes_read: 4096 },
    });

    // 3. LLM call
    await t.llmCall('inference', { model: 'gpt-4o', provider: 'openai' }, async (s) => {
      await new Promise(r => setTimeout(r, 50));
      s.setOutput({
        response: 'Here is the refactored code...',
        tokens: { input: 2000, output: 500 },
      });
    });

    // 4. Write output
    await t.span('write-output', async (s) => {
      s.setOutput({ written: true });
    }, {
      kind: { type: 'fs_write', path: '/src/main_refactored.py', file_version: 'sha256:abc123', bytes_written: 3500 },
    });

    // 5. Custom span
    await t.span('post-process', async (s) => {
      s.setOutput({ warnings: 0, errors: 0 });
    }, {
      kind: { type: 'custom', kind: 'validation', attributes: { linter: 'ruff' } },
    });

    return t.traceId;
  });

  // Verify all spans
  const result = await client.getTrace(traceId);
  assert(result.count === 5, `Expected 5 spans, got ${result.count}`);

  const names = new Set(result.spans.map(s => s.name));
  for (const expected of ['read-config', 'read-source', 'inference', 'write-output', 'post-process']) {
    assert(names.has(expected), `Missing span '${expected}'`);
  }

  // All completed
  for (const span of result.spans) {
    assert(statusKind(span.status) === 'completed', `Span ${span.name} is ${JSON.stringify(span.status)}`);
  }

  // Check kind parsing
  const byName = Object.fromEntries(result.spans.map(s => [s.name, s]));

  const readConfig = byName['read-config'];
  assert(readConfig.kind?.type === 'fs_read', 'read-config should be fs_read');
  assert((readConfig.kind as any).path === '/config.yaml', 'path mismatch');

  const inference = byName['inference'];
  assert(inference.kind?.type === 'llm_call', 'inference should be llm_call');
  assert((inference.kind as any).model === 'gpt-4o', 'model mismatch');

  const writeOutput = byName['write-output'];
  assert(writeOutput.kind?.type === 'fs_write', 'write-output should be fs_write');
  assert((writeOutput.kind as any).bytes_written === 3500, 'bytes_written mismatch');

  console.log('PASS: testMultiSpanTraceWithKinds');
  return traceId;
}

async function testNestedSpans() {
  const traceId = crypto.randomUUID();

  // Root span
  const root = await client.startSpan({
    traceId,
    name: 'root-task',
    kind: { type: 'custom', kind: 'orchestrator', attributes: {} },
  });

  // Child 1
  const child1 = await client.startSpan({
    traceId,
    parentId: root.id,
    name: 'child-1',
    kind: { type: 'custom', kind: 'subtask', attributes: {} },
    input: { step: 1 },
  });
  await client.completeSpan(child1.id, { done: true });

  // Child 2
  const child2 = await client.startSpan({
    traceId,
    parentId: root.id,
    name: 'child-2',
    kind: { type: 'custom', kind: 'subtask', attributes: {} },
    input: { step: 2 },
  });
  await client.completeSpan(child2.id, { done: true });

  // Complete root
  await client.completeSpan(root.id, { children_completed: 2 });

  const result = await client.getTrace(traceId);
  assert(result.count === 3, `Expected 3 spans, got ${result.count}`);

  const byName = Object.fromEntries(result.spans.map(s => [s.name, s]));
  assert(byName['root-task'].parent_id === null, 'root should have no parent');
  assert(byName['child-1'].parent_id === root.id, 'child-1 parent mismatch');
  assert(byName['child-2'].parent_id === root.id, 'child-2 parent mismatch');

  await client.deleteTrace(traceId);
  console.log('PASS: testNestedSpans');
}

async function testSpanFiltering() {
  await client.clearAll();

  // Trace A: LLM + file read
  const tidA = await client.trace('trace-a', async (t) => {
    await t.llmCall('call-gpt4', { model: 'gpt-4' }, async (s) => {
      s.setOutput({ text: 'hello' });
    });
    await t.span('read-file', async (s) => {
      s.setOutput({});
    }, {
      kind: { type: 'fs_read', path: '/a.txt', bytes_read: 100 },
    });
    return t.traceId;
  });

  // Trace B: different model
  await client.trace('trace-b', async (t) => {
    await t.llmCall('call-claude', { model: 'claude-3' }, async (s) => {
      s.setOutput({ text: 'hi' });
    });
    return t.traceId;
  });

  // Filter by kind
  const llmSpans = await client.getSpans({ kind: 'llm_call' });
  assert(llmSpans.count === 2, `Expected 2 LLM spans, got ${llmSpans.count}`);

  const fsSpans = await client.getSpans({ kind: 'fs_read' });
  assert(fsSpans.count === 1, `Expected 1 fs_read span, got ${fsSpans.count}`);

  // Filter by trace
  const traceASpans = await client.getSpans({ trace_id: tidA });
  assert(traceASpans.count === 2, `Expected 2 spans in trace A, got ${traceASpans.count}`);

  // Filter by name
  const gptSpans = await client.getSpans({ name_contains: 'gpt4' });
  assert(gptSpans.count === 1, `Expected 1 span matching 'gpt4', got ${gptSpans.count}`);

  // Filter by status
  const completed = await client.getSpans({ status: 'completed' });
  assert(completed.count === 3, `Expected 3 completed spans, got ${completed.count}`);

  console.log('PASS: testSpanFiltering');
}

async function testExportAndStats() {
  await client.clearAll();

  const traceId = await client.trace('export-test', async (t) => {
    await t.span('s1', async (s) => { s.setOutput({ v: 1 }); }, {
      kind: { type: 'custom', kind: 'task', attributes: {} },
    });
    await t.span('s2', async (s) => { s.setOutput({ v: 2 }); }, {
      kind: { type: 'custom', kind: 'task', attributes: {} },
    });
    return t.traceId;
  });

  // Stats
  const stats = await client.getStats();
  assert(stats.trace_count >= 1, `Expected >=1 traces, got ${stats.trace_count}`);
  assert(stats.span_count >= 2, `Expected >=2 spans, got ${stats.span_count}`);

  // Export all
  const exported = await client.exportJson();
  assert(traceId in exported.traces, 'Trace not in export');
  assert(exported.traces[traceId].length === 2, `Expected 2 spans in export, got ${exported.traces[traceId].length}`);

  // Export single trace
  const exportSingle = await client.exportJson(traceId);
  assert(traceId in exportSingle.traces, 'Trace not in single export');

  console.log('PASS: testExportAndStats');
}

async function testAnalyticsSummary() {
  await client.clearAll();

  await client.trace('analytics-test', async (t) => {
    await t.llmCall('call-1', { model: 'gpt-4', provider: 'openai' }, async (s) => {
      s.setOutput({ text: 'response' });
    });
    await t.llmCall('call-2', { model: 'claude-3', provider: 'anthropic' }, async (s) => {
      s.setOutput({ text: 'response2' });
    });
    await t.span('read', async (s) => { s.setOutput({}); }, {
      kind: { type: 'fs_read', path: '/f.txt', bytes_read: 50 },
    });
    return t.traceId;
  });

  // Analytics summary via raw fetch
  const summaryResp = await fetch(`${BASE}/analytics/summary`);
  assert(summaryResp.ok, `analytics/summary failed: ${summaryResp.status}`);
  const summary = await summaryResp.json();
  assert(summary.total_spans >= 3, `Expected >=3 spans, got ${summary.total_spans}`);
  assert(summary.total_llm_calls >= 2, `Expected >=2 LLM calls, got ${summary.total_llm_calls}`);
  assert(summary.models_used.length >= 2, `Expected >=2 models, got ${summary.models_used.length}`);
  assert(summary.models_used.includes('gpt-4'), 'Missing gpt-4');
  assert(summary.models_used.includes('claude-3'), 'Missing claude-3');

  // Flexible analytics query
  const analyticsResp = await fetch(`${BASE}/analytics`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      metrics: ['span_count', 'total_tokens'],
      group_by: ['model'],
      filter: { kind: 'llm_call' },
    }),
  });
  assert(analyticsResp.ok, `analytics query failed: ${analyticsResp.status}`);
  const analytics = await analyticsResp.json();
  assert('groups' in analytics, 'Missing groups in analytics response');
  assert('totals' in analytics, 'Missing totals in analytics response');

  console.log('PASS: testAnalyticsSummary');
}

async function testTraceLifecycle() {
  const traceId = await client.trace('lifecycle-test', async (t) => {
    await t.span('temp', async (s) => { s.setOutput({}); }, {
      kind: { type: 'custom', kind: 'task', attributes: {} },
    });
    return t.traceId;
  });

  // Read
  const result = await client.getTrace(traceId);
  assert(result.count === 1, `Expected 1 span, got ${result.count}`);

  // Health check
  const health = await client.getHealth();
  assert(health.status === 'ok', `Expected 'ok', got '${health.status}'`);

  // Delete
  await client.deleteTrace(traceId);

  // Verify gone
  try {
    await client.getTrace(traceId);
    assert(false, 'Should have thrown for deleted trace');
  } catch {
    // Expected — 404
  }

  console.log('PASS: testTraceLifecycle');
}

async function main() {
  await testMultiSpanTraceWithKinds();
  await testNestedSpans();
  await testSpanFiltering();
  await testExportAndStats();
  await testAnalyticsSummary();
  await testTraceLifecycle();
  console.log('\nAll complex tests passed!');
}

main().catch(e => {
  console.error(e);
  process.exit(1);
});
