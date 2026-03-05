import { ScopeQuery } from "../core/types";
import { JsonValue } from "../core/json";

export interface EvalRun {
  id: string;
  dataset_id: string;
  name?: string;
  config: JsonValue;
  scoring: string;
  status: string;
  results: JsonValue;
  trace_id?: string;
  created_at: string;
  completed_at?: string;
  error?: string;
}

export interface EvalResult {
  id: string;
  run_id: string;
  datapoint_id: string;
  status: string;
  actual_output: JsonValue;
  score?: number;
  score_reason?: string;
  latency_ms: number;
  input_tokens?: number;
  output_tokens?: number;
  error?: string;
  span_id?: string;
  created_at: string;
}

export type CreateEvalRunRequest = ScopeQuery & {
  id?: string;
  dataset_id: string;
  name?: string;
  config: JsonValue;
  scoring?: string;
};

export type UpdateEvalRunRequest = ScopeQuery & {
  id: string;
  status?: string;
  results?: JsonValue;
  trace_id?: string;
  completed_at?: string;
  error?: string;
};

export type CreateEvalResultRequest = ScopeQuery & {
  id?: string;
  run_id: string;
  datapoint_id: string;
  status?: string;
  actual_output?: JsonValue;
  score?: number;
  score_reason?: string;
  latency_ms?: number;
  input_tokens?: number;
  output_tokens?: number;
  error?: string;
  span_id?: string;
};
