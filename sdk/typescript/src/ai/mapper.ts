/**
 * Maps OpenTelemetry span attributes from the Vercel AI SDK telemetry
 * format into Traceway span/trace structures.
 *
 * AI SDK span attribute reference:
 *   https://sdk.vercel.ai/docs/ai-sdk-core/telemetry#collected-data
 */

import type { ReadableSpan } from '@opentelemetry/sdk-trace-base';
import type { SpanKind } from '../types.js';

// ── Attribute keys emitted by the AI SDK ────────────────────────────

const AI = {
  operationId: 'ai.operationId',
  modelId: 'ai.model.id',
  modelProvider: 'ai.model.provider',
  promptTokens: 'ai.usage.promptTokens',
  completionTokens: 'ai.usage.completionTokens',
  prompt: 'ai.prompt',
  promptMessages: 'ai.prompt.messages',
  responseText: 'ai.response.text',
  responseToolCalls: 'ai.response.toolCalls',
  responseFinishReason: 'ai.response.finishReason',
  responseModel: 'ai.response.model',
  responseId: 'ai.response.id',
  responseMsToFirstChunk: 'ai.response.msToFirstChunk',
  responseMsToFinish: 'ai.response.msToFinish',
  responseAvgTokensPerSec: 'ai.response.avgCompletionTokensPerSecond',
  functionId: 'ai.telemetry.functionId',
  toolCallName: 'ai.toolCall.name',
  toolCallId: 'ai.toolCall.id',
  toolCallArgs: 'ai.toolCall.args',
  toolCallResult: 'ai.toolCall.result',
  settingsMaxOutputTokens: 'ai.settings.maxOutputTokens',
  // GenAI semantic conventions
  genAiSystem: 'gen_ai.system',
  genAiInputTokens: 'gen_ai.usage.input_tokens',
  genAiOutputTokens: 'gen_ai.usage.output_tokens',
} as const;

// ── Helpers ─────────────────────────────────────────────────────────

function attr(span: ReadableSpan, key: string): unknown {
  return span.attributes[key];
}

function attrStr(span: ReadableSpan, key: string): string | undefined {
  const v = attr(span, key);
  return typeof v === 'string' ? v : undefined;
}

function attrNum(span: ReadableSpan, key: string): number | undefined {
  const v = attr(span, key);
  if (typeof v === 'number') return v;
  if (typeof v === 'string') {
    const n = Number(v);
    return Number.isFinite(n) ? n : undefined;
  }
  return undefined;
}

// ── Operation detection ─────────────────────────────────────────────

export type AiOperationType =
  | 'generateText'
  | 'generateText.doGenerate'
  | 'streamText'
  | 'streamText.doStream'
  | 'toolCall'
  | 'embed'
  | 'embed.doEmbed'
  | 'embedMany'
  | 'embedMany.doEmbed'
  | 'unknown';

/**
 * Determine which AI SDK operation a span represents.
 */
export function getOperationType(span: ReadableSpan): AiOperationType {
  const op = attrStr(span, AI.operationId);
  switch (op) {
    case 'ai.generateText':           return 'generateText';
    case 'ai.generateText.doGenerate': return 'generateText.doGenerate';
    case 'ai.streamText':             return 'streamText';
    case 'ai.streamText.doStream':    return 'streamText.doStream';
    case 'ai.toolCall':               return 'toolCall';
    case 'ai.embed':                  return 'embed';
    case 'ai.embed.doEmbed':          return 'embed.doEmbed';
    case 'ai.embedMany':              return 'embedMany';
    case 'ai.embedMany.doEmbed':      return 'embedMany.doEmbed';
    default:                          return 'unknown';
  }
}

/**
 * Is this span a "root" AI operation (generateText / streamText)?
 * These map to Traceway traces.
 */
export function isRootAiSpan(span: ReadableSpan): boolean {
  const op = getOperationType(span);
  return op === 'generateText' || op === 'streamText';
}

/**
 * Is this an LLM call span (doGenerate / doStream)?
 * These map to Traceway LLM call spans.
 */
export function isLlmCallSpan(span: ReadableSpan): boolean {
  const op = getOperationType(span);
  return op === 'generateText.doGenerate' || op === 'streamText.doStream';
}

/**
 * Is this a tool call span?
 */
export function isToolCallSpan(span: ReadableSpan): boolean {
  return getOperationType(span) === 'toolCall';
}

// ── Mapping to Traceway types ───────────────────────────────────────

/**
 * Extract a human-readable name for the span.
 */
export function getSpanName(span: ReadableSpan): string {
  // functionId is the user-specified name override
  const functionId = attrStr(span, AI.functionId);
  if (functionId) return functionId;

  const op = getOperationType(span);
  switch (op) {
    case 'toolCall': {
      const toolName = attrStr(span, AI.toolCallName);
      return toolName ? `tool:${toolName}` : 'tool_call';
    }
    case 'generateText':
    case 'streamText':
      return span.name || op;
    case 'generateText.doGenerate':
    case 'streamText.doStream': {
      const model = attrStr(span, AI.modelId) || attrStr(span, AI.responseModel);
      return model ? `llm:${model}` : span.name || op;
    }
    default:
      return span.name || 'unknown';
  }
}

/**
 * Build a Traceway SpanKind from an OTel span's attributes.
 */
export function toTracewayKind(span: ReadableSpan): SpanKind | undefined {
  const op = getOperationType(span);

  if (op === 'generateText.doGenerate' || op === 'streamText.doStream') {
    const model = attrStr(span, AI.responseModel) || attrStr(span, AI.modelId) || 'unknown';
    const provider = attrStr(span, AI.modelProvider) || attrStr(span, AI.genAiSystem);
    const inputTokens = attrNum(span, AI.promptTokens) ?? attrNum(span, AI.genAiInputTokens);
    const outputTokens = attrNum(span, AI.completionTokens) ?? attrNum(span, AI.genAiOutputTokens);

    return {
      type: 'llm_call',
      model,
      provider,
      input_tokens: inputTokens,
      output_tokens: outputTokens,
    };
  }

  if (op === 'toolCall') {
    const toolName = attrStr(span, AI.toolCallName) || 'unknown_tool';
    return {
      type: 'custom',
      kind: 'tool_call',
      attributes: {
        tool_name: toolName,
        tool_call_id: attrStr(span, AI.toolCallId),
      },
    };
  }

  // Root spans (generateText / streamText) and embedding spans
  // don't get a specific kind — they're just container/grouping spans
  return undefined;
}

/**
 * Extract span input from OTel attributes.
 */
export function getSpanInput(span: ReadableSpan): unknown {
  const op = getOperationType(span);

  if (op === 'toolCall') {
    const args = attrStr(span, AI.toolCallArgs);
    if (args) {
      try { return JSON.parse(args); } catch { return args; }
    }
    return undefined;
  }

  // For doGenerate/doStream, use prompt.messages (array of messages)
  if (isLlmCallSpan(span)) {
    const messages = attrStr(span, AI.promptMessages);
    if (messages) {
      try { return JSON.parse(messages); } catch { return messages; }
    }
  }

  // For root spans, use the prompt
  const prompt = attrStr(span, AI.prompt);
  if (prompt) {
    try { return JSON.parse(prompt); } catch { return prompt; }
  }

  return undefined;
}

/**
 * Extract span output from OTel attributes.
 */
export function getSpanOutput(span: ReadableSpan): unknown {
  const op = getOperationType(span);

  if (op === 'toolCall') {
    const result = attrStr(span, AI.toolCallResult);
    if (result) {
      try { return JSON.parse(result); } catch { return result; }
    }
    return undefined;
  }

  const text = attrStr(span, AI.responseText);
  const toolCalls = attrStr(span, AI.responseToolCalls);
  const finishReason = attrStr(span, AI.responseFinishReason);

  if (!text && !toolCalls) return undefined;

  const output: Record<string, unknown> = {};
  if (text) output.text = text;
  if (toolCalls) {
    try { output.tool_calls = JSON.parse(toolCalls); } catch { output.tool_calls = toolCalls; }
  }
  if (finishReason) output.finish_reason = finishReason;

  // Include streaming-specific metrics if present
  const msToFirstChunk = attrNum(span, AI.responseMsToFirstChunk);
  const msToFinish = attrNum(span, AI.responseMsToFinish);
  const avgTokensPerSec = attrNum(span, AI.responseAvgTokensPerSec);
  if (msToFirstChunk !== undefined) output.ms_to_first_chunk = msToFirstChunk;
  if (msToFinish !== undefined) output.ms_to_finish = msToFinish;
  if (avgTokensPerSec !== undefined) output.avg_completion_tokens_per_sec = avgTokensPerSec;

  return output;
}

/**
 * Extract telemetry metadata from OTel span attributes.
 * Picks up ai.telemetry.metadata.* keys.
 */
export function getMetadata(span: ReadableSpan): Record<string, unknown> | undefined {
  const prefix = 'ai.telemetry.metadata.';
  const meta: Record<string, unknown> = {};
  let hasKeys = false;

  for (const [key, value] of Object.entries(span.attributes)) {
    if (key.startsWith(prefix)) {
      meta[key.slice(prefix.length)] = value;
      hasKeys = true;
    }
  }

  return hasKeys ? meta : undefined;
}
