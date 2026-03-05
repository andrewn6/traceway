import { api } from "encore.dev/api";
import { and, asc, eq } from "drizzle-orm";

import { db } from "../core/database";
import { JsonValue } from "../core/json";
import { fileVersions } from "../core/schema";
import { ScopeQuery } from "../core/types";
import { newId, validateScope } from "../core/utils";

export const listFileVersions = api(
  { expose: true, auth: true, method: "GET", path: "/internal/files" },
  async (req: ScopeQuery & { path?: string }) => {
    validateScope(req);
    const rows = await db
      .select()
      .from(fileVersions)
      .where(
        req.path
          ? and(
              eq(fileVersions.orgId, req.org_id),
              eq(fileVersions.projectId, req.project_id),
              eq(fileVersions.path, req.path)
            )
          : and(eq(fileVersions.orgId, req.org_id), eq(fileVersions.projectId, req.project_id))
      )
      .orderBy(asc(fileVersions.createdAt));

    return {
      items: rows.map((r) => ({
        id: r.id,
        path: r.path,
        hash: r.hash,
        metadata: r.metadata,
        created_by_span: r.createdBySpan,
        created_at: r.createdAt.toISOString(),
      })),
      count: rows.length,
    };
  }
);

export const createFileVersion = api(
  { expose: true, auth: true, method: "POST", path: "/internal/files" },
  async (
    req: ScopeQuery & {
      id?: string;
      path: string;
      hash: string;
      metadata?: JsonValue;
      created_by_span?: string;
    }
  ) => {
    validateScope(req);
    const [row] = await db
      .insert(fileVersions)
      .values({
        id: req.id ?? newId(),
        orgId: req.org_id,
        projectId: req.project_id,
        path: req.path,
        hash: req.hash,
        metadata: req.metadata ?? {},
        createdBySpan: req.created_by_span ?? null,
        createdAt: new Date(),
      })
      .onConflictDoNothing()
      .returning();

    if (!row) {
      return { ok: true, duplicate: true };
    }

    return {
      ok: true,
      duplicate: false,
      file: {
        id: row.id,
        path: row.path,
        hash: row.hash,
        metadata: row.metadata,
        created_by_span: row.createdBySpan,
        created_at: row.createdAt.toISOString(),
      },
    };
  }
);
