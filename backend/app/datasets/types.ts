import { ScopeQuery } from "../core/types";
import { JsonValue } from "../core/json";

export interface Dataset {
  id: string;
  org_id: string;
  project_id: string;
  name: string;
  description?: string;
  created_at: string;
  updated_at: string;
}

export interface Datapoint {
  id: string;
  dataset_id: string;
  kind: JsonValue;
  source: string;
  source_span_id?: string;
  created_at: string;
}

export type CreateDatasetRequest = ScopeQuery & {
  id?: string;
  name: string;
  description?: string;
};

export type UpdateDatasetRequest = ScopeQuery & {
  id: string;
  name?: string;
  description?: string;
};

export type CreateDatapointRequest = ScopeQuery & {
  id?: string;
  dataset_id: string;
  kind: JsonValue;
  source: string;
  source_span_id?: string;
};

export type ListDatapointsRequest = ScopeQuery & {
  id: string;
};
