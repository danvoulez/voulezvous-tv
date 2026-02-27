interface Env {
  DB: D1Database;
  CONTROL_TOKEN: string;
  CONTROL_SECRET: string;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (request.method === "GET" && url.pathname === "/v1/status") {
      const row = await env.DB.prepare("SELECT payload_json FROM status_snapshots WHERE id = 1").first<{ payload_json: string }>();
      if (!row) {
        return Response.json({ state: "RUNNING", buffer_minutes: 60, source: "default" });
      }
      return Response.json(JSON.parse(row.payload_json));
    }

    if (request.method === "GET" && url.pathname === "/v1/reports/daily") {
      const date = url.searchParams.get("date");
      if (!date) return jsonError("missing date", 400);
      const row = await env.DB.prepare("SELECT payload_json FROM daily_reports WHERE date = ?1").bind(date).first<{ payload_json: string }>();
      if (!row) return jsonError("daily report not found", 404);
      return Response.json(JSON.parse(row.payload_json));
    }

    if (request.method === "GET" && url.pathname === "/v1/reports/weekly") {
      const week = url.searchParams.get("week");
      if (!week) return jsonError("missing week", 400);
      const row = await env.DB.prepare("SELECT payload_json FROM weekly_reports WHERE week = ?1").bind(week).first<{ payload_json: string }>();
      if (!row) return jsonError("weekly report not found", 404);
      return Response.json(JSON.parse(row.payload_json));
    }

    if (request.method === "POST" && url.pathname.startsWith("/v1/ingest/")) {
      const bodyText = await request.text();
      const authErr = await verifyControlAuth(request, env, bodyText);
      if (authErr) return authErr;

      if (url.pathname === "/v1/ingest/status") {
        await env.DB
          .prepare(
            "INSERT INTO status_snapshots(id, payload_json, updated_at) VALUES(1, ?1, datetime('now')) ON CONFLICT(id) DO UPDATE SET payload_json = excluded.payload_json, updated_at = excluded.updated_at"
          )
          .bind(bodyText)
          .run();
        return Response.json({ ok: true, kind: "status" });
      }

      if (url.pathname === "/v1/ingest/daily") {
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

      if (url.pathname === "/v1/ingest/weekly") {
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
