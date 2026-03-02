export { Traceway, TraceContext, SpanContext } from './client.js';
export type { TracewayOpts } from './client.js';
export type {
  Span,
  SpanKind,
  SpanMetadata,
  SpanStatus,
  Trace,
  TraceList,
  SpanList,
  Stats,
  ExportData,
  SpanFilter,
  StartSpanOpts,
  CreatedSpan,
  SpanEvent,
  TrackedFile,
  FileVersion,
  FileTraces,
  HealthStatus,
  Dataset,
  DatasetList,
  Datapoint,
  DatapointKind,
  DatapointList,
  DatapointSource,
  Message,
  QueueItem,
  QueueList,
} from './types.js';
export { statusKind, statusError } from './types.js';

// Vercel AI SDK integration (requires @opentelemetry/* peer deps)
export { initTraceway, TracewayExporter } from './ai/index.js';
export type { InitTracewayConfig, InitTracewayResult } from './ai/index.js';
export type { TracewayExporterConfig } from './ai/exporter.js';
