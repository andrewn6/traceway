import { api } from "encore.dev/api";
import { and, eq } from "drizzle-orm";

import { db } from "../core/database";
import { providerConnections } from "../core/schema";
import { ProviderConnectionsService } from "./service";
import { handlePreflight, json, readJsonBody, requireSession, setCors } from "../shared/http";
import { pathSegments } from "../shared/request";
import { apiKeyPreview } from "../shared/redact";

export const listProviderConnectionsPublic = api.raw(
  { expose: true, method: "GET", path: "/provider-connections" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);

    const connections = await ProviderConnectionsService.list(session.org_id, session.project_id);
    json(res, 200, {
      connections: connections.map((c) => ({
        id: c.id,
        name: c.name,
        provider: c.provider,
        base_url: c.base_url,
        api_key_preview: apiKeyPreview(c.api_key),
        default_model: c.default_model,
        created_at: c.created_at,
        updated_at: c.updated_at,
      })),
      count: connections.length,
    });
  }
);

export const createProviderConnectionPublic = api.raw(
  { expose: true, method: "POST", path: "/provider-connections" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const body = await readJsonBody<{
      name: string;
      provider: string;
      base_url?: string;
      api_key?: string;
      default_model?: string;
    }>(req);
    const now = new Date().toISOString();
    const id = crypto.randomUUID();
    await ProviderConnectionsService.upsert(session.org_id, session.project_id, {
      id,
      name: body.name,
      provider: body.provider,
      base_url: body.base_url,
      api_key: body.api_key,
      default_model: body.default_model,
      created_at: now,
      updated_at: now,
    });
    const conn = await ProviderConnectionsService.get(session.org_id, session.project_id, id);
    json(res, 200, {
      id,
      name: conn?.name ?? body.name,
      provider: conn?.provider ?? body.provider,
      base_url: conn?.base_url,
      api_key_preview: apiKeyPreview(conn?.api_key),
      default_model: conn?.default_model,
      created_at: conn?.created_at ?? now,
      updated_at: conn?.updated_at ?? now,
    });
  }
);

export const updateProviderConnectionPublic = api.raw(
  { expose: true, method: "PUT", path: "/provider-connections/:conn_id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const connId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{
      name?: string;
      provider?: string;
      base_url?: string;
      api_key?: string;
      default_model?: string;
    }>(req);
    const current = await db
      .select()
      .from(providerConnections)
      .where(
        and(
          eq(providerConnections.id, connId),
          eq(providerConnections.orgId, session.org_id),
          eq(providerConnections.projectId, session.project_id)
        )
      )
      .limit(1);
    if (current.length === 0) {
      json(res, 404, { error: "Connection not found" });
      return;
    }

    const existing = current[0];
    const now = new Date().toISOString();
    await ProviderConnectionsService.upsert(session.org_id, session.project_id, {
      id: connId,
      name: body.name ?? existing.name,
      provider: body.provider ?? existing.provider,
      base_url: body.base_url ?? existing.baseUrl ?? undefined,
      api_key: body.api_key ?? existing.apiKey ?? undefined,
      default_model: body.default_model ?? existing.defaultModel ?? undefined,
      created_at: existing.createdAt.toISOString(),
      updated_at: now,
    });
    const conn = await ProviderConnectionsService.get(session.org_id, session.project_id, connId);
    json(res, 200, {
      id: conn?.id ?? connId,
      name: conn?.name ?? existing.name,
      provider: conn?.provider ?? existing.provider,
      base_url: conn?.base_url,
      api_key_preview: apiKeyPreview(conn?.api_key),
      default_model: conn?.default_model,
      created_at: conn?.created_at ?? existing.createdAt.toISOString(),
      updated_at: conn?.updated_at ?? now,
    });
  }
);

export const deleteProviderConnectionPublic = api.raw(
  { expose: true, method: "DELETE", path: "/provider-connections/:conn_id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const connId = pathSegments(req)[1] ?? "";
    await ProviderConnectionsService.delete(session.org_id, session.project_id, connId);
    json(res, 200, undefined);
  }
);

export const testProviderConnectionPublic = api.raw(
  { expose: true, method: "POST", path: "/provider-connections/test" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    json(res, 200, { ok: true, models: [] });
  }
);

export const listProviderModelsPublic = api.raw(
  { expose: true, method: "GET", path: "/provider-connections/:conn_id/models" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    json(res, 200, { ok: true, models: [] });
  }
);
