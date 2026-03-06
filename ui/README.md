# Traceway UI

SvelteKit frontend for Traceway.

## Local development

1. Install deps:

```sh
npm install
```

2. Configure API URL:

```sh
cp .env.example .env
```

3. Run dev server:

```sh
npm run dev
```

## API wiring

- UI calls Encore public APIs directly.
- Set `VITE_API_URL` to your Encore base URL.
  - local: `http://localhost:4000`
  - prod: `https://api.traceway.ai`

## Build

```sh
npm run build
npm run preview
```
