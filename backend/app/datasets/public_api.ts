import { api } from "encore.dev/api";
import { and, eq } from "drizzle-orm";

import { db } from "../core/database";
import { JsonValue } from "../core/json";
import { datapoints } from "../core/schema";
import { DatasetsService } from "../datasets/service";
import { QueueService } from "../queue/service";
import { handlePreflight, json, page, readJsonBody, requireScope, setCors } from "../shared/http";
import { pathSegments } from "../shared/request";

export const listDatasetsPublic = api.raw({ expose: true, method: "GET", path: "/datasets" }, async (req, res) => {
  if (handlePreflight(req, res)) return;
  const scope = await requireScope(req, res);
  if (!scope) return;
  setCors(req, res);

  const items = await DatasetsService.list(scope.org_id, scope.project_id);
  const withCount = await Promise.all(
    items.map(async (dataset) => {
      const points = await DatasetsService.listDatapoints(scope.org_id, scope.project_id, dataset.id);
      return { ...dataset, datapoint_count: points.length };
    })
  );
  json(res, 200, { datasets: withCount, count: withCount.length });
});

export const createDatasetPublic = api.raw(
  { expose: true, method: "POST", path: "/datasets" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const scope = await requireScope(req, res);
    if (!scope) return;
    setCors(req, res);
    const body = await readJsonBody<{ name: string; description?: string }>(req);
    const dataset = await DatasetsService.create({
      org_id: scope.org_id,
      project_id: scope.project_id,
      name: body.name,
      description: body.description,
    });
    json(res, 200, dataset);
  }
);

export const getDatasetPublic = api.raw(
  { expose: true, method: "GET", path: "/datasets/:id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const scope = await requireScope(req, res);
    if (!scope) return;
    setCors(req, res);

    const datasetId = pathSegments(req)[1] ?? "";
    const dataset = await DatasetsService.get(scope.org_id, scope.project_id, datasetId);
    if (!dataset) {
      json(res, 404, { error: "Dataset not found" });
      return;
    }
    const points = await DatasetsService.listDatapoints(scope.org_id, scope.project_id, datasetId);
    json(res, 200, { ...dataset, datapoint_count: points.length });
  }
);

export const updateDatasetPublic = api.raw(
  { expose: true, method: "PUT", path: "/datasets/:id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const scope = await requireScope(req, res);
    if (!scope) return;
    setCors(req, res);

    const datasetId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ name?: string; description?: string }>(req);
    const updated = await DatasetsService.update({
      org_id: scope.org_id,
      project_id: scope.project_id,
      id: datasetId,
      name: body.name,
      description: body.description,
    });
    if (!updated) {
      json(res, 404, { error: "Dataset not found" });
      return;
    }
    const points = await DatasetsService.listDatapoints(scope.org_id, scope.project_id, datasetId);
    json(res, 200, { ...updated, datapoint_count: points.length });
  }
);

export const deleteDatasetPublic = api.raw(
  { expose: true, method: "DELETE", path: "/datasets/:id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const scope = await requireScope(req, res);
    if (!scope) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    await DatasetsService.delete(scope.org_id, scope.project_id, datasetId);
    json(res, 200, { ok: true });
  }
);

export const listDatapointsPublic = api.raw(
  { expose: true, method: "GET", path: "/datasets/:id/datapoints" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const scope = await requireScope(req, res);
    if (!scope) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const items = await DatasetsService.listDatapoints(scope.org_id, scope.project_id, datasetId);
    json(res, 200, page(items));
  }
);

export const getDatapointPublic = api.raw(
  { expose: true, method: "GET", path: "/datasets/:id/datapoints/:dp_id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const scope = await requireScope(req, res);
    if (!scope) return;
    setCors(req, res);
    const parts = pathSegments(req);
    const datasetId = parts[1] ?? "";
    const datapointId = parts[3] ?? "";
    const items = await DatasetsService.listDatapoints(scope.org_id, scope.project_id, datasetId);
    const found = items.find((d) => d.id === datapointId);
    if (!found) {
      json(res, 404, { error: "Datapoint not found" });
      return;
    }
    json(res, 200, found);
  }
);

export const createDatapointPublic = api.raw(
  { expose: true, method: "POST", path: "/datasets/:id/datapoints" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const scope = await requireScope(req, res);
    if (!scope) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ kind: unknown }>(req);
    const item = await DatasetsService.createDatapoint({
      org_id: scope.org_id,
      project_id: scope.project_id,
      dataset_id: datasetId,
      kind: (body.kind ?? null) as JsonValue,
      source: "manual",
    });
    json(res, 200, item);
  }
);

export const deleteDatapointPublic = api.raw(
  { expose: true, method: "DELETE", path: "/datasets/:id/datapoints/:dp_id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const scope = await requireScope(req, res);
    if (!scope) return;
    setCors(req, res);
    const parts = pathSegments(req);
    const datasetId = parts[1] ?? "";
    const datapointId = parts[3] ?? "";
    await db
      .delete(datapoints)
      .where(
        and(
          eq(datapoints.id, datapointId),
          eq(datapoints.datasetId, datasetId),
          eq(datapoints.orgId, scope.org_id),
          eq(datapoints.projectId, scope.project_id)
        )
      );
    json(res, 200, { ok: true });
  }
);

export const exportSpanToDatasetPublic = api.raw(
  { expose: true, method: "POST", path: "/datasets/:id/export-span" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const scope = await requireScope(req, res);
    if (!scope) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ span_id: string }>(req);
    const dp = await DatasetsService.createDatapoint({
      org_id: scope.org_id,
      project_id: scope.project_id,
      dataset_id: datasetId,
      source: "span_export",
      source_span_id: body.span_id,
      kind: { type: "generic", input: null, expected_output: null, actual_output: null } as JsonValue,
    });
    json(res, 200, dp);
  }
);

export const listQueueByDatasetPublic = api.raw(
  { expose: true, method: "GET", path: "/datasets/:id/queue" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const scope = await requireScope(req, res);
    if (!scope) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const items = await QueueService.list(scope.org_id, scope.project_id, datasetId);
    json(res, 200, page(items));
  }
);

export const enqueueDatapointsPublic = api.raw(
  { expose: true, method: "POST", path: "/datasets/:id/queue" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const scope = await requireScope(req, res);
    if (!scope) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ datapoint_ids: string[] }>(req);
    const items = await QueueService.enqueue(scope.org_id, scope.project_id, datasetId, body.datapoint_ids ?? []);
    json(res, 200, page(items));
  }
);

export const exportAndEnqueuePublic = api.raw(
  { expose: true, method: "POST", path: "/datasets/:id/export-span-and-enqueue" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const scope = await requireScope(req, res);
    if (!scope) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ span_id: string }>(req);
    const dp = await DatasetsService.createDatapoint({
      org_id: scope.org_id,
      project_id: scope.project_id,
      dataset_id: datasetId,
      source: "span_export",
      source_span_id: body.span_id,
      kind: { type: "generic", input: null, expected_output: null, actual_output: null } as JsonValue,
    });
    const [item] = await QueueService.enqueue(scope.org_id, scope.project_id, datasetId, [dp.id]);
    json(res, 200, item ?? null);
  }
);
