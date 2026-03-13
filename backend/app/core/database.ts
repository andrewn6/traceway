import { SQLDatabase } from "encore.dev/storage/sqldb";
import { drizzle } from "drizzle-orm/pg-proxy";

const DB = new SQLDatabase("traceway_backend", {
  migrations: {
    path: "./migrations",
    source: "drizzle",
  },
});

export const db = drizzle(async (sql, params, method) => {
  if (method === "execute") {
    await DB.rawExec(sql, ...params);
    return { rows: [] };
  }

  const rows: Record<string, unknown>[] = [];
  for await (const row of DB.rawQuery(sql, ...params)) {
    rows.push(row);
  }
  return { rows };
});
