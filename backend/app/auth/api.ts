import { api } from "encore.dev/api";

import { handlePreflight, json, readJsonBody, setCors } from "../shared/http";
import {
  acceptInviteToken,
  clearSessionCookie,
  createApiKey,
  createInvite,
  createProject,
  deleteApiKey,
  deleteInvite,
  deleteProject,
  issuePasswordReset,
  listApiKeys,
  listInvites,
  listMembers,
  listProjects,
  login,
  logoutSession,
  meFromSessionToken,
  orgFromSession,
  parseCookie,
  resetPasswordByToken,
  sessionCookie,
  signup,
  switchProject,
} from "./service";

function currentSessionToken(req: import("http").IncomingMessage): string | undefined {
  return parseCookie(req.headers.cookie, "session");
}

function pathSegments(req: import("http").IncomingMessage): string[] {
  return new URL(req.url ?? "/", "http://local").pathname.split("/").filter(Boolean);
}

export const authConfig = api.raw(
  { expose: true, method: "GET", path: "/auth/config" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    json(res, 200, { mode: "cloud", features: ["auth", "teams", "api_keys"] });
  }
);

export const authMe = api.raw(
  { expose: true, method: "GET", path: "/auth/me" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    const me = await meFromSessionToken(currentSessionToken(req));
    if (!me) {
      json(res, 401, { error: "Unauthorized" });
      return;
    }
    json(res, 200, me);
  }
);

export const authSignup = api.raw(
  { expose: true, method: "POST", path: "/auth/signup" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    try {
      const body = await readJsonBody<{ email?: string; password?: string; name?: string; org_name?: string }>(req);
      if (!body.email || !body.password) {
        json(res, 400, { error: "email and password are required" });
        return;
      }
      const { user, token } = await signup(body as { email: string; password: string; name?: string; org_name?: string });
      res.setHeader("set-cookie", sessionCookie(token));
      json(res, 201, {
        user_id: user.id,
        org_id: user.org_id,
        email: user.email,
        name: user.name,
        role: user.role,
      });
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Signup failed";
      json(res, msg.includes("already") ? 409 : 500, { error: msg });
    }
  }
);

export const authLogin = api.raw(
  { expose: true, method: "POST", path: "/auth/login" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    try {
      const body = await readJsonBody<{ email?: string; password?: string }>(req);
      if (!body.email || !body.password) {
        json(res, 400, { error: "email and password are required" });
        return;
      }
      const { user, token } = await login({ email: body.email, password: body.password });
      res.setHeader("set-cookie", sessionCookie(token));
      json(res, 200, {
        user_id: user.id,
        org_id: user.org_id,
        email: user.email,
        name: user.name,
        role: user.role,
      });
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Login failed";
      json(res, msg.includes("Invalid") ? 401 : 500, { error: msg });
    }
  }
);

export const authLogout = api.raw(
  { expose: true, method: "POST", path: "/auth/logout" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    await logoutSession(currentSessionToken(req));
    res.setHeader("set-cookie", clearSessionCookie());
    json(res, 200, { ok: true });
  }
);

export const getOrg = api.raw(
  { expose: true, method: "GET", path: "/org" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    const org = await orgFromSession(currentSessionToken(req));
    if (!org) {
      json(res, 401, { error: "Unauthorized" });
      return;
    }
    json(res, 200, org);
  }
);

export const getMembers = api.raw(
  { expose: true, method: "GET", path: "/org/members" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    const members = await listMembers(currentSessionToken(req));
    if (!members) {
      json(res, 401, { error: "Unauthorized" });
      return;
    }
    json(res, 200, members);
  }
);

export const getApiKeysEndpoint = api.raw(
  { expose: true, method: "GET", path: "/org/api-keys" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    const keys = await listApiKeys(currentSessionToken(req));
    if (!keys) {
      json(res, 401, { error: "Unauthorized" });
      return;
    }
    json(res, 200, keys);
  }
);

export const createApiKeyEndpoint = api.raw(
  { expose: true, method: "POST", path: "/org/api-keys" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    const body = await readJsonBody<{ name?: string; scopes?: string[] }>(req);
    if (!body.name?.trim()) {
      json(res, 400, { error: "name is required" });
      return;
    }
    const created = await createApiKey(currentSessionToken(req), body.name.trim(), body.scopes);
    if (!created) {
      json(res, 401, { error: "Unauthorized" });
      return;
    }
    json(res, 200, created);
  }
);

export const deleteApiKeyEndpoint = api.raw(
  { expose: true, method: "DELETE", path: "/org/api-keys/:id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    const id = pathSegments(req)[2] ?? "";
    const ok = await deleteApiKey(currentSessionToken(req), id);
    if (ok === null) {
      json(res, 401, { error: "Unauthorized" });
      return;
    }
    json(res, 200, { ok });
  }
);

export const getInvitesEndpoint = api.raw(
  { expose: true, method: "GET", path: "/org/invites" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    const invites = await listInvites(currentSessionToken(req));
    if (!invites) {
      json(res, 401, { error: "Unauthorized" });
      return;
    }
    json(res, 200, invites);
  }
);

export const createInviteEndpoint = api.raw(
  { expose: true, method: "POST", path: "/org/invites" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    const body = await readJsonBody<{ email?: string; role?: string }>(req);
    if (!body.email?.trim()) {
      json(res, 400, { error: "email is required" });
      return;
    }
    let invite;
    try {
      invite = await createInvite(currentSessionToken(req), body.email, body.role);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to send invite";
      json(res, 502, { error: message });
      return;
    }
    if (!invite) {
      json(res, 401, { error: "Unauthorized" });
      return;
    }
    json(res, 200, invite);
  }
);

export const deleteInviteEndpoint = api.raw(
  { expose: true, method: "DELETE", path: "/org/invites/:id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    const id = pathSegments(req)[2] ?? "";
    const ok = await deleteInvite(currentSessionToken(req), id);
    if (ok === null) {
      json(res, 401, { error: "Unauthorized" });
      return;
    }
    json(res, 200, { ok });
  }
);

export const listProjectsEndpoint = api.raw(
  { expose: true, method: "GET", path: "/projects" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    const projects = await listProjects(currentSessionToken(req));
    if (!projects) {
      json(res, 401, { error: "Unauthorized" });
      return;
    }
    json(res, 200, projects);
  }
);

export const createProjectEndpoint = api.raw(
  { expose: true, method: "POST", path: "/projects" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    const body = await readJsonBody<{ name?: string }>(req);
    if (!body.name?.trim()) {
      json(res, 400, { error: "name is required" });
      return;
    }
    const project = await createProject(currentSessionToken(req), body.name.trim());
    if (!project) {
      json(res, 401, { error: "Unauthorized" });
      return;
    }
    json(res, 200, project);
  }
);

export const deleteProjectEndpoint = api.raw(
  { expose: true, method: "DELETE", path: "/projects/:id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    const id = pathSegments(req)[1] ?? "";
    const ok = await deleteProject(currentSessionToken(req), id);
    if (ok === null) {
      json(res, 401, { error: "Unauthorized" });
      return;
    }
    json(res, 200, { ok });
  }
);

export const switchProjectEndpoint = api.raw(
  { expose: true, method: "POST", path: "/projects/switch" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    const body = await readJsonBody<{ project_id?: string }>(req);
    if (!body.project_id) {
      json(res, 400, { error: "project_id is required" });
      return;
    }

    const next = await switchProject(currentSessionToken(req), body.project_id);
    if (!next) {
      json(res, 401, { error: "Unauthorized" });
      return;
    }

    res.setHeader("set-cookie", sessionCookie(next));
    json(res, 200, { ok: true });
  }
);

export const acceptInviteEndpoint = api.raw(
  { expose: true, method: "POST", path: "/auth/accept-invite" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    const body = await readJsonBody<{ token?: string; password?: string; name?: string }>(req);
    if (!body.token || !body.password) {
      json(res, 400, { error: "token and password are required" });
      return;
    }
    const accepted = await acceptInviteToken({ token: body.token, password: body.password, name: body.name });
    if (!accepted) {
      json(res, 400, { error: "Invalid or expired invite" });
      return;
    }

    res.setHeader("set-cookie", sessionCookie(accepted.token));
    json(res, 200, {
      user_id: accepted.user.id,
      org_id: accepted.user.org_id,
      email: accepted.user.email,
      name: accepted.user.name,
      role: accepted.user.role,
    });
  }
);

export const forgotPasswordEndpoint = api.raw(
  { expose: true, method: "POST", path: "/auth/forgot-password" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    const body = await readJsonBody<{ email?: string }>(req);
    if (!body.email) {
      json(res, 400, { ok: false, message: "email is required" });
      return;
    }
    await issuePasswordReset(body.email);
    json(res, 200, { ok: true, message: "If account exists, reset was issued" });
  }
);

export const resetPasswordEndpoint = api.raw(
  { expose: true, method: "POST", path: "/auth/reset-password" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);
    const body = await readJsonBody<{ token?: string; password?: string }>(req);
    if (!body.token || !body.password) {
      json(res, 400, { ok: false, message: "token and password are required" });
      return;
    }
    const ok = await resetPasswordByToken(body.token, body.password);
    if (!ok) {
      json(res, 400, { ok: false, message: "Invalid or expired token" });
      return;
    }
    json(res, 200, { ok: true, message: "Password reset" });
  }
);
