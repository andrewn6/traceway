import { IncomingMessage, ServerResponse } from "node:http";

import { defaultScopeForLocal, meFromSessionToken, parseCookie, scopeFromApiKey } from "../auth/service";

type Session = NonNullable<Awaited<ReturnType<typeof meFromSessionToken>>>;
export type RequestScope = {
  org_id: string;
  project_id: string;
  user_id?: string;
  principal: "session" | "daemon" | "api_key";
};

function bearerToken(req: IncomingMessage): string | undefined {
  const auth = req.headers.authorization;
  const raw = typeof auth === "string" ? auth : Array.isArray(auth) ? auth[0] : undefined;
  if (!raw) return undefined;
  const m = raw.match(/^Bearer\s+(.+)$/i);
  return m?.[1]?.trim();
}

function allowOrigin(origin: string | undefined): string | null {
  if (!origin) return null;
  const configured = process.env.ALLOWED_ORIGINS;
  if (!configured || configured.trim() === "") {
    return origin;
  }
  const allowed = configured
    .split(",")
    .map((v) => v.trim())
    .filter(Boolean);
  return allowed.includes(origin) ? origin : null;
}

export function setCors(req: IncomingMessage, res: ServerResponse): void {
  if (!req.headers.origin) {
    return;
  }
  // Encore Cloud may already set CORS headers — avoid duplicating them.
  if (res.getHeader("access-control-allow-origin")) {
    return;
  }
  const origin = allowOrigin(req.headers.origin);
  if (origin) {
    res.setHeader("access-control-allow-origin", origin);
    res.setHeader("vary", "Origin");
  }
  res.setHeader("access-control-allow-credentials", "true");
  res.setHeader("access-control-allow-headers", "content-type");
  res.setHeader("access-control-allow-methods", "GET,POST,PUT,PATCH,DELETE,OPTIONS");
}

export function handlePreflight(req: IncomingMessage, res: ServerResponse): boolean {
  setCors(req, res);
  if (req.method === "OPTIONS") {
    res.statusCode = 204;
    res.end();
    return true;
  }
  return false;
}

export function json(res: ServerResponse, status: number, payload: unknown): void {
  res.statusCode = status;
  res.setHeader("content-type", "application/json; charset=utf-8");
  res.end(JSON.stringify(payload));
}

export async function readJsonBody<T>(req: IncomingMessage): Promise<T> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  }
  if (chunks.length === 0) {
    return {} as T;
  }
  return JSON.parse(Buffer.concat(chunks).toString("utf8")) as T;
}

export function query(req: IncomingMessage): URLSearchParams {
  const url = new URL(req.url ?? "/", "http://local");
  return url.searchParams;
}

export async function requireSession(req: IncomingMessage, res: ServerResponse): Promise<Session | null> {
  const token = parseCookie(req.headers.cookie, "session");
  const session = await meFromSessionToken(token);
  if (!session) {
    json(res, 401, { error: "Unauthorized" });
    return null;
  }
  return session;
}

export async function requireScope(req: IncomingMessage, res: ServerResponse): Promise<RequestScope | null> {
  const bearer = bearerToken(req);
  const apiKeyScope = await scopeFromApiKey(bearer);
  if (apiKeyScope) {
    return {
      org_id: apiKeyScope.org_id,
      project_id: apiKeyScope.project_id,
      principal: "api_key",
    };
  }

  const bootstrapApiKey = process.env.TRACEWAY_API_KEY?.trim();
  if (bootstrapApiKey && bearer && bearer === bootstrapApiKey) {
    const scope = await defaultScopeForLocal();
    if (!scope) {
      json(res, 401, { error: "No project scope available for TRACEWAY_API_KEY" });
      return null;
    }
    return {
      org_id: scope.org_id,
      project_id: scope.project_id,
      principal: "api_key",
    };
  }

  const expected = process.env.TRACEWAY_BACKEND_TOKEN ?? process.env.TRACEWAY_CONTROL_PLANE_TOKEN ?? "";
  const provided = req.headers["x-traceway-control-token"];
  const token = typeof provided === "string" ? provided.trim() : Array.isArray(provided) ? (provided[0] ?? "").trim() : "";

  if (expected && token && token === expected) {
    const orgHeader = req.headers["x-traceway-org-id"];
    const projectHeader = req.headers["x-traceway-project-id"];
    const orgId = typeof orgHeader === "string" ? orgHeader : Array.isArray(orgHeader) ? orgHeader[0] : undefined;
    const projectId = typeof projectHeader === "string" ? projectHeader : Array.isArray(projectHeader) ? projectHeader[0] : undefined;

    if (!orgId || !projectId) {
      json(res, 401, { error: "Daemon auth missing x-traceway-org-id/x-traceway-project-id headers" });
      return null;
    }

    return {
      org_id: orgId,
      project_id: projectId,
      principal: "daemon",
    };
  }

  const session = await requireSession(req, res);
  if (!session) return null;
  return {
    org_id: session.org_id,
    project_id: session.project_id,
    user_id: session.user_id,
    principal: "session",
  };
}

type PageOptions = {
  cursor?: string | null;
  limit?: number | null;
  maxLimit?: number;
};

export function page<T>(
  items: T[],
  options?: PageOptions,
): { items: T[]; total: number; next_cursor: string | null; has_more: boolean } {
  const hasPaginationInput = options?.cursor != null || options?.limit != null;
  if (!hasPaginationInput) {
    return { items, total: items.length, next_cursor: null, has_more: false };
  }

  const maxLimit = options?.maxLimit ?? 200;
  const parsedLimit = Number(options?.limit ?? 50);
  const limit = Number.isFinite(parsedLimit)
    ? Math.max(1, Math.min(maxLimit, Math.floor(parsedLimit)))
    : 50;

  const parsedOffset = Number(options?.cursor ?? 0);
  const offset = Number.isFinite(parsedOffset) ? Math.max(0, Math.floor(parsedOffset)) : 0;

  const pagedItems = items.slice(offset, offset + limit);
  const nextOffset = offset + pagedItems.length;
  const hasMore = nextOffset < items.length;

  return {
    items: pagedItems,
    total: items.length,
    next_cursor: hasMore ? String(nextOffset) : null,
    has_more: hasMore,
  };
}
