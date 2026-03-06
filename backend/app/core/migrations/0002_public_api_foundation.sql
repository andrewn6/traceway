CREATE TABLE IF NOT EXISTS "api_keys" (
	"id" uuid PRIMARY KEY NOT NULL,
	"org_id" uuid NOT NULL,
	"name" text NOT NULL,
	"key_prefix" text NOT NULL,
	"key_hash" text NOT NULL,
	"scopes" jsonb NOT NULL,
	"created_at" timestamp with time zone NOT NULL,
	"last_used_at" timestamp with time zone,
	CONSTRAINT "api_keys_key_hash_unique" UNIQUE("key_hash")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "invites" (
	"id" uuid PRIMARY KEY NOT NULL,
	"org_id" uuid NOT NULL,
	"email" text NOT NULL,
	"role" text DEFAULT 'member' NOT NULL,
	"invited_by" uuid NOT NULL,
	"token_hash" text NOT NULL,
	"expires_at" timestamp with time zone NOT NULL,
	"created_at" timestamp with time zone NOT NULL,
	CONSTRAINT "invites_token_hash_unique" UNIQUE("token_hash")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "password_resets" (
	"id" uuid PRIMARY KEY NOT NULL,
	"user_id" uuid NOT NULL,
	"token_hash" text NOT NULL,
	"expires_at" timestamp with time zone NOT NULL,
	"created_at" timestamp with time zone NOT NULL,
	"used_at" timestamp with time zone,
	CONSTRAINT "password_resets_token_hash_unique" UNIQUE("token_hash")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "traces" (
	"id" uuid PRIMARY KEY NOT NULL,
	"org_id" uuid NOT NULL,
	"project_id" uuid NOT NULL,
	"name" text,
	"tags" jsonb DEFAULT '[]'::jsonb NOT NULL,
	"started_at" timestamp with time zone NOT NULL,
	"ended_at" timestamp with time zone,
	"machine_id" text
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "spans" (
	"id" uuid PRIMARY KEY NOT NULL,
	"org_id" uuid NOT NULL,
	"project_id" uuid NOT NULL,
	"trace_id" uuid NOT NULL,
	"parent_id" uuid,
	"name" text NOT NULL,
	"kind" jsonb NOT NULL,
	"status" jsonb NOT NULL,
	"input" jsonb,
	"output" jsonb,
	"started_at" timestamp with time zone NOT NULL,
	"ended_at" timestamp with time zone
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "file_contents" (
	"hash" text PRIMARY KEY NOT NULL,
	"content" text NOT NULL,
	"created_at" timestamp with time zone NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "event_log" (
	"id" bigserial PRIMARY KEY NOT NULL,
	"org_id" uuid NOT NULL,
	"project_id" uuid NOT NULL,
	"event_type" text NOT NULL,
	"payload" jsonb NOT NULL,
	"created_at" timestamp with time zone NOT NULL
);
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "api_keys" ADD CONSTRAINT "api_keys_org_id_organizations_id_fk" FOREIGN KEY ("org_id") REFERENCES "public"."organizations"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "invites" ADD CONSTRAINT "invites_org_id_organizations_id_fk" FOREIGN KEY ("org_id") REFERENCES "public"."organizations"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "invites" ADD CONSTRAINT "invites_invited_by_users_id_fk" FOREIGN KEY ("invited_by") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "password_resets" ADD CONSTRAINT "password_resets_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "api_keys_org_idx" ON "api_keys" USING btree ("org_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "api_keys_prefix_idx" ON "api_keys" USING btree ("key_prefix");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "invites_org_idx" ON "invites" USING btree ("org_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "invites_email_idx" ON "invites" USING btree ("email");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "invites_token_hash_idx" ON "invites" USING btree ("token_hash");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "password_resets_user_idx" ON "password_resets" USING btree ("user_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "password_resets_token_idx" ON "password_resets" USING btree ("token_hash");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "traces_org_project_started_idx" ON "traces" USING btree ("org_id", "project_id", "started_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "spans_org_project_trace_idx" ON "spans" USING btree ("org_id", "project_id", "trace_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "spans_org_project_started_idx" ON "spans" USING btree ("org_id", "project_id", "started_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "file_contents_created_idx" ON "file_contents" USING btree ("created_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "event_log_org_project_id_idx" ON "event_log" USING btree ("org_id", "project_id", "id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "event_log_created_idx" ON "event_log" USING btree ("created_at");
