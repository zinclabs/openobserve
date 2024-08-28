// sw.js

// Version identifier for cache and update management
const swURL = new URL(self.location.href);
const version = swURL.searchParams.get("version");
const isWebPath = swURL.searchParams.get("webpath");
const cacheVersion = `O2-cache-${version}`;
// Function to fetch the asset manifest

let pathPrefix = "/";
if (isWebPath == "true" || isWebPath == true) {
  pathPrefix = "/web/";
}
async function fetchManifest() {
  let response;
  if (pathPrefix == "/web/") {
    response = await fetch(`${pathPrefix}manifest.json`);
  } else {
    response = await fetch(`${pathPrefix}assets/manifest.json`);
  }
  return response.json();
}
self.addEventListener("install", function (event) {
  self.skipWaiting();
  event.waitUntil(
    (async () => {
      // You can manually provide a URL here to set the pathPrefix
      // adjustPathPrefix(self.location.href); // or use a default URL
      const manifest = await fetchManifest();

      // List of files to cache
      const filesToCache = [];

      Object.keys(manifest).forEach((key) => {
        if (key == "index.html") {
          filesToCache.push(`${pathPrefix}`);
          filesToCache.push(`${pathPrefix}favicon.ico`);
          filesToCache.push(`${pathPrefix}${manifest[key]["file"]}`);
          filesToCache.push(`${pathPrefix}sw.js`);
          // filesToCache.push("/web/");
          // filesToCache.push("/web/favicon.ico");
          // filesToCache.push(`/web/${manifest[key]["file"]}`);
          // filesToCache.push("/web/sw.js");
        }
        if (
          typeof manifest[key] == "object" &&
          manifest[key]?.file &&
          manifest[key]?.file.indexOf(".js") > -1
        ) {
          filesToCache.push(`${pathPrefix}${manifest[key]["file"]}`);
          filesToCache.push(`${pathPrefix}sw.js`);
          // filesToCache.push(`/web/${manifest[key]["file"]}`);
          // filesToCache.push(`/web/sw.js`);
        }
        if (
          typeof manifest[key] == "object" &&
          manifest[key]?.file &&
          manifest[key]?.file.indexOf(".js") > -1
        ) {
          if (pathPrefix == "/web/") {
            filesToCache.push(`${pathPrefix}assets/${manifest[key]["file"]}`);
          } else {
            filesToCache.push(`${pathPrefix}${manifest[key]["file"]}`);
          }
          // filesToCache.push(`/web/assets/${manifest[key]["file"]}`);
        }
      });

      // Open the cache
      const cache = await caches.open(cacheVersion);

      // Fetch and cache the files
      await Promise.all(
        filesToCache.map(async (file) => {
          try {
            const response = await fetch(file);
            if (!response.ok) {
              self.client.postMessage("staledata");
              throw new Error(
                `Request for ${file} failed with status ${response.status}`,
              );
            }

            await cache.put(file, response.clone());
          } catch (error) {
            console.error(`Failed to cache ${file}:`, error);
          }
        }),
      );
    })(),
  );
});

self.addEventListener("activate", function (event) {
  event.waitUntil(
    caches
      .keys()
      .then(function (cacheNames) {
        return Promise.all(
          cacheNames
            .filter(function (cacheName) {
              // Check if cacheName starts with the cacheVersion or contains it as part of the name
              return !cacheName.startsWith(cacheVersion);
            })
            .map(function (cacheName) {
              return caches.delete(cacheName);
            }),
        );
      })
      .then(function () {
        // Claim clients immediately for the updated service worker
        return self.clients.claim();
      }),
  );
});

self.addEventListener("fetch", function (event) {
  event.respondWith(
    caches
      .open(cacheVersion)
      .then(function (cache) {
        return cache.match(event.request);
      })
      .then(function (response) {
        if (response) {
          return response.clone();
        }

        var fetchRequest = event.request;
        return fetch(fetchRequest)
          .then(function (response) {
            if (
              !response ||
              response.status !== 200 ||
              response.type !== "basic"
            ) {
              let staleFlag = false;
              self.clients.matchAll().then((clients) => {
                clients.forEach((client) => {
                  if (event.request.url.endsWith(".js")) {
                    staleFlag = true;
                    if (staleFlag) {
                      // self.skipWaiting();
                      // caches.delete("cache-name");
                      client.postMessage("staledata");
                    }
                  }
                });
              });
              return response;
            }
            // if (event.request.method === "POST") {
            //   // Do not cache POST requests
            //   event.respondWith(
            //     fetch(event.request).catch(function (error) {
            //       throw error;
            //     }),
            //   );
            //   return;
            // }
            if (event.request.url.endsWith(".js")) {
              var responseToCache = response.clone();
              caches
                .open(cacheVersion)
                .then(function (cache) {
                  cache
                    .put(event.request, responseToCache)
                    .catch(function (error) {
                      console.error("Cache put failed:", error);
                    });
                })
                .catch(function (error) {
                  console.error("Cache open failed:", error);
                });
            }
            var responseToCache = response.clone();
            caches
              .open(cacheVersion)
              .then(function (cache) {
                cache
                  .put(event.request, responseToCache)
                  .catch(function (error) {
                    console.error("Cache put failed:", error);
                  });
                });
                return response;
              }
              if (event.request.method === 'POST') {
                // Do not cache POST requests
                event.respondWith(
                  fetch(event.request).catch(function(error) {
                    throw error;
                  })
                );
                return;
              }
              var responseToCache = response.clone();
              caches
                .open(cacheVersion)
                .then(function (cache) {
                  cache
                    .put(event.request, responseToCache)
                    .catch(function (error) {
                      console.error("Cache put failed:", error);
                    });
                })
                .catch(function (error) {
                  console.error("Cache open failed:", error);
                });
              return response;
            })
            .catch(function (error) {
              console.error("Fetch failed:", error);
              throw error;
            });
        })
        .catch(function (error) {
          console.error("Caches match failed:", error);
          throw error;
        }),
      );
    })(),
  );
});

self.addEventListener("activate", function (event) {
  event.waitUntil(
    caches
      .keys()
      .then(function (cacheNames) {
        return Promise.all(
          cacheNames
            .filter(function (cacheName) {
              // Check if cacheName starts with the cacheVersion or contains it as part of the name
              return !cacheName.startsWith(cacheVersion);
            })
            .map(function (cacheName) {
              return caches.delete(cacheName);
            }),
        );
      })
      .then(function () {
        // Claim clients immediately for the updated service worker
        return self.clients.claim();
      }),
  );
});

self.addEventListener("fetch", function (event) {
  event.respondWith(
    caches
      .open(cacheVersion)
      .then(function (cache) {
        return cache.match(event.request);
      })
      .then(function (response) {
        if (response) {
          return response.clone();
        }

        var fetchRequest = event.request;
        return fetch(fetchRequest)
          .then(function (response) {
            if (
              !response ||
              response.status !== 200 ||
              response.type !== "basic"
            ) {
              let staleFlag = false;
              self.clients.matchAll().then((clients) => {
                clients.forEach((client) => {
                  if (event.request.url.endsWith(".js")) {
                    staleFlag = true;
                    if (staleFlag) {
                      // self.skipWaiting();
                      // caches.delete("cache-name");
                      client.postMessage("staledata");
                    }
                  }
                });
              });
              return response;
            }
            if (event.request.method === "POST") {
              // Do not cache POST requests
              event.respondWith(
                fetch(event.request).catch(function (error) {
                  throw error;
                }),
              );
              return;
            }
            var responseToCache = response.clone();
            caches
              .open(cacheVersion)
              .then(function (cache) {
                cache
                  .put(event.request, responseToCache)
                  .catch(function (error) {
                    console.error("Cache put failed:", error);
                  });
              })
              .catch(function (error) {
                console.error("Cache open failed:", error);
              });
            return response;
          })
          .catch(function (error) {
            console.error("Fetch failed:", error);
            throw error;
          });
      })
      .catch(function (error) {
        console.error("Caches match failed:", error);
        throw error;
      }),
  );
});

self.addEventListener("message", function (event) {
  if (event.data === "skipWaiting") {
    caches.delete(cacheVersion);
    self.skipWaiting();
  }
});
