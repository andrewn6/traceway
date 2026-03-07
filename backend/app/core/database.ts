import { SQLDatabase } from "encore.dev/storage/sqldb";
import { drizzle } from "drizzle-orm/node-postgres";
import { Pool } from "pg";

const DB = new SQLDatabase("traceway_backend", {
  migrations: {
    path: "./migrations",
    source: "drizzle",
  },
});

const pool = new Pool({
  connectionString: DB.connectionString,
  max: Number(process.env.PG_POOL_MAX ?? "4"),
  idleTimeoutMillis: Number(process.env.PG_IDLE_TIMEOUT_MS ?? "30000"),
  connectionTimeoutMillis: Number(process.env.PG_CONNECT_TIMEOUT_MS ?? "10000"),
  allowExitOnIdle: false,
});

export const db = drizzle(pool);
