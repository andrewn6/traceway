import { IncomingMessage, ServerResponse } from "node:http";

import { meFromSessionToken, parseCookie } from "../auth/service";

type Session = NonNullable<Awaited<ReturnType<typeof meFromSessionToken>>>;

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

export function page<T>(items: T[]): { items: T[]; total: number; next_cursor: null; has_more: false } {
  return { items, total: items.length, next_cursor: null, has_more: false };
}
