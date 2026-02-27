interface Env {
  DB: D1Database;
  CONTROL_TOKEN: string;
  CONTROL_SECRET: string;
  PUBLIC_STREAM_URL?: string;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const path = normalizePath(url.pathname);

    if (request.method === "GET" && path === "/") {
      return new Response(publicPageHtml(env.PUBLIC_STREAM_URL), {
        headers: {
          "content-type": "text/html; charset=utf-8",
          "cache-control": "public, max-age=120",
        },
      });
    }

    if (request.method === "GET" && path === "/v1/status") {
      const row = await env.DB.prepare("SELECT payload_json FROM status_snapshots WHERE id = 1").first<{ payload_json: string }>();
      if (!row) {
        return Response.json({ state: "RUNNING", buffer_minutes: 60, source: "default" });
      }
      return Response.json(JSON.parse(row.payload_json));
    }

    if (request.method === "GET" && path === "/v1/reports/daily") {
      const date = url.searchParams.get("date");
      if (!date) return jsonError("missing date", 400);
      const row = await env.DB.prepare("SELECT payload_json FROM daily_reports WHERE date = ?1").bind(date).first<{ payload_json: string }>();
      if (!row) return jsonError("daily report not found", 404);
      return Response.json(JSON.parse(row.payload_json));
    }

    if (request.method === "GET" && path === "/v1/reports/weekly") {
      const week = url.searchParams.get("week");
      if (!week) return jsonError("missing week", 400);
      const row = await env.DB.prepare("SELECT payload_json FROM weekly_reports WHERE week = ?1").bind(week).first<{ payload_json: string }>();
      if (!row) return jsonError("weekly report not found", 404);
      return Response.json(JSON.parse(row.payload_json));
    }

    if (request.method === "POST" && path.startsWith("/v1/ingest/")) {
      const bodyText = await request.text();
      const authErr = await verifyControlAuth(request, env, bodyText);
      if (authErr) return authErr;

      if (path === "/v1/ingest/status") {
        await env.DB
          .prepare(
            "INSERT INTO status_snapshots(id, payload_json, updated_at) VALUES(1, ?1, datetime('now')) ON CONFLICT(id) DO UPDATE SET payload_json = excluded.payload_json, updated_at = excluded.updated_at"
          )
          .bind(bodyText)
          .run();
        return Response.json({ ok: true, kind: "status" });
      }

      if (path === "/v1/ingest/daily") {
        const payload = JSON.parse(bodyText) as { date?: string };
        if (!payload.date) return jsonError("daily report missing date", 400);
        await env.DB
          .prepare(
            "INSERT INTO daily_reports(date, payload_json, updated_at) VALUES(?1, ?2, datetime('now')) ON CONFLICT(date) DO UPDATE SET payload_json = excluded.payload_json, updated_at = excluded.updated_at"
          )
          .bind(payload.date, bodyText)
          .run();
        return Response.json({ ok: true, kind: "daily" });
      }

      if (path === "/v1/ingest/weekly") {
        const payload = JSON.parse(bodyText) as { week?: string };
        if (!payload.week) return jsonError("weekly report missing week", 400);
        await env.DB
          .prepare(
            "INSERT INTO weekly_reports(week, payload_json, updated_at) VALUES(?1, ?2, datetime('now')) ON CONFLICT(week) DO UPDATE SET payload_json = excluded.payload_json, updated_at = excluded.updated_at"
          )
          .bind(payload.week, bodyText)
          .run();
        return Response.json({ ok: true, kind: "weekly" });
      }

      return jsonError("unsupported ingest endpoint", 404);
    }

    return jsonError("not found", 404);
  },
};

function normalizePath(pathname: string): string {
  if (pathname === "/vvtv") return "/";
  if (pathname.startsWith("/vvtv/")) {
    return pathname.slice("/vvtv".length);
  }
  return pathname;
}

function publicPageHtml(streamUrl?: string): string {
  const src = streamUrl || "https://core.voulezvous.tv/hls/index.m3u8";
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover" />
  <title>voulezvous.tv</title>
  <style>
    :root {
      --bg: #000000;
      --panel: #1a1a1a;
      --panel-border: #2e2e2e;
      --pink: #ff1684;
      --text: #f6f6f6;
    }
    * { box-sizing: border-box; }
    html, body {
      margin: 0;
      width: 100%;
      height: 100%;
      background: var(--bg);
      color: var(--text);
      font-family: "Helvetica Neue", Helvetica, Arial, sans-serif;
    }
    .page {
      min-height: 100%;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 3vh 3vw;
    }
    .stack {
      width: min(94vw, 1800px);
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 2vh;
    }
    .player-shell {
      width: 100%;
      aspect-ratio: 16 / 9;
      background: var(--panel);
      border: 1px solid var(--panel-border);
      border-radius: 18px;
      overflow: hidden;
      position: relative;
      box-shadow: 0 20px 48px rgba(0, 0, 0, 0.58);
      touch-action: manipulation;
    }
    video {
      width: 100%;
      height: 100%;
      object-fit: cover;
      background: #111;
    }
    .overlay {
      position: absolute;
      inset: 0;
      display: grid;
      place-items: center;
      background: linear-gradient(to bottom, rgba(0,0,0,0.28), rgba(0,0,0,0.46));
      transition: opacity .2s ease;
      cursor: pointer;
    }
    .overlay.hidden {
      opacity: 0;
      pointer-events: none;
    }
    .play {
      width: 18%;
      min-width: 120px;
      height: 50%;
      border-radius: 999px;
      border: 0;
      background: rgba(255, 255, 255, 0.12);
      backdrop-filter: blur(8px);
      color: #fff;
      font-size: clamp(2.4rem, 6vw, 5.5rem);
      line-height: 1;
      display: grid;
      place-items: center;
      padding-left: 0.14em;
    }
    .brand {
      letter-spacing: 0.12em;
      text-transform: lowercase;
      font-size: clamp(1.05rem, 2.2vw, 2.2rem);
      font-weight: 700;
      color: var(--pink);
      text-shadow: 0 0 12px rgba(255, 22, 132, 0.45);
      user-select: none;
    }
    @media (max-width: 900px) {
      .page { padding: 2vh 2vw; }
      .player-shell { border-radius: 10px; }
      .play { min-width: 88px; }
    }
  </style>
</head>
<body>
  <main class="page">
    <section class="stack">
      <div class="player-shell" id="shell">
        <video id="stream" preload="metadata" playsinline webkit-playsinline></video>
        <div class="overlay" id="overlay" aria-label="Play stream">
          <button class="play" id="playBtn" aria-label="Play">▶</button>
        </div>
      </div>
      <div class="brand">voulezvous</div>
    </section>
  </main>
  <script src="https://cdn.jsdelivr.net/npm/hls.js@1.5.20/dist/hls.min.js"></script>
  <script>
    const STREAM_URL = ${JSON.stringify(src)};
    const shell = document.getElementById("shell");
    const video = document.getElementById("stream");
    const overlay = document.getElementById("overlay");
    const playBtn = document.getElementById("playBtn");

    function loadStream() {
      if (window.Hls && window.Hls.isSupported()) {
        const hls = new Hls({ lowLatencyMode: true, enableWorker: true });
        hls.loadSource(STREAM_URL);
        hls.attachMedia(video);
      } else {
        video.src = STREAM_URL;
      }
    }

    function showOverlay(show) {
      overlay.classList.toggle("hidden", !show);
    }

    async function goLive() {
      if (!document.fullscreenElement) {
        try { await shell.requestFullscreen(); } catch (_) {}
      }
      try {
        await video.play();
        showOverlay(false);
      } catch (_) {
        showOverlay(true);
      }
    }

    shell.addEventListener("mouseenter", () => { video.controls = true; });
    shell.addEventListener("mouseleave", () => { video.controls = false; });
    video.addEventListener("pause", () => showOverlay(true));
    video.addEventListener("playing", () => showOverlay(false));
    overlay.addEventListener("click", goLive);
    playBtn.addEventListener("click", goLive);

    loadStream();
    showOverlay(true);
  </script>
</body>
</html>`;
}

async function verifyControlAuth(request: Request, env: Env, body: string): Promise<Response | null> {
  const auth = request.headers.get("authorization") || "";
  if (auth !== `Bearer ${env.CONTROL_TOKEN}`) {
    return jsonError("invalid token", 401);
  }

  const ts = request.headers.get("x-vvtv-ts") || "";
  const sig = request.headers.get("x-vvtv-signature") || "";
  if (!ts || !sig) {
    return jsonError("missing signature headers", 401);
  }

  const tsNum = Number(ts);
  if (!Number.isFinite(tsNum)) {
    return jsonError("invalid timestamp", 401);
  }
  const now = Math.floor(Date.now() / 1000);
  if (Math.abs(now - tsNum) > 300) {
    return jsonError("stale timestamp", 401);
  }

  const url = new URL(request.url);
  const canonical = `${request.method}\n${url.pathname}\n${ts}\n${body}`;
  const expected = await hmacSha256Hex(env.CONTROL_SECRET, canonical);
  if (expected !== sig) {
    return jsonError("invalid signature", 401);
  }

  return null;
}

async function hmacSha256Hex(secret: string, data: string): Promise<string> {
  const enc = new TextEncoder();
  const key = await crypto.subtle.importKey(
    "raw",
    enc.encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"]
  );
  const signature = await crypto.subtle.sign("HMAC", key, enc.encode(data));
  const bytes = new Uint8Array(signature);
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

function jsonError(message: string, status: number): Response {
  return Response.json({ error: message }, { status });
}
