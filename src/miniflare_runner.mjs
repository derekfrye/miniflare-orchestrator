import fs from "node:fs";
import http from "node:http";
import https from "node:https";
import path from "node:path";
import { Readable } from "node:stream";

const MINIFLARE_MODULE_CANDIDATES = [
  process.env.WORKER_RUNTIME_HOST_MINIFLARE_MODULE,
  "/usr/local/lib/node_modules/wrangler/node_modules/miniflare/dist/src/index.js",
  "/usr/local/lib/node_modules/miniflare/dist/src/index.js",
].filter(Boolean);

// Localhost test/dev certificate material only. Keep this aligned with
// Miniflare's local-dev default so existing test clients continue to
// trust/ignore the same self-signed cert. Users may replace this key/cert pair
// with their own self-signed localhost certificate, or keep using this one.
const HTTPS_KEY = `
-----BEGIN EC PRIVATE KEY-----
MHcCAQEEIC+umAaVUbEfPqGA9M7b5zAP7tN2eLT1bu8U8gpbaKbsoAoGCCqGSM49
AwEHoUQDQgAEtrIEgzogjrUHIvB4qgjg/cT7blhWuLUfSUp6H62NCo21NrVWgPtC
mCWw+vbGTBwIr/9X1S4UL1/f3zDICC7YSA==
-----END EC PRIVATE KEY-----
`;

const HTTPS_CERT = `
-----BEGIN CERTIFICATE-----
MIIDBzCCAq2gAwIBAgIUaEibZTawMcz6xQ/0rGNlEBKwkUowCgYIKoZIzj0EAwIw
gZExCzAJBgNVBAYTAlVTMQ4wDAYDVQQIDAVUZXhhczEPMA0GA1UEBwwGQXVzdGlu
MRMwEQYDVQQKDApDbG91ZGZsYXJlMRAwDgYDVQQLDAdXb3JrZXJzMRIwEAYDVQQD
DAlsb2NhbGhvc3QxJjAkBgkqhkiG9w0BCQEWF3dyYW5nbGVyQGNsb3VkZmxhcmUu
Y29tMCAXDTI1MTAwMjEzMzQ1MloYDzIxMjUwOTA4MTMzNDUyWjCBkTELMAkGA1UE
BhMCVVMxDjAMBgNVBAgMBVRleGFzMQ8wDQYDVQQHDAZBdXN0aW4xEzARBgNVBAoM
CkNsb3VkZmxhcmUxEDAOBgNVBAsMB1dvcmtlcnMxEjAQBgNVBAMMCWxvY2FsaG9z
dDEmMCQGCSqGSIb3DQEJARYXd3JhbmdsZXJAY2xvdWRmbGFyZS5jb20wWTATBgcq
hkjOPQIBBggqhkjOPQMBBwNCAAS2sgSDOiCOtQci8HiqCOD9xPtuWFa4tR9JSnof
rY0KjbU2tVaA+0KYJbD69sZMHAiv/1fVLhQvX9/fMMgILthIo4HeMIHbMB0GA1Ud
DgQWBBRJdqFOSyLTRzoqFQQIchjgUtbtKjAfBgNVHSMEGDAWgBRJdqFOSyLTRzoq
FQQIchjgUtbtKjAPBgNVHRMBAf8EBTADAQH/MCwGCWCGSAGG+EIBDQQfFh1PcGVu
U1NMIEdlbmVyYXRlZCBDZXJ0aWZpY2F0ZTALBgNVHQ8EBAMCAvQwMQYDVR0lBCow
KAYIKwYBBQUHAwEGCCsGAQUFBwMCBggrBgEFBQcDAwYIKwYBBQUHAwgwGgYDVR0R
BBMwEYIJbG9jYWxob3N0hwR/AAABMAoGCCqGSM49BAMCA0gAMEUCIQDNxEiZc6Q6
8hK0q3y/9lDWc+dHr74gAnBHVJZEo5uyRQIgW6eL31hH7qouqUi9+efWU1N85n0z
X3kip4YDAFo8ozE=
-----END CERTIFICATE-----
`;

const KNOWN_ENV = new Set([
  "HOME",
  "PATH",
  "TMPDIR",
  "WORKER_RUNTIME_HOST_BACKEND",
  "WORKER_RUNTIME_HOST_CONFIG_FILE",
  "WORKER_RUNTIME_HOST_ENV",
  "WORKER_RUNTIME_HOST_INSPECTOR_PORT",
  "WORKER_RUNTIME_HOST_LOG_DIR",
  "WORKER_RUNTIME_HOST_LOG_LEVEL",
  "WORKER_RUNTIME_HOST_MINIFLARE_MODULE",
  "WORKER_RUNTIME_HOST_MINIFLARE_VERBOSE",
  "WORKER_RUNTIME_HOST_MINIFLARE_WORKERD_CONFIG_DEBUG",
  "WORKER_RUNTIME_HOST_MINIFLARE_DISABLE_INSPECTOR",
  "WORKER_RUNTIME_HOST_NODE_BIN",
  "WORKER_RUNTIME_HOST_PORT",
  "WORKER_RUNTIME_HOST_PROTOCOL",
  "WORKER_RUNTIME_HOST_RUNTIME_DIR",
  "WORKER_RUNTIME_HOST_STATE_DIR",
  "WORKER_RUNTIME_HOST_WRANGLER_BIN",
  "MINIFLARE_WORKERD_CONFIG_DEBUG",
  "MINIFLARE_WORKERD_PATH",
]);

const config = {
  runtimeDir: requiredEnv("WORKER_RUNTIME_HOST_RUNTIME_DIR"),
  stateDir: requiredEnv("WORKER_RUNTIME_HOST_STATE_DIR"),
  logDir: requiredEnv("WORKER_RUNTIME_HOST_LOG_DIR"),
  configFile: requiredEnv("WORKER_RUNTIME_HOST_CONFIG_FILE"),
  envName: process.env.WORKER_RUNTIME_HOST_ENV || "dev",
  port: Number(requiredEnv("WORKER_RUNTIME_HOST_PORT")),
  inspectorPort: Number(process.env.WORKER_RUNTIME_HOST_INSPECTOR_PORT || "9229"),
  protocol: process.env.WORKER_RUNTIME_HOST_PROTOCOL || "http",
  logLevel: process.env.WORKER_RUNTIME_HOST_LOG_LEVEL || "warn",
  miniflareVerbose: envFlag("WORKER_RUNTIME_HOST_MINIFLARE_VERBOSE"),
  workerdConfigDebug: envFlag("WORKER_RUNTIME_HOST_MINIFLARE_WORKERD_CONFIG_DEBUG"),
  disableInspector: envFlag("WORKER_RUNTIME_HOST_MINIFLARE_DISABLE_INSPECTOR"),
};
const startupStartedAt = Date.now();

if (!Number.isInteger(config.port) || config.port <= 0) {
  throw new Error(`invalid worker port: ${process.env.WORKER_RUNTIME_HOST_PORT}`);
}
if (!Number.isInteger(config.inspectorPort) || config.inspectorPort <= 0) {
  throw new Error(`invalid inspector port: ${process.env.WORKER_RUNTIME_HOST_INSPECTOR_PORT}`);
}

const { Miniflare, candidate: miniflareModule } = await importMiniflare();
configureWorkerdDebug();
const wrangler = parseWranglerToml(fs.readFileSync(config.configFile, "utf8"), config.envName);
const bindings = { ...wrangler.vars, ...processBindings() };
const scriptPath = path.resolve(config.runtimeDir, wrangler.main || "worker_entry.mjs");
const assets = wrangler.assets?.directory
  ? {
      directory: path.resolve(config.runtimeDir, wrangler.assets.directory),
      binding: wrangler.assets.binding,
      routerConfig: {
        has_user_worker: true,
        invoke_user_worker_ahead_of_assets: true,
      },
    }
  : undefined;
const effectiveConfig = {
  backend: "miniflare",
  env: config.envName,
  scriptPath,
  compatibilityDate: wrangler.compatibilityDate,
  compatibilityFlags: wrangler.compatibilityFlags,
  bindings: {
    vars: Object.keys(bindings).sort(),
    kvNamespaces: Object.keys(wrangler.kvNamespaces).sort(),
    r2Buckets: Object.keys(wrangler.r2Buckets).sort(),
    durableObjects: Object.keys(wrangler.durableObjects).sort(),
    assets: assets?.binding || null,
  },
  persist: {
    kv: true,
    r2: true,
    durableObjects: true,
    cache: true,
    d1: true,
  },
  miniflare: {
    module: miniflareModule,
    verbose: config.miniflareVerbose,
    disableInspector: config.disableInspector,
    workerdPath: process.env.MINIFLARE_WORKERD_PATH || null,
    workerdConfigDebug: process.env.MINIFLARE_WORKERD_CONFIG_DEBUG || null,
  },
};
console.log(`worker-runtime-host-miniflare: effective config ${JSON.stringify(effectiveConfig)}`);

trace("constructing Miniflare instance");
const mf = new Miniflare({
  rootPath: config.runtimeDir,
  scriptPath,
  modules: true,
  modulesRoot: config.runtimeDir,
  modulesRules: [
    { type: "CompiledWasm", include: ["**/*.wasm"] },
    { type: "Data", include: ["**/*.bin"] },
    { type: "Text", include: ["**/*.txt"] },
  ],
  compatibilityDate: wrangler.compatibilityDate,
  compatibilityFlags: wrangler.compatibilityFlags,
  bindings,
  kvNamespaces: wrangler.kvNamespaces,
  r2Buckets: wrangler.r2Buckets,
  durableObjects: wrangler.durableObjects,
  assets,
  defaultPersistRoot: config.stateDir,
  kvPersist: true,
  r2Persist: true,
  durableObjectsPersist: true,
  cachePersist: true,
  d1Persist: true,
  host: "127.0.0.1",
  port: 0,
  ...(config.disableInspector ? {} : { inspectorPort: config.inspectorPort }),
  https: false,
  verbose: config.miniflareVerbose,
});
trace("Miniflare instance constructed");

const internalUrl = await awaitWithPendingTrace("mf.ready", mf.ready);
trace(`mf.ready resolved to ${internalUrl.href}`);
const publicServer = await awaitWithPendingTrace(
  "listenPublicServer",
  listenPublicServer(mf),
);
trace(`public server listening on ${config.protocol}://0.0.0.0:${config.port}`);
console.log(
  `worker-runtime-host-miniflare: ready on ${config.protocol}://0.0.0.0:${config.port} via ${internalUrl.href}`,
);

let stopping = false;
async function stop(signal) {
  if (stopping) return;
  stopping = true;
  console.log(`worker-runtime-host-miniflare: received ${signal}, shutting down`);
  await closeServer(publicServer);
  await mf.dispose();
  process.exit(0);
}

process.on("SIGTERM", () => void stop("SIGTERM"));
process.on("SIGINT", () => void stop("SIGINT"));
await new Promise(() => {});

async function importMiniflare() {
  const errors = [];
  for (const candidate of MINIFLARE_MODULE_CANDIDATES) {
    try {
      trace(`importing Miniflare from ${candidate}`);
      const module = await import(candidate);
      trace(`imported Miniflare from ${candidate}`);
      return { ...module, candidate };
    } catch (error) {
      errors.push(`${candidate}: ${error.message}`);
    }
  }
  throw new Error(`unable to import Miniflare:\n${errors.join("\n")}`);
}

async function listenPublicServer(miniflare) {
  const server =
    config.protocol === "https"
      ? https.createServer({ key: HTTPS_KEY, cert: HTTPS_CERT }, (req, res) =>
          void dispatchRequest(miniflare, req, res),
        )
      : http.createServer((req, res) => void dispatchRequest(miniflare, req, res));
  server.keepAliveTimeout = 0;
  server.headersTimeout = 0;
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(config.port, "0.0.0.0", () => {
      server.off("error", reject);
      resolve();
    });
  });
  return server;
}

async function dispatchRequest(miniflare, req, res) {
  try {
    const [url, init] = requestFromIncoming(req);
    const workerResponse = await miniflare.dispatchFetch(url, init);
    res.statusCode = workerResponse.status;
    res.statusMessage = workerResponse.statusText;
    for (const [name, value] of workerResponse.headers) {
      if (name.toLowerCase() !== "connection") res.setHeader(name, value);
    }
    res.setHeader("Connection", "close");
    res.shouldKeepAlive = false;

    if (workerResponse.body) {
      Readable.fromWeb(workerResponse.body).pipe(res);
    } else {
      res.end();
    }
  } catch (error) {
    if (!res.headersSent) {
      res.writeHead(500, {
        "Content-Type": "text/plain; charset=utf-8",
        Connection: "close",
      });
    }
    res.end(error?.stack || String(error));
  }
}

function requestFromIncoming(req) {
  const host = req.headers.host || `127.0.0.1:${config.port}`;
  const url = `${config.protocol}://${host}${req.url || "/"}`;
  const headers = new Headers();
  for (const [name, value] of Object.entries(req.headers)) {
    if (value === undefined) continue;
    if (Array.isArray(value)) {
      for (const item of value) headers.append(name, item);
    } else {
      headers.set(name, value);
    }
  }
  const init = { method: req.method || "GET", headers, redirect: "manual" };
  if (req.method !== "GET" && req.method !== "HEAD") {
    init.body = Readable.toWeb(req);
    init.duplex = "half";
  }
  return [url, init];
}

async function closeServer(server) {
  await new Promise((resolve) => server.close(resolve));
}

function requiredEnv(name) {
  const value = process.env[name];
  if (!value) throw new Error(`missing required environment variable ${name}`);
  return value;
}

function envFlag(name) {
  const value = process.env[name];
  if (value === undefined || value === "") return false;
  return !["0", "false", "no", "off"].includes(value.toLowerCase());
}

function configureWorkerdDebug() {
  if (config.workerdConfigDebug && !process.env.MINIFLARE_WORKERD_CONFIG_DEBUG) {
    process.env.MINIFLARE_WORKERD_CONFIG_DEBUG = path.join(
      config.logDir,
      "workerd-config.capnp",
    );
  }
}

async function awaitWithPendingTrace(name, promise) {
  const checkpoints = [1000, 5000, 15000, 30000];
  let settled = false;
  const timers = checkpoints.map((delay) =>
    setTimeout(() => {
      if (!settled) {
        trace(`${name} still pending after ${delay}ms; ${activeHandleSummary()}`);
      }
    }, delay),
  );
  try {
    return await promise;
  } finally {
    settled = true;
    for (const timer of timers) clearTimeout(timer);
  }
}

function trace(message) {
  if (!config.miniflareVerbose && !config.workerdConfigDebug) return;
  const elapsedMs = Date.now() - startupStartedAt;
  console.log(
    `worker-runtime-host-miniflare[pid=${process.pid} +${elapsedMs}ms]: ${message}`,
  );
}

function activeHandleSummary() {
  const handles = typeof process._getActiveHandles === "function" ? process._getActiveHandles() : [];
  const requests =
    typeof process._getActiveRequests === "function" ? process._getActiveRequests() : [];
  const handleCounts = summarizeConstructors(handles);
  const requestCounts = summarizeConstructors(requests);
  return `active_handles=${JSON.stringify(handleCounts)} active_requests=${JSON.stringify(requestCounts)}`;
}

function summarizeConstructors(items) {
  const counts = {};
  for (const item of items) {
    const name = item?.constructor?.name || typeof item;
    counts[name] = (counts[name] || 0) + 1;
  }
  return counts;
}

function processBindings() {
  const bindings = {};
  for (const [name, value] of Object.entries(process.env)) {
    if (!KNOWN_ENV.has(name)) bindings[name] = value;
  }
  return bindings;
}

function parseWranglerToml(source, envName) {
  const result = {
    vars: {},
    kvNamespaces: {},
    r2Buckets: {},
    durableObjects: {},
    sqliteClasses: new Set(),
    compatibilityFlags: [],
  };
  let section = "";
  let arrayTable = "";
  let currentObject = null;
  let collecting = null;

  for (const rawLine of source.split(/\r?\n/)) {
    const line = stripComment(rawLine).trim();
    if (!line) continue;

    const arrayHeader = line.match(/^\[\[([^\]]+)]]$/);
    if (arrayHeader) {
      finishObject();
      section = "";
      arrayTable = arrayHeader[1].trim();
      currentObject = {};
      continue;
    }

    const header = line.match(/^\[([^\]]+)]$/);
    if (header) {
      finishObject();
      section = header[1].trim();
      arrayTable = "";
      continue;
    }

    if (collecting) {
      if (line === "]") {
        collecting = null;
        continue;
      }
      parseInlineObject(line, collecting);
      continue;
    }

    const keyValue = line.match(/^([A-Za-z0-9_.-]+)\s*=\s*(.+)$/);
    if (!keyValue) continue;

    const [, key, rawValue] = keyValue;
    if (rawValue === "[") {
      collecting = collectionFor(result, section, key, envName);
      continue;
    }

    const value = parseTomlValue(rawValue);
    if (currentObject && arrayTable) {
      currentObject[key] = value;
      continue;
    }
    applyScalar(result, section, key, value, envName);
  }
  finishObject();

  return {
    main: result.main,
    compatibilityDate: result.compatibilityDate,
    compatibilityFlags: result.compatibilityFlags,
    vars: result.vars,
    kvNamespaces: result.kvNamespaces,
    r2Buckets: result.r2Buckets,
    durableObjects: withSqliteFlags(result.durableObjects, result.sqliteClasses),
    assets: result.assets,
  };

  function finishObject() {
    if (!currentObject || !arrayTable) return;
    applyObject(result, arrayTable, currentObject, envName);
    currentObject = null;
  }
}

function collectionFor(result, section, key, envName) {
  if (key === "kv_namespaces" && (section === "" || section === `env.${envName}`)) {
    return (line) => {
      const object = parseTomlInlineObject(line);
      if (object.binding) result.kvNamespaces[object.binding] = object.id || object.binding;
    };
  }
  if (key === "r2_buckets" && (section === "" || section === `env.${envName}`)) {
    return (line) => applyR2Bucket(result, parseTomlInlineObject(line));
  }
  if (key === "bindings" && (section === "durable_objects" || section === `env.${envName}.durable_objects`)) {
    return (line) => applyDurableObject(result, parseTomlInlineObject(line));
  }
  return () => {};
}

function parseInlineObject(line, apply) {
  const normalized = line.replace(/,$/, "");
  if (normalized.startsWith("{") && normalized.endsWith("}")) apply(normalized);
}

function applyScalar(result, section, key, value, envName) {
  if (section === "") {
    if (key === "main") result.main = value;
    if (key === "compatibility_date") result.compatibilityDate = value;
    if (key === "compatibility_flags" && Array.isArray(value)) result.compatibilityFlags = value;
  }
  if (section === "vars" || section === `env.${envName}.vars`) result.vars[key] = value;
  if (section === "assets" || section === `env.${envName}.assets`) {
    result.assets ||= {};
    if (key === "directory") result.assets.directory = value;
    if (key === "binding") result.assets.binding = value;
  }
}

function applyObject(result, table, object, envName) {
  if (table === "durable_objects.bindings" || table === `env.${envName}.durable_objects.bindings`) {
    applyDurableObject(result, object);
  }
  if (table === "r2_buckets" || table === `env.${envName}.r2_buckets`) {
    applyR2Bucket(result, object);
  }
  if (table === "migrations" || table === `env.${envName}.migrations`) {
    for (const className of object.new_sqlite_classes || []) result.sqliteClasses.add(className);
  }
}

function applyR2Bucket(result, object) {
  const binding = object.binding;
  if (binding) result.r2Buckets[binding] = object.bucket_name || object.bucket || binding;
}

function applyDurableObject(result, object) {
  const name = object.name;
  const className = object.class_name || object.className;
  if (name && className) result.durableObjects[name] = { className };
}

function withSqliteFlags(durableObjects, sqliteClasses) {
  const result = {};
  for (const [name, value] of Object.entries(durableObjects)) {
    result[name] = sqliteClasses.has(value.className) ? { ...value, useSQLite: true } : value;
  }
  return result;
}

function parseTomlInlineObject(value) {
  const object = {};
  const body = value.trim().replace(/^\{/, "").replace(/}$/, "");
  for (const part of body.split(",")) {
    const keyValue = part.trim().match(/^([A-Za-z0-9_.-]+)\s*=\s*(.+)$/);
    if (keyValue) object[keyValue[1]] = parseTomlValue(keyValue[2]);
  }
  return object;
}

function parseTomlValue(value) {
  const trimmed = value.trim().replace(/,$/, "");
  if (trimmed.startsWith('"') && trimmed.endsWith('"')) {
    return trimmed.slice(1, -1).replace(/\\"/g, '"');
  }
  if (trimmed === "true") return true;
  if (trimmed === "false") return false;
  if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
    const body = trimmed.slice(1, -1).trim();
    if (!body) return [];
    return body.split(",").map((item) => parseTomlValue(item.trim()));
  }
  return trimmed;
}

function stripComment(line) {
  let inString = false;
  for (let index = 0; index < line.length; index += 1) {
    const char = line[index];
    if (char === '"' && line[index - 1] !== "\\") inString = !inString;
    if (char === "#" && !inString) return line.slice(0, index);
  }
  return line;
}
