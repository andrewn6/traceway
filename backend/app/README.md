# Traceway Encore Backend

This service is now the browser-facing product API for Traceway.

## Local development

1. Install Encore CLI: <https://encore.dev/docs/ts/install>
2. Create local env file from template:

```bash
cp .env.example .env
```

3. Run backend:

```bash
encore run
```

Encore local dashboard: <http://localhost:9400>

## API base URL

- Local Encore API endpoint is typically `http://localhost:4000`.
- UI should use `VITE_API_URL=http://localhost:4000`.

## Key env vars

- `ALLOWED_ORIGINS`: browser origins for CORS
- `TRACEWAY_BACKEND_TOKEN`: internal service token for system calls
- `POLAR_ACCESS_TOKEN`, `POLAR_WEBHOOK_SECRET`: billing integration
- `TURBOPUFFER_API_KEY`, `TURBOPUFFER_UPSERT_URL`: secondary search indexing

See `.env.example` for the full list.

## Migrations

Generate SQL migrations after schema changes:

```bash
npx drizzle-kit generate
```

## OpenAPI + UI types

From repo root:

```bash
encore gen client --lang openapi --output "../../openapi.json"
npx openapi-typescript "openapi.json" -o "ui/src/lib/api-types.ts"
```
