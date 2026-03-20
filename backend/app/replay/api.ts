import { api } from "encore.dev/api";
import { handlePreflight, json, requireScope, setCors } from "../shared/http";
import { pathSegments } from "../shared/request";
import { getReplayableTrace } from "./service";

export const getReplayableTraceEndpoint = api.raw(
  { expose: true, method: "GET", path: "/replay/:trace_id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const traceId = pathSegments(req)[1] ?? "";
    const trace = await getReplayableTrace(session, traceId);
    if (!trace) {
      json(res, 404, { error: "Trace not found" });
      return;
    }
    json(res, 200, trace);
  }
);
