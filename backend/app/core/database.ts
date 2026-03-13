import { SQLDatabase } from "encore.dev/storage/sqldb";
import { drizzle } from "drizzle-orm/node-postgres";

const DB = new SQLDatabase("traceway_backend", {
  migrations: {
    path: "./migrations",
    source: "drizzle",
  },
});

export const db = drizzle(DB.connectionString);
