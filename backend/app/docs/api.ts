import { api } from "encore.dev/api";
import { promises as fs } from "node:fs";

const SCALAR_HTML = `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Traceway Backend API Docs</title>
  </head>
  <body>
    <script id="api-reference" data-url="/openapi.json"></script>
    <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
  </body>
</html>`;

export const openapiSpec = api.raw(
  { expose: true, method: "GET", path: "/openapi.json" },
  async (_req, res) => {
    try {
      const content = await fs.readFile("openapi.json", "utf8");
      res.setHeader("content-type", "application/json; charset=utf-8");
      res.end(content);
    } catch {
      res.statusCode = 503;
      res.setHeader("content-type", "application/json; charset=utf-8");
      res.end(JSON.stringify({ error: "openapi.json not found; run encore gen client --lang openapi --output openapi.json" }));
    }
  }
);

export const scalarDocs = api.raw(
  { expose: true, method: "GET", path: "/docs" },
  async (_req, res) => {
    res.setHeader("content-type", "text/html; charset=utf-8");
    res.end(SCALAR_HTML);
  }
);
