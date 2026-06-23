use base64::{Engine as _, engine::general_purpose::STANDARD};

#[must_use]
pub fn redirect_lease_bundle() -> serde_json::Value {
    serde_json::json!({
        "runtime_files": [
            {
                "path": "wrangler.toml",
                "content_b64": STANDARD.encode(
                    "name = \"redirect-repro\"\nmain = \"worker_entry.mjs\"\ncompatibility_date = \"2026-03-23\"\n"
                )
            },
            {
                "path": "worker_entry.mjs",
                "content_b64": STANDARD.encode(
                    r#"export default {
  async fetch(request) {
    const url = new URL(request.url);
    if (url.pathname === "/health") return new Response("ok");
    if (url.pathname === "/protected") {
      return new Response(null, {
        status: 303,
        headers: { Location: "/recover?next=%2Fprotected" },
      });
    }
    if (url.pathname === "/recover") {
      return new Response("<h1>recover page</h1>", {
        status: 200,
        headers: { "Content-Type": "text/html; charset=utf-8" },
      });
    }
    return new Response("not found", { status: 404 });
  }
};
"#,
                )
            }
        ],
        "static_files": []
    })
}
