import { randomBytes, scryptSync, timingSafeEqual, createHash, randomUUID } from "node:crypto";
import { and, eq, gt } from "drizzle-orm";

import { db } from "../core/database";
import { authSessions, organizations, projects, users } from "../core/schema";

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

export async function logoutSession(token: string | undefined): Promise<void> {
  if (!token) return;
  await db
    .delete(authSessions)
    .where(eq(authSessions.tokenHash, hashToken(token)));
}
