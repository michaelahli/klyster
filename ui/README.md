# Klyster UI

Svelte 5 + Vite + TypeScript dashboard for the Klyster capacity planning service.

## Quick start

```bash
cd ui
npm install
npm run dev      # http://localhost:5173, proxies /api and /ws to 127.0.0.1:8080
```

## Scripts

| Command           | Description                                                |
| ----------------- | ---------------------------------------------------------- |
| `npm run dev`     | Start Vite dev server with API proxy.                      |
| `npm run build`   | Type-check with `svelte-check` then build into `ui/dist/`. |
| `npm run preview` | Serve the production bundle locally.                       |
| `npm run check`   | Type-check only (no bundle).                               |
| `npm run lint`    | ESLint over `.ts` / `.svelte`.                             |
| `npm run format`  | Format the project with Prettier.                          |

## Project layout

```
ui/
├── index.html              # Vite entry HTML
├── package.json
├── tsconfig.json           # references app + node configs
├── tsconfig.app.json       # client-side TS settings
├── tsconfig.node.json      # vite.config.ts settings
├── vite.config.ts          # dev server proxy + build options
├── eslint.config.js
├── svelte.config.js
├── .prettierrc.json
├── public/
│   └── favicon.svg
└── src/
    ├── main.ts             # mounts App.svelte
    ├── App.svelte          # placeholder shell (CP-M5-001)
    ├── styles/
    │   └── app.css         # global theme tokens
    └── vite-env.d.ts
```

## Backend integration

The dev server proxies `/api`, `/healthz`, `/readyz`, `/metrics`, and `/ws/*`
to the Rust web service on `127.0.0.1:8080`. In production the Rust binary
will embed the `dist/` output via `rust-embed` (see CP-M5-002).
