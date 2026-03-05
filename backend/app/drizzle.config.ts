import { defineConfig } from "drizzle-kit";

export default defineConfig({
  out: "./core/migrations",
  schema: "./core/schema.ts",
  dialect: "postgresql",
});
