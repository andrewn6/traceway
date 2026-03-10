import { randomBytes, scryptSync, timingSafeEqual, createHash, randomUUID } from "node:crypto";
import { and, eq, gt, isNull } from "drizzle-orm";

import { db } from "../core/database";
import { sendInviteEmail, sendPasswordResetEmail } from "../email/resend";
import {
  apiKeys,
  authSessions,
  invites,
  organizations,
  passwordResets,
  projects,
  users,
} from "../core/schema";

const SESSION_COOKIE = "session";
const SESSION_MAX_AGE_SECONDS = 60 * 60 * 24 * 30;

export type AuthUser = {
  id: string;
  org_id: string;
  project_id: string;
  email: string;
  name?: string;
  role: string;
};

export type OrgInfo = {
  id: string;
  name: string;
  slug: string;
  plan: string;
  plan_limits: {
    spans_per_month: number;
    max_team_members: number;
    retention_days: number;
  };
};

export type OrgMember = {
  id: string;
  email: string;
  name?: string;
  role: string;
};

export type ApiKeyInfo = {
  id: string;
  name: string;
  key_prefix: string;
  scopes: string[];
  created_at: string;
  last_used_at?: string | null;
};

export type ApiKeyCreated = ApiKeyInfo & { key: string };

export type InviteInfo = {
  id: string;
  email: string;
  role: string;
  invited_by: string;
  expires_at: string;
  created_at: string;
};

function slugify(value: string): string {
  const base = value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return `${base || "org"}-${randomBytes(3).toString("hex")}`;
}

function hashPassword(password: string): string {
  const salt = randomBytes(16).toString("hex");
  const derived = scryptSync(password, salt, 64).toString("hex");
  return `${salt}:${derived}`;
}

function verifyPassword(password: string, storedHash: string): boolean {
  const [salt, expected] = storedHash.split(":");
  if (!salt || !expected) return false;
  const candidate = scryptSync(password, salt, 64).toString("hex");
  const a = Buffer.from(expected, "hex");
  const b = Buffer.from(candidate, "hex");
  if (a.length !== b.length) return false;
  return timingSafeEqual(a, b);
}

function newSessionToken(): string {
  return randomBytes(32).toString("base64url");
}

function hashToken(token: string): string {
  return createHash("sha256").update(token).digest("hex");
}

function makeApiKeyToken(): string {
  return `tw_${randomBytes(24).toString("base64url")}`;
}

function planLimits(plan: string): OrgInfo["plan_limits"] {
  if (plan === "pro") {
    return { spans_per_month: 1000000, max_team_members: 50, retention_days: 90 };
  }
  return { spans_per_month: 100000, max_team_members: 5, retention_days: 14 };
}

export function parseCookie(rawCookieHeader: string | undefined, name: string): string | undefined {
  if (!rawCookieHeader) return undefined;
  for (const part of rawCookieHeader.split(";")) {
    const [k, ...rest] = part.trim().split("=");
    if (k === name) return rest.join("=");
  }
  return undefined;
}

export function sessionCookie(token: string): string {
  const crossOrigin = Boolean(process.env.ALLOWED_ORIGINS);
  if (crossOrigin) {
    return `${SESSION_COOKIE}=${token}; HttpOnly; SameSite=None; Secure; Path=/; Max-Age=${SESSION_MAX_AGE_SECONDS}`;
  }
  return `${SESSION_COOKIE}=${token}; HttpOnly; SameSite=Lax; Path=/; Max-Age=${SESSION_MAX_AGE_SECONDS}`;
}

export function clearSessionCookie(): string {
  const crossOrigin = Boolean(process.env.ALLOWED_ORIGINS);
  if (crossOrigin) {
    return `${SESSION_COOKIE}=; HttpOnly; SameSite=None; Secure; Path=/; Max-Age=0`;
  }
  return `${SESSION_COOKIE}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0`;
}

export async function signup(input: {
  email: string;
  password: string;
  name?: string;
  org_name?: string;
}): Promise<{ user: AuthUser; token: string }> {
  const email = input.email.trim().toLowerCase();
  const existing = await db
    .select({ id: users.id })
    .from(users)
    .where(eq(users.email, email))
    .limit(1);
  if (existing.length > 0) {
    throw new Error("Email already in use");
  }

  const now = new Date();
  const orgName = input.org_name?.trim() || `${email.split("@")[0] || "User"}'s Org`;

  const [org] = await db
    .insert(organizations)
    .values({
      id: randomUUID(),
      name: orgName,
      slug: slugify(orgName),
      plan: "free",
      createdAt: now,
      updatedAt: now,
    })
    .returning();

  const [project] = await db
    .insert(projects)
    .values({
      id: randomUUID(),
      orgId: org.id,
      name: "Default",
      slug: "default",
      isDefault: true,
      createdAt: now,
      updatedAt: now,
    })
    .returning();

  const [user] = await db
    .insert(users)
    .values({
      id: randomUUID(),
      orgId: org.id,
      email,
      name: input.name?.trim() || null,
      role: "owner",
      passwordHash: hashPassword(input.password),
      createdAt: now,
      updatedAt: now,
    })
    .returning();

  const token = newSessionToken();
  await db.insert(authSessions).values({
    id: randomUUID(),
    userId: user.id,
    orgId: org.id,
    projectId: project.id,
    tokenHash: hashToken(token),
    expiresAt: new Date(Date.now() + SESSION_MAX_AGE_SECONDS * 1000),
    createdAt: now,
  });

  return {
    token,
    user: {
      id: user.id,
      org_id: org.id,
      project_id: project.id,
      email: user.email,
      name: user.name ?? undefined,
      role: user.role,
    },
  };
}

export async function login(input: {
  email: string;
  password: string;
}): Promise<{ user: AuthUser; token: string }> {
  const email = input.email.trim().toLowerCase();
  const [user] = await db
    .select()
    .from(users)
    .where(eq(users.email, email))
    .limit(1);
  if (!user || !verifyPassword(input.password, user.passwordHash)) {
    throw new Error("Invalid email or password");
  }

  const [project] = await db
    .select()
    .from(projects)
    .where(and(eq(projects.orgId, user.orgId), eq(projects.isDefault, true)))
    .limit(1);
  if (!project) {
    throw new Error("Default project not found");
  }

  const token = newSessionToken();
  await db.insert(authSessions).values({
    id: randomUUID(),
    userId: user.id,
    orgId: user.orgId,
    projectId: project.id,
    tokenHash: hashToken(token),
    expiresAt: new Date(Date.now() + SESSION_MAX_AGE_SECONDS * 1000),
    createdAt: new Date(),
  });

  return {
    token,
    user: {
      id: user.id,
      org_id: user.orgId,
      project_id: project.id,
      email: user.email,
      name: user.name ?? undefined,
      role: user.role,
    },
  };
}

export async function meFromSessionToken(token: string | undefined) {
  if (!token) return null;
  const tokenHash = hashToken(token);
  const [session] = await db
    .select()
    .from(authSessions)
    .where(and(eq(authSessions.tokenHash, tokenHash), gt(authSessions.expiresAt, new Date())))
    .limit(1);
  if (!session) return null;

  const [user] = await db
    .select()
    .from(users)
    .where(eq(users.id, session.userId))
    .limit(1);
  if (!user) return null;

  await db
    .update(authSessions)
    .set({ lastUsedAt: new Date() })
    .where(eq(authSessions.id, session.id));

  return {
    org_id: session.orgId,
    project_id: session.projectId,
    user_id: session.userId,
    scopes: ["all"],
    is_local_mode: false,
    email: user.email,
    name: user.name ?? undefined,
    role: user.role,
  };
}

export async function orgFromSession(token: string | undefined): Promise<OrgInfo | null> {
  const me = await meFromSessionToken(token);
  if (!me) return null;
  const [org] = await db.select().from(organizations).where(eq(organizations.id, me.org_id)).limit(1);
  if (!org) return null;
  return {
    id: org.id,
    name: org.name,
    slug: org.slug,
    plan: org.plan,
    plan_limits: planLimits(org.plan),
  };
}

export async function listMembers(token: string | undefined): Promise<OrgMember[] | null> {
  const me = await meFromSessionToken(token);
  if (!me) return null;
  const rows = await db.select().from(users).where(eq(users.orgId, me.org_id));
  return rows.map((u) => ({ id: u.id, email: u.email, name: u.name ?? undefined, role: u.role }));
}

export async function listProjects(token: string | undefined) {
  const me = await meFromSessionToken(token);
  if (!me) return null;
  const rows = await db
    .select()
    .from(projects)
    .where(eq(projects.orgId, me.org_id));
  return rows.map((p) => ({
    id: p.id,
    org_id: p.orgId,
    name: p.name,
    slug: p.slug,
    created_at: p.createdAt.toISOString(),
    updated_at: p.updatedAt.toISOString(),
  }));
}

export async function createProject(token: string | undefined, name: string) {
  const me = await meFromSessionToken(token);
  if (!me) return null;
  const now = new Date();
  const slug = slugify(name);
  const [project] = await db
    .insert(projects)
    .values({
      id: randomUUID(),
      orgId: me.org_id,
      name,
      slug,
      isDefault: false,
      createdAt: now,
      updatedAt: now,
    })
    .returning();
  return {
    id: project.id,
    org_id: project.orgId,
    name: project.name,
    slug: project.slug,
    created_at: project.createdAt.toISOString(),
    updated_at: project.updatedAt.toISOString(),
  };
}

export async function deleteProject(token: string | undefined, id: string): Promise<boolean | null> {
  const me = await meFromSessionToken(token);
  if (!me) return null;
  const [target] = await db
    .select()
    .from(projects)
    .where(and(eq(projects.id, id), eq(projects.orgId, me.org_id)))
    .limit(1);
  if (!target || target.isDefault) return false;

  const deleted = await db
    .delete(projects)
    .where(and(eq(projects.id, id), eq(projects.orgId, me.org_id)))
    .returning({ id: projects.id });
  return deleted.length > 0;
}

export async function switchProject(token: string | undefined, projectId: string): Promise<string | null> {
  const me = await meFromSessionToken(token);
  if (!me) return null;
  const [project] = await db
    .select()
    .from(projects)
    .where(and(eq(projects.id, projectId), eq(projects.orgId, me.org_id)))
    .limit(1);
  if (!project) return null;

  const newToken = newSessionToken();
  await db.insert(authSessions).values({
    id: randomUUID(),
    userId: me.user_id!,
    orgId: me.org_id,
    projectId,
    tokenHash: hashToken(newToken),
    expiresAt: new Date(Date.now() + SESSION_MAX_AGE_SECONDS * 1000),
    createdAt: new Date(),
  });
  await logoutSession(token);
  return newToken;
}

export async function listApiKeys(token: string | undefined): Promise<ApiKeyInfo[] | null> {
  const me = await meFromSessionToken(token);
  if (!me) return null;
  const rows = await db
    .select()
    .from(apiKeys)
    .where(eq(apiKeys.orgId, me.org_id));
  return rows.map((k) => ({
    id: k.id,
    name: k.name,
    key_prefix: k.keyPrefix,
    scopes: (k.scopes as string[]) ?? ["all"],
    created_at: k.createdAt.toISOString(),
    last_used_at: k.lastUsedAt?.toISOString() ?? null,
  }));
}

export async function createApiKey(token: string | undefined, name: string, scopes?: string[]): Promise<ApiKeyCreated | null> {
  const me = await meFromSessionToken(token);
  if (!me) return null;
  const raw = makeApiKeyToken();
  const now = new Date();
  const [created] = await db
    .insert(apiKeys)
    .values({
      id: randomUUID(),
      orgId: me.org_id,
      name,
      keyPrefix: raw.slice(0, 10),
      keyHash: hashToken(raw),
      scopes: scopes ?? ["all"],
      createdAt: now,
      lastUsedAt: null,
    })
    .returning();

  return {
    id: created.id,
    key: raw,
    name: created.name,
    key_prefix: created.keyPrefix,
    scopes: (created.scopes as string[]) ?? ["all"],
    created_at: created.createdAt.toISOString(),
    last_used_at: created.lastUsedAt?.toISOString() ?? null,
  };
}

export async function deleteApiKey(token: string | undefined, id: string): Promise<boolean | null> {
  const me = await meFromSessionToken(token);
  if (!me) return null;
  const deleted = await db
    .delete(apiKeys)
    .where(and(eq(apiKeys.id, id), eq(apiKeys.orgId, me.org_id)))
    .returning({ id: apiKeys.id });
  return deleted.length > 0;
}

export async function scopeFromApiKey(rawToken: string | undefined): Promise<{ org_id: string; project_id: string } | null> {
  const token = rawToken?.trim();
  if (!token) return null;

  const [apiKey] = await db
    .select({ id: apiKeys.id, orgId: apiKeys.orgId })
    .from(apiKeys)
    .where(eq(apiKeys.keyHash, hashToken(token)))
    .limit(1);

  if (!apiKey) return null;

  await db
    .update(apiKeys)
    .set({ lastUsedAt: new Date() })
    .where(eq(apiKeys.id, apiKey.id));

  const [defaultProject] = await db
    .select({ id: projects.id })
    .from(projects)
    .where(and(eq(projects.orgId, apiKey.orgId), eq(projects.isDefault, true)))
    .limit(1);

  if (defaultProject) {
    return { org_id: apiKey.orgId, project_id: defaultProject.id };
  }

  const [anyProject] = await db
    .select({ id: projects.id })
    .from(projects)
    .where(eq(projects.orgId, apiKey.orgId))
    .limit(1);

  if (!anyProject) return null;
  return { org_id: apiKey.orgId, project_id: anyProject.id };
}

export async function defaultScopeForLocal(): Promise<{ org_id: string; project_id: string } | null> {
  const envOrgId = process.env.TRACEWAY_ORG_ID?.trim();
  const envProjectId = process.env.TRACEWAY_PROJECT_ID?.trim();

  if (envOrgId && envProjectId) {
    return { org_id: envOrgId, project_id: envProjectId };
  }

  if (envOrgId) {
    const [projectInOrg] = await db
      .select({ id: projects.id })
      .from(projects)
      .where(eq(projects.orgId, envOrgId))
      .limit(1);
    if (projectInOrg) return { org_id: envOrgId, project_id: projectInOrg.id };
  }

  const [defaultProject] = await db
    .select({ id: projects.id, orgId: projects.orgId })
    .from(projects)
    .where(eq(projects.isDefault, true))
    .limit(1);

  if (defaultProject) {
    return { org_id: defaultProject.orgId, project_id: defaultProject.id };
  }

  const [firstProject] = await db
    .select({ id: projects.id, orgId: projects.orgId })
    .from(projects)
    .limit(1);

  if (!firstProject) return null;
  return { org_id: firstProject.orgId, project_id: firstProject.id };
}

export async function createInvite(token: string | undefined, email: string, role?: string): Promise<InviteInfo | null> {
  const me = await meFromSessionToken(token);
  if (!me || !me.user_id) return null;
  const inviteToken = randomBytes(32).toString("base64url");
  const [inviter] = await db.select({ name: users.name }).from(users).where(eq(users.id, me.user_id)).limit(1);
  await sendInviteEmail(email.toLowerCase().trim(), inviteToken, inviter?.name ?? undefined);
  const now = new Date();
  const expires = new Date(now.getTime() + 7 * 24 * 60 * 60 * 1000);
  const [inv] = await db
    .insert(invites)
    .values({
      id: randomUUID(),
      orgId: me.org_id,
      email: email.toLowerCase().trim(),
      role: role ?? "member",
      invitedBy: me.user_id,
      tokenHash: hashToken(inviteToken),
      expiresAt: expires,
      createdAt: now,
    })
    .returning();

  return {
    id: inv.id,
    email: inv.email,
    role: inv.role,
    invited_by: inv.invitedBy,
    expires_at: inv.expiresAt.toISOString(),
    created_at: inv.createdAt.toISOString(),
  };
}

export async function listInvites(token: string | undefined): Promise<InviteInfo[] | null> {
  const me = await meFromSessionToken(token);
  if (!me) return null;
  const rows = await db
    .select()
    .from(invites)
    .where(and(eq(invites.orgId, me.org_id), gt(invites.expiresAt, new Date())));
  return rows.map((inv) => ({
    id: inv.id,
    email: inv.email,
    role: inv.role,
    invited_by: inv.invitedBy,
    expires_at: inv.expiresAt.toISOString(),
    created_at: inv.createdAt.toISOString(),
  }));
}

export async function deleteInvite(token: string | undefined, id: string): Promise<boolean | null> {
  const me = await meFromSessionToken(token);
  if (!me) return null;
  const deleted = await db
    .delete(invites)
    .where(and(eq(invites.id, id), eq(invites.orgId, me.org_id)))
    .returning({ id: invites.id });
  return deleted.length > 0;
}

export async function acceptInviteToken(input: {
  token: string;
  password: string;
  name?: string;
}): Promise<{ user: AuthUser; token: string } | null> {
  const tokenHash = hashToken(input.token);
  const [inv] = await db
    .select()
    .from(invites)
    .where(and(eq(invites.tokenHash, tokenHash), gt(invites.expiresAt, new Date())))
    .limit(1);
  if (!inv) return null;

  const email = inv.email.toLowerCase().trim();
  let [user] = await db.select().from(users).where(eq(users.email, email)).limit(1);
  if (!user) {
    [user] = await db
      .insert(users)
      .values({
        id: randomUUID(),
        orgId: inv.orgId,
        email,
        name: input.name?.trim() || null,
        role: inv.role,
        passwordHash: hashPassword(input.password),
        createdAt: new Date(),
        updatedAt: new Date(),
      })
      .returning();
  } else {
    [user] = await db
      .update(users)
      .set({
        passwordHash: hashPassword(input.password),
        name: input.name?.trim() || user.name,
        updatedAt: new Date(),
      })
      .where(eq(users.id, user.id))
      .returning();
  }

  const [project] = await db
    .select()
    .from(projects)
    .where(and(eq(projects.orgId, inv.orgId), eq(projects.isDefault, true)))
    .limit(1);
  if (!project) return null;

  const token = newSessionToken();
  await db.insert(authSessions).values({
    id: randomUUID(),
    userId: user.id,
    orgId: inv.orgId,
    projectId: project.id,
    tokenHash: hashToken(token),
    expiresAt: new Date(Date.now() + SESSION_MAX_AGE_SECONDS * 1000),
    createdAt: new Date(),
  });

  await db.delete(invites).where(eq(invites.id, inv.id));

  return {
    token,
    user: {
      id: user.id,
      org_id: user.orgId,
      project_id: project.id,
      email: user.email,
      name: user.name ?? undefined,
      role: user.role,
    },
  };
}

export async function issuePasswordReset(email: string): Promise<{ ok: boolean; token?: string }> {
  const [user] = await db.select().from(users).where(eq(users.email, email.toLowerCase().trim())).limit(1);
  if (!user) return { ok: true };

  const token = randomBytes(32).toString("base64url");
  try {
    await sendPasswordResetEmail(user.email, token);
  } catch (err) {
    console.error("[auth] failed to send password reset email", err);
  }
  await db.insert(passwordResets).values({
    id: randomUUID(),
    userId: user.id,
    tokenHash: hashToken(token),
    expiresAt: new Date(Date.now() + 60 * 60 * 1000),
    createdAt: new Date(),
    usedAt: null,
  });
  return { ok: true, token };
}

export async function resetPasswordByToken(token: string, password: string): Promise<boolean> {
  const tokenHash = hashToken(token);
  const [reset] = await db
    .select()
    .from(passwordResets)
    .where(and(eq(passwordResets.tokenHash, tokenHash), gt(passwordResets.expiresAt, new Date()), isNull(passwordResets.usedAt)))
    .limit(1);
  if (!reset) return false;

  await db.update(users).set({ passwordHash: hashPassword(password), updatedAt: new Date() }).where(eq(users.id, reset.userId));
  await db.update(passwordResets).set({ usedAt: new Date() }).where(eq(passwordResets.id, reset.id));
  return true;
}

export async function logoutSession(token: string | undefined): Promise<void> {
  if (!token) return;
  await db
    .delete(authSessions)
    .where(eq(authSessions.tokenHash, hashToken(token)));
}
