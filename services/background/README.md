# Compose shared background removal

The desktop app sends one image to this Rust service and receives a transparent
PNG. Only the server uses the bgclear key. Users need no account, token or key.
The provider call always requests its documented free `preview` size, not paid
full-resolution output. Images stay in memory for the request; no image files,
database or account system are created.

The endpoint is intentionally anonymous. Rate limits and concurrency bounds
protect the shared allowance; a static desktop token would be extractable and
would not authenticate users. Limits are per service process and reset on restart.
Keep one service instance initially. Operator limits and provider limits both apply.

## Render Free deployment

Compose's shared deployment is live at
`https://compose-background.onrender.com`. Health/readiness and the packaged native
Remove background flow were verified on 2026-10-11 using the bundled fictional
portrait: an 800 × 800 transparent PNG, unchanged bounds/style, Undo/Redo and
editable save/reopen. This validates that sample; it is not a broad quality
benchmark for every subject or an uptime guarantee.

The root `render.yaml` explicitly selects the free compute plan, Frankfurt,
Docker, `/health`, port 10000 and manual deploys. It creates no paid databases,
disks or workers. The provider key uses `sync: false`: enter it in Render's secret
field during creation. Do not add that value to this repository or desktop build.

[Deploy this repository to Render](https://render.com/deploy?repo=https://github.com/Imlokzu/compose)

After deployment, copy the actual HTTPS hostname from the dashboard and append
`/v1/background/remove` when building the desktop app. Do not assume that the
service name becomes the exact hostname; Render may add a suffix.

Render Free sleeps after 15 idle minutes and may need about a minute to wake.
The desktop client checks `/health` before posting image bytes, allowing up to
120 seconds for an `onrender.com` host to wake. This happens only when the user
requests removal; no periodic keep-alive is used. The image POST is sent once.

The Render profile accepts its real-client `X-Forwarded-For` entry only from a
private proxy peer and only when Render's own runtime flag is present. The generic
Docker profile continues to ignore untrusted forwarding headers. Limits are
in-memory and reset when a free instance sleeps/restarts; they are not durable
daily quota accounting. The fixed provider preview parameters remain enforced.

Free compute is intended for hobby/testing use and has monthly build, instance-hour
and bandwidth limits. With a payment method, some overages can be billed; without
one, exhausted limits can suspend the service. Do not add a paid plan/payment method
merely to complete this free deployment.

Sources checked 2026-10-11: [free limits](https://render.com/docs/free),
[Blueprint configuration](https://render.com/docs/blueprint-spec),
[proxy client IP](https://render.com/articles/how-render-handles-ddos-attacks).

## Run directly

Set `COMPOSE_BGCLEAR_API_KEY` through your server's secret settings, then:

```sh
cargo run -p compose-background-service --release --locked
```

The default listener is `127.0.0.1:8787`. Missing/invalid configuration fails at
startup. `COMPOSE_BGCLEAR_API_KEY_FILE` can instead name a mounted secret file.
Keep mounted secret files outside the Docker build context.
Do not put the provider key in desktop build variables, source or public config.

## Docker on your server

```sh
cp services/background/.env.example services/background/.env
# Edit that ignored .env on the server, entering the provider key there.
docker compose -f services/background/compose.yaml up -d --build
curl --fail http://127.0.0.1:8787/health
```

The image runs as a non-root user with a read-only filesystem, a 256 MB memory
limit, no added capabilities, and two concurrent uploads by default. Runtime
requires only the service executable; no GPU, Python or ML model is included.

Expose the listener through an HTTPS reverse proxy or your existing Cloudflare
Tunnel. Point a public hostname to `http://127.0.0.1:8787`. Only set
`COMPOSE_TRUSTED_PROXY_IPS` to exact immediate proxy/tunnel IPs you control; the
service otherwise ignores forwarded client-IP headers. If the tunnel runs in
another container, use its private container address/network and verify the
actual peer address before configuring trust. Never expose raw HTTP publicly.

## Connect a released desktop build

Build Compose with the public endpoint, without any provider secret:

```sh
COMPOSE_BACKGROUND_SERVICE_URL=https://your-service.example/v1/background/remove \
  ./scripts/package-macos.sh
```

Windows PowerShell:

```powershell
$env:COMPOSE_BACKGROUND_SERVICE_URL = 'https://your-service.example/v1/background/remove'
./scripts/package-windows.ps1
```

The packaging scripts default to the live Compose URL above; an explicit
`COMPOSE_BACKGROUND_SERVICE_URL` selects another deployment. The public URL is
compiled into the image client. Normal users just click
**Remove background** or **Without background** in search. A runtime
`COMPOSE_BACKGROUND_SERVICE_URL` override is available for local development;
HTTP is accepted only for loopback. A build with no endpoint explains that the
service is not connected; it never asks users for a provider key.

## API

| Route | Result |
|---|---|
| `GET /health` | Process liveness JSON |
| `GET /ready` | Local configuration ready; does not spend a provider call |
| `POST /v1/background/remove` | Raw image bytes in, transparent preview PNG out |

POST accepts an `image/*` content type and at most 25 MB / 32 megapixels.
Public error responses contain a typed code and request ID, never provider
errors, credentials or stack traces. Logs record only ID, status and duration.
Responses have `Cache-Control: no-store` and `X-Content-Type-Options: nosniff`.
Browser-origin requests are rejected; the native client requires no CORS access.

Defaults: 3 attempts/IP/minute, 30/IP/day, 500 total/day, 2 concurrent requests,
65-second server deadline and 55-second upstream deadline. Rejected/failed
attempts may consume a local limit slot. Change these with the named environment
variables in `.env.example`. Paid provider size is never selected automatically.

## Validation

```sh
cargo test -p compose-background-service -p image_tools --locked
```

Tests run a real local HTTP service with a controlled provider fixture: keyless
desktop client, PNG/alpha validation, health, invalid uploads, blocked origins,
spoofed forwarding headers, quotas, concurrency and sanitized provider failures.
They do not claim actual provider mask quality. An authenticated live cutout
requires the operator's real bgclear key and a reachable deployment.

The opt-in packaged native check sends the bundled sample portrait to the configured
HTTPS service. It uses the compiled public URL when no runtime override is set:

```sh
dist/Compose.app/Contents/MacOS/compose --ui-smoke --shared-service-live --compact
```

It verifies the actual button, transparent image, original geometry, Undo/Redo and
native save/reopen. Its files are written under `artifacts/shared-service/live/`.
The ordinary `--shared-service` check remains restricted to a loopback fixture.
