/**
 * Traceway integration for the Vercel AI SDK.
 *
 * Usage:
 *
 *   import { initTraceway } from 'traceway/ai';
 *   import { generateText } from 'ai';
 *   import { openai } from '@ai-sdk/openai';
 *
 *   const { tracer } = initTraceway({ apiKey: 'tw_sk_...' });
 *
 *   const result = await generateText({
 *     model: openai('gpt-4o'),
 *     prompt: 'Hello!',
 *     experimental_telemetry: { isEnabled: true, tracer },
 *   });
 */

import {
  BasicTracerProvider,
  BatchSpanProcessor,
  type TracerConfig,
} from '@opentelemetry/sdk-trace-base';
import { trace, type Tracer } from '@opentelemetry/api';

import { TracewayExporter, type TracewayExporterConfig } from './exporter.js';

// ── Config ──────────────────────────────────────────────────────────

export interface InitTracewayConfig extends TracewayExporterConfig {
  /**
   * Name for the tracer provider (shows in OTel diagnostics).
   * Defaults to 'traceway'.
   */
  serviceName?: string;

  /**
   * Max export batch size for the BatchSpanProcessor.
   * Defaults to 64.
   */
  maxExportBatchSize?: number;

  /**
   * Max delay in ms before the BatchSpanProcessor flushes.
   * Defaults to 1000 (1 second).
   */
  scheduledDelayMillis?: number;
}

export interface InitTracewayResult {
  /**
   * The OTel Tracer to pass to the AI SDK's `experimental_telemetry.tracer`.
   */
  tracer: Tracer;

  /**
   * The TracerProvider. Call `provider.shutdown()` on process exit
   * to ensure all pending spans are flushed.
   */
  provider: BasicTracerProvider;

  /**
   * Convenience: force-flush all pending spans.
   */
  flush: () => Promise<void>;

  /**
   * Convenience: shut down the provider (flushes + cleans up).
   */
  shutdown: () => Promise<void>;
}

// ── Main entry point ────────────────────────────────────────────────

/**
 * Initialize Traceway telemetry for the Vercel AI SDK.
 *
 * Returns a `tracer` that you pass directly to AI SDK calls:
 *
 * ```ts
 * const { tracer } = initTraceway();
 *
 * await generateText({
 *   model: openai('gpt-4o'),
 *   prompt: 'Hello!',
 *   experimental_telemetry: { isEnabled: true, tracer },
 * });
 * ```
 */
export function initTraceway(config?: InitTracewayConfig): InitTracewayResult {
  const exporter = new TracewayExporter(config);

  const processor = new BatchSpanProcessor(exporter, {
    maxExportBatchSize: config?.maxExportBatchSize ?? 64,
    scheduledDelayMillis: config?.scheduledDelayMillis ?? 1000,
  });

  const providerConfig: TracerConfig = {
    // Resource attributes could be added here if needed
  };

  const provider = new BasicTracerProvider(providerConfig);
  provider.addSpanProcessor(processor);

  // Register as the global provider so the AI SDK picks it up
  // if no explicit tracer is passed.
  provider.register();

  const serviceName = config?.serviceName ?? 'traceway';
  const tracer = trace.getTracer(serviceName);

  return {
    tracer,
    provider,
    flush: () => provider.forceFlush(),
    shutdown: () => provider.shutdown(),
  };
}

// ── Re-exports ──────────────────────────────────────────────────────

export { TracewayExporter, type TracewayExporterConfig } from './exporter.js';
export {
  getOperationType,
  getSpanName,
  toTracewayKind,
  getSpanInput,
  getSpanOutput,
  getMetadata,
  isRootAiSpan,
  isLlmCallSpan,
  isToolCallSpan,
  type AiOperationType,
} from './mapper.js';
