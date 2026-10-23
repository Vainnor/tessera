# The website image: the static Astro build served by Caddy on :8080 as an unprivileged user.
# TLS terminates at the reverse proxy in front of it. Build from the repo root:
#   docker build -f apps/web/Dockerfile .

# The output is static files, so the build stage runs natively and only the runtime stage is
# per-platform.
FROM --platform=$BUILDPLATFORM node:24.22.0-alpine@sha256:44fce696f38ba8c987c4db89954051000bbd05ffd8d8659ba15ed4a8602e937f AS build
WORKDIR /src
# corepack installs the pnpm version pinned in package.json's packageManager.
RUN corepack enable
COPY . .
RUN pnpm install --frozen-lockfile --filter "@vainnor/tessera-web..." \
    && pnpm --filter "@vainnor/tessera-web..." build

# The upstream binary carries cap_net_bind_service for ports below 1024, which makes exec fail
# under no-new-privileges. Port 8080 needs no capability, so take a copy without the attribute.
FROM caddy:2.11.7-alpine@sha256:eb4d35b7452d1ab6171fbdbe8d9275b2301dad6dad92115d91e19fb9b24d510f AS caddy
RUN cp /usr/bin/caddy /caddy

# The same Alpine release the Caddy image is built on; keeps busybox wget for the healthcheck.
FROM alpine:3.24.0@sha256:a2d49ea686c2adfe3c992e47dc3b5e7fa6e6b5055609400dc2acaeb241c829f4
# Caddy keeps nothing on disk with this config, but still resolves its data and config folders
# at startup; point them at /tmp so the root filesystem can be mounted read-only.
ENV XDG_DATA_HOME=/tmp/caddy/data XDG_CONFIG_HOME=/tmp/caddy/config
COPY --from=caddy /caddy /usr/bin/caddy
COPY apps/web/Caddyfile /etc/caddy/Caddyfile
COPY --from=build /src/apps/web/dist /srv
USER 65532:65532
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s CMD wget -q --spider http://127.0.0.1:8080/ || exit 1
CMD ["caddy", "run", "--config", "/etc/caddy/Caddyfile", "--adapter", "caddyfile"]
