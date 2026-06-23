use base64::{Engine as _, engine::general_purpose::STANDARD};

// Localhost test/dev certificate material only. These fixtures self-sign the
// fake HTTPS worker used by integration tests. Users may replace this key/cert
// pair with their own self-signed localhost certificate, or keep using this one.
const CERT_PEM: &str = r"-----BEGIN CERTIFICATE-----
MIIDCTCCAfGgAwIBAgIUa4bmGbFzGAtopO9BxwcCgzv69fswDQYJKoZIhvcNAQEL
BQAwFDESMBAGA1UEAwwJbG9jYWxob3N0MB4XDTI2MDQyMzIxMDMzOVoXDTI2MDQy
NDIxMDMzOVowFDESMBAGA1UEAwwJbG9jYWxob3N0MIIBIjANBgkqhkiG9w0BAQEF
AAOCAQ8AMIIBCgKCAQEApgDR5YL/Z8YcNWNkcA8C2kdnzMQ2KgF9RtiCTLeZbrea
4ychqxMW3bJBVnNA9pXcB+MJrf49gUb4NciZHxDInP3rSQ6dICmUiMVpIlTw7gBa
10FyMJDY6j5uDkK3T33VgxOpK/XGYMslOcdvAuGD4L2qp72RpcKcBLwJqETXby31
0WTe/wy2mzC0CQMPqecXjt9STothVjrSc/cm05axtuBxPiJpmBaNwyO+mZaTfr54
a7DU5xxW7rtoPUzYxTW/cvnmj34Emf/JW0YJNeimcO0ahzxpNlR65L0AowhqdfvU
BXlehLukaFWwm0gJY2+6FEYXo92a0kJ9UekjFy+EsQIDAQABo1MwUTAdBgNVHQ4E
FgQUOOf4Oi9Ap2aZ1sK84M42NQ6o/w8wHwYDVR0jBBgwFoAUOOf4Oi9Ap2aZ1sK8
4M42NQ6o/w8wDwYDVR0TAQH/BAUwAwEB/zANBgkqhkiG9w0BAQsFAAOCAQEACCZq
MuYfZCDBKxnF54imPmDR2elEMYytSGVewa7EstSbKp+mEAtMTFh/VPrVMs5fDElA
5WAYlQokD223F/8OkDcirADtCpgV117CweqKYdJANPUbz0pagOCes2cPE0nSBo6n
gqAwrrbg9qROz0Veh+lAg5r7N7nPHGdWr0KCugeMJx5oS8PVAJnHBC7+QraLIcC1
Zpf9TBWqG7kA7VENtS7zj4LYBajcxC4sNKJiHpo1JvB8yrcP6LQdHxoypBLEdalc
pGVvdoLerkibE1qwOzjilDJyZSaHwIpq4UTSJEh12Drd+lZgYkWAt4Wdelj+8GRF
xMyvGXvKTgB8K/XLZw==
-----END CERTIFICATE-----";

const KEY_PEM: &str = r"-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQCmANHlgv9nxhw1
Y2RwDwLaR2fMxDYqAX1G2IJMt5lut5rjJyGrExbdskFWc0D2ldwH4wmt/j2BRvg1
yJkfEMic/etJDp0gKZSIxWkiVPDuAFrXQXIwkNjqPm4OQrdPfdWDE6kr9cZgyyU5
x28C4YPgvaqnvZGlwpwEvAmoRNdvLfXRZN7/DLabMLQJAw+p5xeO31JOi2FWOtJz
9ybTlrG24HE+ImmYFo3DI76ZlpN+vnhrsNTnHFbuu2g9TNjFNb9y+eaPfgSZ/8lb
Rgk16KZw7RqHPGk2VHrkvQCjCGp1+9QFeV6Eu6RoVbCbSAljb7oURhej3ZrSQn1R
6SMXL4SxAgMBAAECggEABFkBxlCAMNMoiJms/w4Wl5nOiTdzmXWq5rrLjVey6Rpe
EDBVfnf4Vae6NDneKAMoZUy8A3qE2PyfSjXR9FVFws5X7eS4BuhN4YflkzKNMZeQ
6YG3XFgj1T07t8wFpetkVtuyRuJnzt/NLppngPhf8kg3Zq+lZ5VtbYaQ/G8qowjS
0AREux07mkizlmpPbQOa70LtYNMsFBQndW6/rqGo6hemlDlgcC8+ZdBNMN+Vo08/
zSp4M0TboSOShhm1dhlre7JNonttDOmgy5bwWGWBbvXxtmfIzKumRuH97VJM7ebn
HfeFI4X46ookHzLJLeg4InqLJqIG2etFHwaAxHp3IQKBgQDnhYB2dNaERObPda/e
IC3Ymnx+tuHhlmK5hRGyw5gvcP+nGm1xtjbFAH3hp0fvR5ulxlFBQBfhYubgBJp8
XneVjIb9DfBrIn9GRDkCirUDyk9hUPkciGj3/vUDA5AC3T1sy6I8qvS78PiumdlG
Alu9aS8Q5jmJpcQ8MRyrHAEGYQKBgQC3jfYQWylY15vgjL2RVaq/xYJvGttIGV8n
J4WxmmW607TSZgJLhCVeBOK7juzrzXuaqeHDlgGOgC0QN4sd8nH5mMaNZy64Ez1n
EJC9uTv+h6Y1Uu/LMwWXflraevE0HtBalitYxGxWuuY55OKL+tzj0Laj1KEm2syI
DxjX/yyAUQKBgE54tGrx/QeF5wloJTzPgVqKOiokyHjSpRGmZbJGk115Gl3EGlZR
YUzydrg2H66dgcb1afMIy09MW16QkOAYkMWyhMpeoB9f6O2jEAOpieMH/lHIxTaD
kbiExPzJh1VBMaGff5S6iQruiQt8/+S3xep0LUy4C8Z41gNkzge9DSfhAoGAQX+8
4JVHKda2XuiZ9LSXG5uFMdRpj07Ob5Bg2sF3r7U//xw7kWr0Upp3teoIjRRkGQqp
7zsmDw2aBwFq+SK8nQ5xO6AoQbNL4+07WRgyWl9ZZgnUW7Q3OJn0HaZlT+3293xC
t4hQCJrk8J/GX6EDFaAyrD+ByfWlqp+Ig/sgjCECgYBvQvXVvrvfxP38NnS83oIw
2KslCDaRMmoKKcoxenijo45Z+4n0SSpMrqO3pf38o/yx1Dc8Wr1rkP/ndvkYTBpS
UeFs0jwC/wHbuJkCP+6JnirwWJnNRnd2pgKd68mhChTKY4lpP9EpKw/J7gJYkF7+
v/T4lWKsYQdTZ1e+biUfjg==
-----END PRIVATE KEY-----";

#[must_use]
pub fn lease_bundle() -> serde_json::Value {
    serde_json::json!({
        "runtime_files": [
            {
                "path": "wrangler.toml",
                "content_b64": STANDARD.encode(
                    "name = \"food-tracker\"\nmain = \"worker_entry.mjs\"\ncompatibility_date = \"2026-03-23\"\n"
                )
            },
            {
                "path": "worker_entry.mjs",
                "content_b64": STANDARD.encode(
                    r#"export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (url.pathname === "/health") return new Response("ok");
    if (url.pathname === "/env") {
      const name = url.searchParams.get("name") || "";
      return new Response(env[name] || "");
    }
    return new Response("not found", { status: 404 });
  }
};
"#,
                )
            }
        ],
        "static_files": [
            {
                "path": "index.html",
                "content_b64": STANDARD.encode("<h1>ok</h1>\n")
            }
        ]
    })
}

#[must_use]
pub fn fake_wrangler_script() -> String {
    [
        r#"#!/usr/bin/env python3
import http.server
import os
import socketserver
import ssl
import sys
import urllib.parse
from pathlib import Path

args = sys.argv[1:]
port = 0
for index, arg in enumerate(args):
    if arg == "--port" and index + 1 < len(args):
        port = int(args[index + 1])

def env_value(name):
    return os.environ.get(name, "")

class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        if parsed.path == "/health":
            self.send_response(200)
            self.send_header("content-type", "text/plain")
            self.end_headers()
            self.wfile.write(b"ok")
            print(f"health {self.server.server_port}", flush=True)
            return

        if parsed.path == "/env":
            params = urllib.parse.parse_qs(parsed.query)
            name = params.get("name", [""])[0]
            value = env_value(name)
            print(f"env {name}={value}", flush=True)
            self.send_response(200)
            self.send_header("content-type", "text/plain")
            self.end_headers()
            self.wfile.write(value.encode("utf-8"))
            return

        self.send_response(404)
        self.send_header("content-type", "text/plain")
        self.end_headers()
        self.wfile.write(b"not found")

    def log_message(self, format, *args):
        print(format % args, flush=True)

class Server(socketserver.TCPServer):
    allow_reuse_address = True

Path("self-signed.crt").write_text('''"#,
        CERT_PEM,
        r#"''')
Path("self-signed.key").write_text('''"#,
        KEY_PEM,
        r#"''')

with Server(("127.0.0.1", port), Handler) as server:
    if os.environ.get("WORKER_RUNTIME_HOST_PROTOCOL", "http") == "https":
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain("self-signed.crt", "self-signed.key")
        server.socket = context.wrap_socket(server.socket, server_side=True)
    print(f"listening {port}", flush=True)
    server.serve_forever()
"#,
    ]
    .concat()
}
