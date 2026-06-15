# Deploying the Dashboard to Vercel

The Guardrail dashboard is a Next.js 16 app in [`dashboard/`](../dashboard). The
Rust engine, API, and analytics are **not** deployed to Vercel — only the
read-only dashboard is. It talks to a running `guardrail-api` over HTTP.

## One-time project setup

1. Import `github.com/caelum0x/guardrail` into Vercel.
2. **Set Root Directory to `dashboard`** (Project → Settings → General → Root
   Directory). This is required because the Next.js app lives in a subdirectory;
   Vercel then reads [`dashboard/vercel.json`](../dashboard/vercel.json).
3. Framework preset: **Next.js** (auto-detected).
4. Environment variables (Project → Settings → Environment Variables):
   - `NEXT_PUBLIC_API_URL` — the public URL of your `guardrail-api`
     (e.g. `https://api.your-host.example`). The dashboard fetches the read-only
     API from this base. With no value it falls back to `http://localhost:8080`
     for local dev.

## Continuous deployment

With the GitHub integration connected, every push to `main` triggers a
production deployment (`git.deploymentEnabled.main` in `dashboard/vercel.json`).
Pull requests get preview deployments automatically.

## Manual deploy (optional)

From a machine where you've run `vercel login`:

```bash
cd dashboard
vercel --prod
```

## Notes

- The dashboard is read-only: it never holds keys or signs transactions
  (self-custody stays with TWAK + the operator). It only renders what the API
  exposes.
- For a zero-dependency alternative that needs no build, see the static cockpit
  in [`clients/web-lite/`](../clients/web-lite) — open `index.html` directly.
