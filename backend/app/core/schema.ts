import * as p from "drizzle-orm/pg-core";

export const organizations = p.pgTable(
  "organizations",
  {
    id: p.uuid().primaryKey(),
    name: p.text().notNull(),
    slug: p.text().notNull().unique(),
    plan: p.text().notNull().default("free"),
    createdAt: p.timestamp("created_at", { withTimezone: true }).notNull(),
    updatedAt: p.timestamp("updated_at", { withTimezone: true }).notNull(),
  },
  (table) => [p.index("organizations_slug_idx").on(table.slug)]
);

export const projects = p.pgTable(
  "projects",
  {
    id: p.uuid().primaryKey(),
    orgId: p
      .uuid("org_id")
      .notNull()
      .references(() => organizations.id, { onDelete: "cascade" }),
    name: p.text().notNull(),
    slug: p.text().notNull(),
    isDefault: p.boolean("is_default").notNull().default(false),
    createdAt: p.timestamp("created_at", { withTimezone: true }).notNull(),
    updatedAt: p.timestamp("updated_at", { withTimezone: true }).notNull(),
  },
  (table) => [
    p.index("projects_org_idx").on(table.orgId),
    p.unique("projects_org_slug_unique").on(table.orgId, table.slug),
  ]
);

export const users = p.pgTable(
  "users",
  {
    id: p.uuid().primaryKey(),
    orgId: p
      .uuid("org_id")
      .notNull()
      .references(() => organizations.id, { onDelete: "cascade" }),
    email: p.text().notNull().unique(),
    name: p.text(),
    role: p.text().notNull().default("member"),
    passwordHash: p.text("password_hash").notNull(),
    createdAt: p.timestamp("created_at", { withTimezone: true }).notNull(),
    updatedAt: p.timestamp("updated_at", { withTimezone: true }).notNull(),
  },
  (table) => [
    p.index("users_org_idx").on(table.orgId),
    p.index("users_email_idx").on(table.email),
  ]
);

export const authSessions = p.pgTable(
  "auth_sessions",
  {
    id: p.uuid().primaryKey(),
    userId: p
      .uuid("user_id")
      .notNull()
      .references(() => users.id, { onDelete: "cascade" }),
    orgId: p
      .uuid("org_id")
      .notNull()
      .references(() => organizations.id, { onDelete: "cascade" }),
    projectId: p
      .uuid("project_id")
      .notNull()
      .references(() => projects.id, { onDelete: "cascade" }),
    tokenHash: p.text("token_hash").notNull().unique(),
    expiresAt: p.timestamp("expires_at", { withTimezone: true }).notNull(),
    createdAt: p.timestamp("created_at", { withTimezone: true }).notNull(),
    lastUsedAt: p.timestamp("last_used_at", { withTimezone: true }),
  },
  (table) => [
    p.index("auth_sessions_user_idx").on(table.userId),
    p.index("auth_sessions_token_hash_idx").on(table.tokenHash),
    p.index("auth_sessions_expires_idx").on(table.expiresAt),
  ]
);

export const providerConnections = p.pgTable(
  "provider_connections",
  {
    id: p.uuid().primaryKey(),
    orgId: p.uuid("org_id").notNull(),
    projectId: p.uuid("project_id").notNull(),
    name: p.text().notNull(),
    provider: p.text().notNull(),
    baseUrl: p.text("base_url"),
    apiKey: p.text("api_key"),
    defaultModel: p.text("default_model"),
    createdAt: p.timestamp("created_at", { withTimezone: true }).notNull(),
    updatedAt: p.timestamp("updated_at", { withTimezone: true }).notNull(),
  },
  (table) => [
    p.unique("provider_connections_org_project_name_unique").on(
      table.orgId,
      table.projectId,
      table.name
    ),
    p.index("provider_connections_org_project_idx").on(table.orgId, table.projectId),
  ]
);

export const datasets = p.pgTable(
  "datasets",
  {
    id: p.uuid().primaryKey(),
    orgId: p.uuid("org_id").notNull(),
    projectId: p.uuid("project_id").notNull(),
    name: p.text().notNull(),
    description: p.text(),
    createdAt: p.timestamp("created_at", { withTimezone: true }).notNull(),
    updatedAt: p.timestamp("updated_at", { withTimezone: true }).notNull(),
  },
  (table) => [
    p.unique("datasets_org_project_name_unique").on(table.orgId, table.projectId, table.name),
    p.index("datasets_org_project_idx").on(table.orgId, table.projectId),
  ]
);

export const datapoints = p.pgTable(
  "datapoints",
  {
    id: p.uuid().primaryKey(),
    orgId: p.uuid("org_id").notNull(),
    projectId: p.uuid("project_id").notNull(),
    datasetId: p
      .uuid("dataset_id")
      .notNull()
      .references(() => datasets.id, { onDelete: "cascade" }),
    kind: p.jsonb().notNull(),
    source: p.text().notNull(),
    sourceSpanId: p.uuid("source_span_id"),
    createdAt: p.timestamp("created_at", { withTimezone: true }).notNull(),
  },
  (table) => [
    p.index("datapoints_dataset_created_idx").on(table.datasetId, table.createdAt),
    p.index("datapoints_org_project_idx").on(table.orgId, table.projectId),
  ]
);

export const queueItems = p.pgTable(
  "queue_items",
  {
    id: p.uuid().primaryKey(),
    orgId: p.uuid("org_id").notNull(),
    projectId: p.uuid("project_id").notNull(),
    datasetId: p
      .uuid("dataset_id")
      .notNull()
      .references(() => datasets.id, { onDelete: "cascade" }),
    datapointId: p
      .uuid("datapoint_id")
      .notNull()
      .references(() => datapoints.id, { onDelete: "cascade" }),
    status: p.text().notNull(),
    claimedBy: p.text("claimed_by"),
    claimedAt: p.timestamp("claimed_at", { withTimezone: true }),
    originalData: p.jsonb("original_data"),
    editedData: p.jsonb("edited_data"),
    createdAt: p.timestamp("created_at", { withTimezone: true }).notNull(),
    updatedAt: p.timestamp("updated_at", { withTimezone: true }).notNull(),
  },
  (table) => [
    p.index("queue_items_dataset_status_idx").on(table.datasetId, table.status),
    p.index("queue_items_org_project_idx").on(table.orgId, table.projectId),
  ]
);

export const evalRuns = p.pgTable(
  "eval_runs",
  {
    id: p.uuid().primaryKey(),
    orgId: p.uuid("org_id").notNull(),
    projectId: p.uuid("project_id").notNull(),
    datasetId: p
      .uuid("dataset_id")
      .notNull()
      .references(() => datasets.id, { onDelete: "cascade" }),
    name: p.text(),
    config: p.jsonb().notNull(),
    scoring: p.text().notNull(),
    status: p.text().notNull(),
    results: p.jsonb().notNull(),
    traceId: p.uuid("trace_id"),
    createdAt: p.timestamp("created_at", { withTimezone: true }).notNull(),
    completedAt: p.timestamp("completed_at", { withTimezone: true }),
    error: p.text(),
  },
  (table) => [
    p.index("eval_runs_dataset_created_idx").on(table.datasetId, table.createdAt),
    p.index("eval_runs_org_project_idx").on(table.orgId, table.projectId),
  ]
);

export const evalResults = p.pgTable(
  "eval_results",
  {
    id: p.uuid().primaryKey(),
    orgId: p.uuid("org_id").notNull(),
    projectId: p.uuid("project_id").notNull(),
    runId: p
      .uuid("run_id")
      .notNull()
      .references(() => evalRuns.id, { onDelete: "cascade" }),
    datapointId: p
      .uuid("datapoint_id")
      .notNull()
      .references(() => datapoints.id, { onDelete: "cascade" }),
    status: p.text().notNull(),
    actualOutput: p.jsonb("actual_output").notNull(),
    score: p.doublePrecision(),
    scoreReason: p.text("score_reason"),
    latencyMs: p.bigint("latency_ms", { mode: "number" }).notNull(),
    inputTokens: p.integer("input_tokens"),
    outputTokens: p.integer("output_tokens"),
    error: p.text(),
    spanId: p.uuid("span_id"),
    createdAt: p.timestamp("created_at", { withTimezone: true }).notNull(),
  },
  (table) => [
    p.index("eval_results_run_datapoint_idx").on(table.runId, table.datapointId),
    p.index("eval_results_org_project_idx").on(table.orgId, table.projectId),
  ]
);

export const captureRules = p.pgTable(
  "capture_rules",
  {
    id: p.uuid().primaryKey(),
    orgId: p.uuid("org_id").notNull(),
    projectId: p.uuid("project_id").notNull(),
    datasetId: p
      .uuid("dataset_id")
      .notNull()
      .references(() => datasets.id, { onDelete: "cascade" }),
    name: p.text().notNull(),
    enabled: p.boolean().notNull(),
    filters: p.jsonb().notNull(),
    sampleRate: p.doublePrecision("sample_rate").notNull(),
    capturedCount: p.bigint("captured_count", { mode: "number" }).notNull(),
    createdAt: p.timestamp("created_at", { withTimezone: true }).notNull(),
  },
  (table) => [
    p.index("capture_rules_dataset_idx").on(table.datasetId),
    p.index("capture_rules_org_project_idx").on(table.orgId, table.projectId),
  ]
);

export const fileVersions = p.pgTable(
  "file_versions",
  {
    id: p.uuid().primaryKey(),
    orgId: p.uuid("org_id").notNull(),
    projectId: p.uuid("project_id").notNull(),
    path: p.text().notNull(),
    hash: p.text().notNull(),
    metadata: p.jsonb().notNull(),
    createdBySpan: p.uuid("created_by_span"),
    createdAt: p.timestamp("created_at", { withTimezone: true }).notNull(),
  },
  (table) => [
    p.unique("file_versions_org_project_path_hash_unique").on(
      table.orgId,
      table.projectId,
      table.path,
      table.hash
    ),
    p.index("file_versions_org_project_path_idx").on(table.orgId, table.projectId, table.path),
  ]
);
