use bityzba::requires;

pub const RELEASE_SERVICE_WORKER_TEMPLATE: &str = r#"const CACHE_VERSION = __CACHE_VERSION_JSON__;
const STATIC_CACHE_NAME = `jbotci-static-${CACHE_VERSION}`;
const RUNTIME_CACHE_NAME = `jbotci-runtime-${CACHE_VERSION}`;
const CURRENT_CACHE_NAMES = new Set([STATIC_CACHE_NAME, RUNTIME_CACHE_NAME]);
const PRECACHE_PATHS = __PRECACHE_PATHS_JSON__;

const SCOPE_URL = new URL(self.registration.scope);
if (!SCOPE_URL.pathname.endsWith("/")) {
  SCOPE_URL.pathname = `${SCOPE_URL.pathname}/`;
}
const APP_SHELL_URL = new URL("index.html", SCOPE_URL).href;
const PRECACHE_URLS = new Set(
  PRECACHE_PATHS.map((path) => new URL(path, SCOPE_URL).href),
);

self.addEventListener("install", (event) => {
  event.waitUntil((async () => {
    const cache = await caches.open(STATIC_CACHE_NAME);
    await cache.addAll(
      PRECACHE_PATHS.map((path) => new Request(new URL(path, SCOPE_URL), {
        cache: "default",
      })),
    );
    await self.skipWaiting();
  })());
});

self.addEventListener("activate", (event) => {
  event.waitUntil((async () => {
    const cacheNames = await caches.keys();
    await Promise.all(cacheNames.map((name) => {
      if (name.startsWith("jbotci-") && !CURRENT_CACHE_NAMES.has(name)) {
        return caches.delete(name);
      }
      return Promise.resolve(false);
    }));
    await self.clients.claim();
  })());
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET") {
    return;
  }

  const url = new URL(request.url);
  if (url.origin !== self.location.origin) {
    return;
  }

  const relativePath = relativeScopedPath(url);
  if (relativePath === null) {
    return;
  }

  if (isApiRequest(relativePath)) {
    event.respondWith(networkOnlyJson(request));
    return;
  }

  if (isEmbeddingAssetRequest(relativePath)) {
    return;
  }

  if (request.mode === "navigate") {
    event.respondWith(networkFirst(request, RUNTIME_CACHE_NAME, APP_SHELL_URL));
    return;
  }

  if (PRECACHE_URLS.has(url.href)) {
    event.respondWith(networkFirst(request, STATIC_CACHE_NAME, null));
    return;
  }

  if (isStaticOrCoreRequest(relativePath)) {
    event.respondWith(networkFirst(request, RUNTIME_CACHE_NAME, null));
  }
});

function relativeScopedPath(url) {
  if (!url.pathname.startsWith(SCOPE_URL.pathname)) {
    return null;
  }
  return url.pathname.slice(SCOPE_URL.pathname.length);
}

function isApiRequest(relativePath) {
  return relativePath === "api" || relativePath.startsWith("api/");
}

function isEmbeddingAssetRequest(relativePath) {
  return relativePath.startsWith("assets/embeddings/");
}

function isStaticOrCoreRequest(relativePath) {
  return relativePath === ""
    || relativePath === "index.html"
    || relativePath === "manifest.webmanifest"
    || relativePath === "service-worker.js"
    || relativePath.startsWith("assets/");
}

async function networkFirst(request, cacheName, fallbackUrl) {
  const cache = await caches.open(cacheName);
  try {
    const response = await fetch(request);
    if (response.ok && response.type !== "opaque") {
      await cache.put(request, response.clone());
    }
    return response;
  } catch (error) {
    const cached = await caches.match(request);
    if (cached) {
      return cached;
    }
    if (fallbackUrl !== null) {
      const fallback = await caches.match(fallbackUrl);
      if (fallback) {
        return fallback;
      }
    }
    return offlineTextResponse();
  }
}

async function networkOnlyJson(request) {
  try {
    return await fetch(request);
  } catch (error) {
    return new Response(JSON.stringify({
      error: "offline",
      message: "jbotci is offline and this API request is not cached.",
    }), {
      status: 503,
      headers: {
        "Content-Type": "application/json; charset=utf-8",
      },
    });
  }
}

function offlineTextResponse() {
  return new Response("jbotci is offline and this resource is not cached.", {
    status: 503,
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
    },
  });
}
"#;

#[requires(!cache_version.is_empty())]
#[requires(precache_paths.iter().all(|path| !path.is_empty() && !path.starts_with('/')))]
#[bityzba::ensures(ret.as_ref().is_ok_and(|script| script.contains(cache_version)) || ret.is_err())]
pub fn render_release_service_worker(
    cache_version: &str,
    precache_paths: &[String],
) -> Result<String, serde_json::Error> {
    let cache_version_json = serde_json::to_string(cache_version)?;
    let precache_paths_json = serde_json::to_string(precache_paths)?;
    Ok(RELEASE_SERVICE_WORKER_TEMPLATE
        .replace("__CACHE_VERSION_JSON__", &cache_version_json)
        .replace("__PRECACHE_PATHS_JSON__", &precache_paths_json))
}
