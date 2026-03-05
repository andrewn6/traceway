import { ScopeQuery } from "../core/types";
import { JsonValue } from "../core/json";

export interface CaptureRule {
  id: string;
  dataset_id: string;
  name: string;
  enabled: boolean;
  filters: JsonValue;
  sample_rate: number;
  captured_count: number;
  created_at: string;
}

export type CreateCaptureRuleRequest = ScopeQuery & {
  id?: string;
  dataset_id: string;
  name: string;
  filters?: JsonValue;
  sample_rate?: number;
};

export type UpdateCaptureRuleRequest = ScopeQuery & {
  id: string;
  name?: string;
  filters?: JsonValue;
  sample_rate?: number;
};
