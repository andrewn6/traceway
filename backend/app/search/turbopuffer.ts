import { Turbopuffer } from "@turbopuffer/turbopuffer";
import type { Row } from "@turbopuffer/turbopuffer/resources";

export type MirrorDoc = {
  id: string;
  kind: "trace" | "span";
  org_id: string;
  project_id: string;
  trace_id?: string;
  text: string;
  metadata: Record<string, unknown>;
};

const defaultRegion = "gcp-us-central1";
const defaultNamespace = "traceway";

const mirrorSchema = {
  text: {
    type: "string" as const,
    full_text_search: {
      language: "english" as const,
      stemming: true,
      remove_stopwords: true,
      case_sensitive: false,
    },
  },
  kind: { type: "string" as const, filterable: true },
  org_id: { type: "string" as const, filterable: true },
  project_id: { type: "string" as const, filterable: true },
  trace_id: { type: "string" as const, filterable: true },
  metadata_json: { type: "string" as const },
};

let client: Turbopuffer | null = null;
let clientCacheKey = "";

function turbopufferTimeoutMs(): number | undefined {
  const raw = process.env.TURBOPUFFER_TIMEOUT;
  if (!raw) return undefined;
  const secs = Number(raw);
  if (!Number.isFinite(secs) || secs <= 0) return undefined;
  return Math.max(1000, Math.floor(secs * 1000));
}

function getClient(): Turbopuffer | null {
  const apiKey = process.env.TURBOPUFFER_API_KEY?.trim();
  if (!apiKey) return null;

  const region = (process.env.TURBOPUFFER_REGION ?? defaultRegion).trim();
  const baseURL = process.env.TURBOPUFFER_BASE_URL?.trim() ?? "";
  const key = `${apiKey}\0${region}\0${baseURL}`;
  if (client && clientCacheKey === key) return client;

  const timeout = turbopufferTimeoutMs();
  clientCacheKey = key;
  client = new Turbopuffer({
    apiKey,
    region,
    ...(baseURL ? { baseURL } : {}),
    ...(timeout != null ? { timeout } : {}),
  });
  return client;
}

function namespaceName(): string {
  return (process.env.TURBOPUFFER_NAMESPACE ?? defaultNamespace).trim() || defaultNamespace;
}

async function postDoc(doc: MirrorDoc): Promise<void> {
  const tpuf = getClient();
  if (!tpuf) return;

  const row: Row = {
    id: doc.id,
    kind: doc.kind,
    org_id: doc.org_id,
    project_id: doc.project_id,
    text: doc.text,
    metadata_json: JSON.stringify(doc.metadata),
    ...(doc.trace_id ? { trace_id: doc.trace_id } : {}),
  };

  await tpuf.namespace(namespaceName()).write({
    upsert_rows: [row],
    schema: mirrorSchema,
  });
}

export async function mirrorTrace(doc: MirrorDoc): Promise<void> {
  try {
    await postDoc(doc);
  } catch {
    // best-effort mirror only
  }
}

export async function mirrorSpan(doc: MirrorDoc): Promise<void> {
  try {
    await postDoc(doc);
  } catch {
    // best-effort mirror only
  }
}
