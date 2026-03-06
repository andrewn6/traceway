import { api } from "encore.dev/api";

import {
  clearSessionCookie,
  login,
  logoutSession,
  meFromSessionToken,
  parseCookie,
  sessionCookie,
  signup,
} from "./service";

async function readJsonBody(req: import("http").IncomingMessage): Promise<unknown> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  }
  if (chunks.length === 0) return {};
  return JSON.parse(Buffer.concat(chunks).toString("utf8"));
}

function badRequest(res: import("http").ServerResponse, message: string) {
  res.statusCode = 400;
  res.setHeader("content-type", "application/json; charset=utf-8");
  res.end(JSON.stringify({ error: message }));
}

export const authConfig = api.raw(
  { expose: true, method: "GET", path: "/auth/config" },
  async (_req, res) => {
    res.setHeader("content-type", "application/json; charset=utf-8");
    res.end(
      JSON.stringify({
        mode: "cloud",
        features: ["auth", "teams", "api_keys"],
      })
    );
  }
);

export const authMe = api.raw(
  { expose: true, method: "GET", path: "/auth/me" },
  async (req, res) => {
    const token = parseCookie(req.headers.cookie, "session");
    const me = await meFromSessionToken(token);

    if (!me) {
      res.statusCode = 401;
      res.setHeader("content-type", "application/json; charset=utf-8");
      res.end(JSON.stringify({ error: "Unauthorized" }));
      return;
    }

    res.setHeader("content-type", "application/json; charset=utf-8");
    res.end(JSON.stringify(me));
  }
);

export const authSignup = api.raw(
  { expose: true, method: "POST", path: "/auth/signup" },
  async (req, res) => {
    try {
      const body = (await readJsonBody(req)) as {
        email?: string;
        password?: string;
        name?: string;
        org_name?: string;
      };

      if (!body.email || !body.password) {
        return badRequest(res, "email and password are required");
      }

      const { user, token } = await signup({
        email: body.email,
        password: body.password,
        name: body.name,
        org_name: body.org_name,
      });

      res.statusCode = 201;
      res.setHeader("set-cookie", sessionCookie(token));
      res.setHeader("content-type", "application/json; charset=utf-8");
      res.end(
        JSON.stringify({
          user_id: user.id,
          org_id: user.org_id,
          email: user.email,
          name: user.name,
          role: user.role,
        })
      );
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Signup failed";
      const status = msg.includes("already") ? 409 : 500;
      res.statusCode = status;
      res.setHeader("content-type", "application/json; charset=utf-8");
      res.end(JSON.stringify({ error: msg }));
    }
  }
);

export const authLogin = api.raw(
  { expose: true, method: "POST", path: "/auth/login" },
  async (req, res) => {
    try {
      const body = (await readJsonBody(req)) as {
        email?: string;
        password?: string;
      };

      if (!body.email || !body.password) {
        return badRequest(res, "email and password are required");
      }

      const { user, token } = await login({ email: body.email, password: body.password });

      res.statusCode = 200;
      res.setHeader("set-cookie", sessionCookie(token));
      res.setHeader("content-type", "application/json; charset=utf-8");
      res.end(
        JSON.stringify({
          user_id: user.id,
          org_id: user.org_id,
          email: user.email,
          name: user.name,
          role: user.role,
        })
      );
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Login failed";
      const status = msg.includes("Invalid") ? 401 : 500;
      res.statusCode = status;
      res.setHeader("content-type", "application/json; charset=utf-8");
      res.end(JSON.stringify({ error: msg }));
    }
  }
);

export const authLogout = api.raw(
  { expose: true, method: "POST", path: "/auth/logout" },
  async (req, res) => {
    const token = parseCookie(req.headers.cookie, "session");
    await logoutSession(token);
    res.statusCode = 200;
    res.setHeader("set-cookie", clearSessionCookie());
    res.setHeader("content-type", "application/json; charset=utf-8");
    res.end(JSON.stringify({ ok: true }));
  }
);
