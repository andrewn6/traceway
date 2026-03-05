CREATE TABLE IF NOT EXISTS "capture_rules" (
	"id" uuid PRIMARY KEY NOT NULL,
	"org_id" uuid NOT NULL,
	"project_id" uuid NOT NULL,
	"dataset_id" uuid NOT NULL,
	"name" text NOT NULL,
	"enabled" boolean NOT NULL,
	"filters" jsonb NOT NULL,
	"sample_rate" double precision NOT NULL,
	"captured_count" bigint NOT NULL,
	"created_at" timestamp with time zone NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "datapoints" (
	"id" uuid PRIMARY KEY NOT NULL,
	"org_id" uuid NOT NULL,
	"project_id" uuid NOT NULL,
	"dataset_id" uuid NOT NULL,
	"kind" jsonb NOT NULL,
	"source" text NOT NULL,
	"source_span_id" uuid,
	"created_at" timestamp with time zone NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "datasets" (
	"id" uuid PRIMARY KEY NOT NULL,
	"org_id" uuid NOT NULL,
	"project_id" uuid NOT NULL,
	"name" text NOT NULL,
	"description" text,
	"created_at" timestamp with time zone NOT NULL,
	"updated_at" timestamp with time zone NOT NULL,
	CONSTRAINT "datasets_org_project_name_unique" UNIQUE("org_id","project_id","name")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "eval_results" (
	"id" uuid PRIMARY KEY NOT NULL,
	"org_id" uuid NOT NULL,
	"project_id" uuid NOT NULL,
	"run_id" uuid NOT NULL,
	"datapoint_id" uuid NOT NULL,
	"status" text NOT NULL,
	"actual_output" jsonb NOT NULL,
	"score" double precision,
	"score_reason" text,
	"latency_ms" bigint NOT NULL,
	"input_tokens" integer,
	"output_tokens" integer,
	"error" text,
	"span_id" uuid,
	"created_at" timestamp with time zone NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "eval_runs" (
	"id" uuid PRIMARY KEY NOT NULL,
	"org_id" uuid NOT NULL,
	"project_id" uuid NOT NULL,
	"dataset_id" uuid NOT NULL,
	"name" text,
	"config" jsonb NOT NULL,
	"scoring" text NOT NULL,
	"status" text NOT NULL,
	"results" jsonb NOT NULL,
	"trace_id" uuid,
	"created_at" timestamp with time zone NOT NULL,
	"completed_at" timestamp with time zone,
	"error" text
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "file_versions" (
	"id" uuid PRIMARY KEY NOT NULL,
	"org_id" uuid NOT NULL,
	"project_id" uuid NOT NULL,
	"path" text NOT NULL,
	"hash" text NOT NULL,
	"metadata" jsonb NOT NULL,
	"created_by_span" uuid,
	"created_at" timestamp with time zone NOT NULL,
	CONSTRAINT "file_versions_org_project_path_hash_unique" UNIQUE("org_id","project_id","path","hash")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "provider_connections" (
	"id" uuid PRIMARY KEY NOT NULL,
	"org_id" uuid NOT NULL,
	"project_id" uuid NOT NULL,
	"name" text NOT NULL,
	"provider" text NOT NULL,
	"base_url" text,
	"api_key" text,
	"default_model" text,
	"created_at" timestamp with time zone NOT NULL,
	"updated_at" timestamp with time zone NOT NULL,
	CONSTRAINT "provider_connections_org_project_name_unique" UNIQUE("org_id","project_id","name")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "queue_items" (
	"id" uuid PRIMARY KEY NOT NULL,
	"org_id" uuid NOT NULL,
	"project_id" uuid NOT NULL,
	"dataset_id" uuid NOT NULL,
	"datapoint_id" uuid NOT NULL,
	"status" text NOT NULL,
	"claimed_by" text,
	"claimed_at" timestamp with time zone,
	"original_data" jsonb,
	"edited_data" jsonb,
	"created_at" timestamp with time zone NOT NULL,
	"updated_at" timestamp with time zone NOT NULL
);
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "capture_rules" ADD CONSTRAINT "capture_rules_dataset_id_datasets_id_fk" FOREIGN KEY ("dataset_id") REFERENCES "public"."datasets"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "datapoints" ADD CONSTRAINT "datapoints_dataset_id_datasets_id_fk" FOREIGN KEY ("dataset_id") REFERENCES "public"."datasets"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "eval_results" ADD CONSTRAINT "eval_results_run_id_eval_runs_id_fk" FOREIGN KEY ("run_id") REFERENCES "public"."eval_runs"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "eval_results" ADD CONSTRAINT "eval_results_datapoint_id_datapoints_id_fk" FOREIGN KEY ("datapoint_id") REFERENCES "public"."datapoints"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "eval_runs" ADD CONSTRAINT "eval_runs_dataset_id_datasets_id_fk" FOREIGN KEY ("dataset_id") REFERENCES "public"."datasets"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "queue_items" ADD CONSTRAINT "queue_items_dataset_id_datasets_id_fk" FOREIGN KEY ("dataset_id") REFERENCES "public"."datasets"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "queue_items" ADD CONSTRAINT "queue_items_datapoint_id_datapoints_id_fk" FOREIGN KEY ("datapoint_id") REFERENCES "public"."datapoints"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "capture_rules_dataset_idx" ON "capture_rules" USING btree ("dataset_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "capture_rules_org_project_idx" ON "capture_rules" USING btree ("org_id","project_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "datapoints_dataset_created_idx" ON "datapoints" USING btree ("dataset_id","created_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "datapoints_org_project_idx" ON "datapoints" USING btree ("org_id","project_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "datasets_org_project_idx" ON "datasets" USING btree ("org_id","project_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "eval_results_run_datapoint_idx" ON "eval_results" USING btree ("run_id","datapoint_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "eval_results_org_project_idx" ON "eval_results" USING btree ("org_id","project_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "eval_runs_dataset_created_idx" ON "eval_runs" USING btree ("dataset_id","created_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "eval_runs_org_project_idx" ON "eval_runs" USING btree ("org_id","project_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "file_versions_org_project_path_idx" ON "file_versions" USING btree ("org_id","project_id","path");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "provider_connections_org_project_idx" ON "provider_connections" USING btree ("org_id","project_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "queue_items_dataset_status_idx" ON "queue_items" USING btree ("dataset_id","status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "queue_items_org_project_idx" ON "queue_items" USING btree ("org_id","project_id");