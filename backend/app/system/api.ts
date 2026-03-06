import { api } from "encore.dev/api";

import { stats } from "../tracing/service";
import { handlePreflight, json, requireSession, setCors } from "../shared/http";

export const healthPublic = api.raw({ expose: true, method: "GET", path: "/health" }, async (req, res) => {
  if (handlePreflight(req, res)) return;
  setCors(req, res);
  const session = await requireSession(req, res);
  if (!session) return;
  const count = await stats(session);
  json(res, 200, {
    status: "ok",
    version: "encore",
    uptime_secs: Math.floor(process.uptime()),
    storage: {
      backend: "postgres",
      trace_count: count.trace_count,
      span_count: count.span_count,
    },
  });
});
