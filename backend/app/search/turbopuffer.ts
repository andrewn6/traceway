type MirrorDoc = {
  id: string;
  kind: "trace" | "span";
  org_id: string;
  project_id: string;
  trace_id?: string;
  text: string;
  metadata: Record<string, unknown>;
};

async function postDoc(doc: MirrorDoc): Promise<void> {
  const endpoint = process.env.TURBOPUFFER_UPSERT_URL;
  const apiKey = process.env.TURBOPUFFER_API_KEY;
  if (!endpoint || !apiKey) {
    return;
  }

  await fetch(endpoint, {
    method: "POST",
    headers: {
      authorization: `Bearer ${apiKey}`,
      "content-type": "application/json",
    },
    body: JSON.stringify({
      documents: [doc],
    }),
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
