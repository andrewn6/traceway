import { createHmac, timingSafeEqual } from "node:crypto";

import { api } from "encore.dev/api";
import { eq } from "drizzle-orm";

import { db } from "../core/database";
import { organizations, users } from "../core/schema";
import { handlePreflight, json, readJsonBody, requireSession, setCors } from "../core/public_api";

type PolarWebhookPayload = {
  type: string;
  data: {
    status?: string;
    cancel_at_period_end?: boolean;
    product?: {
      name?: string;
      metadata?: Record<string, unknown>;
    };
    customer?: {
      external_id?: string;
      metadata?: Record<string, unknown>;
    };
    metadata?: Record<string, unknown>;
  };
};

function polarProductId(plan: string): string | null {
  if (plan === "pro") {
    return process.env.POLAR_PRODUCT_PRO_ID ?? "02a58cb6-1853-4179-ba29-6d65c71836db";
  }
  if (plan === "team") {
    return process.env.POLAR_PRODUCT_TEAM_ID ?? "507c5aeb-b4a2-47e4-870b-ea6b89814a9f";
  }
  return null;
}

function planFromProduct(product: { name?: string; metadata?: Record<string, unknown> } | undefined): "free" | "pro" | "team" {
  const mapped = typeof product?.metadata?.traceway_plan === "string" ? product.metadata.traceway_plan : null;
  if (mapped === "pro" || mapped === "team") return mapped;

  const name = (product?.name ?? "").toLowerCase();
  if (name.includes("team") || name.includes("enterprise")) return "team";
  if (name.includes("pro")) return "pro";
  return "free";
}

function extractOrgId(payload: PolarWebhookPayload): string | null {
  const md = payload.data?.metadata;
  if (md && typeof md.org_id === "string") return md.org_id;

  const externalId = payload.data?.customer?.external_id;
  if (typeof externalId === "string" && externalId.length > 0) return externalId;

  const customerMd = payload.data?.customer?.metadata;
  if (customerMd && typeof customerMd.org_id === "string") return customerMd.org_id;

  return null;
}

function headerValue(headers: import("http").IncomingHttpHeaders, key: string): string | null {
  const value = headers[key.toLowerCase()];
  if (typeof value === "string") return value;
  if (Array.isArray(value) && value.length > 0) return value[0] ?? null;
  return null;
}

function verifyPolarWebhook(
  body: Buffer,
  headers: import("http").IncomingHttpHeaders,
  secret: string
): boolean {
  const msgId = headerValue(headers, "webhook-id");
  const ts = headerValue(headers, "webhook-timestamp");
  const sig = headerValue(headers, "webhook-signature");
  if (!msgId || !ts || !sig) return false;

  const timestamp = Number(ts);
  if (!Number.isFinite(timestamp)) return false;
  if (Math.abs(Math.floor(Date.now() / 1000) - timestamp) > 300) return false;

  const secretBase64 = secret.startsWith("whsec_") ? secret.slice("whsec_".length) : secret;
  let key: Buffer;
  try {
    key = Buffer.from(secretBase64, "base64");
  } catch {
    return false;
  }

  const signed = `${msgId}.${ts}.${body.toString("utf8")}`;
  const expected = createHmac("sha256", key).update(signed).digest("base64");

  return sig.split(" ").some((part) => {
    const value = part.startsWith("v1,") ? part.slice(3) : "";
    if (!value) return false;
    const a = Buffer.from(value);
    const b = Buffer.from(expected);
    return a.length === b.length && timingSafeEqual(a, b);
  });
}

async function updateOrgPlan(orgId: string, plan: "free" | "pro" | "team"): Promise<void> {
  await db
    .update(organizations)
    .set({
      plan,
      updatedAt: new Date(),
    })
    .where(eq(organizations.id, orgId));
}

export const createCheckout = api.raw(
  { expose: true, method: "POST", path: "/billing/checkout" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);

    const token = process.env.POLAR_ACCESS_TOKEN;
    if (!token) {
      json(res, 503, { error: "Billing not configured" });
      return;
    }

    const body = await readJsonBody<{ plan?: string }>(req);
    const plan = (body.plan ?? "").trim().toLowerCase();
    const productId = polarProductId(plan);
    if (!productId) {
      json(res, 400, { error: "Invalid plan. Use 'pro' or 'team'." });
      return;
    }

    const [row] = await db.select({ email: users.email }).from(users).where(eq(users.id, session.user_id ?? "")).limit(1);
    const members = await db.select({ id: users.id }).from(users).where(eq(users.orgId, session.org_id));

    const appUrl = process.env.APP_URL ?? "https://app.traceway.ai";
    const payload: Record<string, unknown> = {
      products: [productId],
      success_url: `${appUrl}/settings/billing?checkout=success`,
      seats: Math.max(1, members.length),
      metadata: { org_id: session.org_id },
    };
    if (row?.email) payload.customer_email = row.email;

    const resp = await fetch("https://api.polar.sh/v1/checkouts/", {
      method: "POST",
      headers: {
        authorization: `Bearer ${token}`,
        "content-type": "application/json",
      },
      body: JSON.stringify(payload),
    });

    if (!resp.ok) {
      const text = await resp.text();
      json(res, 502, { error: `Polar API error (${resp.status}): ${text}` });
      return;
    }

    const checkout = (await resp.json()) as { url?: string };
    if (!checkout.url) {
      json(res, 502, { error: "Polar response missing checkout URL" });
      return;
    }

    json(res, 200, { url: checkout.url });
  }
);

export const polarWebhook = api.raw(
  { expose: true, method: "POST", path: "/billing/polar/webhook" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);

    const chunks: Buffer[] = [];
    for await (const chunk of req) {
      chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
    }
    const body = Buffer.concat(chunks);
    const secret = process.env.POLAR_WEBHOOK_SECRET;
    if (secret && !verifyPolarWebhook(body, req.headers, secret)) {
      json(res, 403, { error: "Invalid webhook signature" });
      return;
    }

    let payload: PolarWebhookPayload;
    try {
      payload = JSON.parse(body.toString("utf8")) as PolarWebhookPayload;
    } catch {
      json(res, 400, { error: "Invalid payload" });
      return;
    }

    const orgId = extractOrgId(payload);
    if (!orgId) {
      json(res, 200, { ok: true, skipped: "no-org-id" });
      return;
    }

    const status = payload.data?.status ?? "";
    const eventType = payload.type;
    const shouldDowngrade = eventType.includes("revoked") || eventType.includes("canceled") || status === "canceled";
    const targetPlan = shouldDowngrade ? "free" : planFromProduct(payload.data?.product);

    const [org] = await db
      .select({ id: organizations.id })
      .from(organizations)
      .where(eq(organizations.id, orgId))
      .limit(1);

    if (!org) {
      json(res, 200, { ok: true, skipped: "org-not-found" });
      return;
    }

    await updateOrgPlan(org.id, targetPlan);
    json(res, 200, { ok: true });
  }
);
