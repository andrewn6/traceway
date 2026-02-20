/**
 * Simple SDK test: create a trace, add a span, complete it, verify.
 *
 * Run: npx tsx tests/test_simple.ts
 * (from sdk/typescript/, no build step needed)
 */

import { Traceway, type SpanKind } from '../src/index.js';
import { statusKind, statusError } from '../src/types.js';

const BASE = process.env.TRACEWAY_URL ?? 'http://localhost:3000';
const client = new Traceway({ url: BASE });

function assert(cond: boolean, msg: string) {
  if (!cond) throw new Error(`ASSERT FAILED: ${msg}`);
}

async function testCreateAndCompleteSpan() {
  // Clean slate
  await client.clearAll();

  // Create a trace with a span
  const traceId = await client.trace('simple-test', async (t) => {
    await t.span('step-1', async (s) => {
      s.setOutput({ answer: 42 });
    }, {
      kind: { type: 'custom', kind: 'task', attributes: { key: 'val' } },
    });
    return t.traceId;
  });

  // Verify trace exists
  const traces = await client.getTraces();
  assert(traces.count >= 1, `Expected >=1 traces, got ${traces.count}`);
  const found = traces.traces.some(tr => tr.id === traceId);
  assert(found, `Trace ${traceId} not in trace list`);

  // Verify span
  const spanList = await client.getTrace(traceId);
  assert(spanList.count === 1, `Expected 1 span, got ${spanList.count}`);
  const span = spanList.spans[0];
  assert(span.name === 'step-1', `Expected name 'step-1', got '${span.name}'`);
  assert(statusKind(span.status) === 'completed', `Expected completed, got ${JSON.stringify(span.status)}`);
  assert(JSON.stringify(span.output) === JSON.stringify({ answer: 42 }), 'Output mismatch');

  // Verify kind
  assert(span.kind?.type === 'custom', `Expected custom kind, got ${span.kind?.type}`);

  // Verify stats
  const stats = await client.getStats();
  assert(stats.span_count >= 1, `Expected >=1 spans, got ${stats.span_count}`);

  // Cleanup
  await client.deleteTrace(traceId);
  console.log('PASS: testCreateAndCompleteSpan');
}

async function testFailSpan() {
  const traceId = await client.trace('fail-test', async (t) => {
    try {
      await t.span('will-fail', async () => {
        throw new Error('something broke');
      }, {
        kind: { type: 'custom', kind: 'task', attributes: {} },
      });
    } catch {
      // expected
    }
    return t.traceId;
  });

  const spanList = await client.getTrace(traceId);
  assert(spanList.count === 1, `Expected 1 span, got ${spanList.count}`);
  const span = spanList.spans[0];
  assert(statusKind(span.status) === 'failed', `Expected failed, got ${JSON.stringify(span.status)}`);

  const err = statusError(span.status);
  assert(err !== null && err.includes('something broke'), `Expected error containing 'something broke', got '${err}'`);

  await client.deleteTrace(traceId);
  console.log('PASS: testFailSpan');
}

async function main() {
  await testCreateAndCompleteSpan();
  await testFailSpan();
  console.log('\nAll simple tests passed!');
}

main().catch(e => {
  console.error(e);
  process.exit(1);
});
