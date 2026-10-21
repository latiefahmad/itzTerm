# syntax=docker/dockerfile:1

# The runtime base matches the release runner (ubuntu-22.04) that builds the
# itzterm-server assets, so the binary loads the same library versions it was
# linked against.
ARG UBUNTU_VERSION=22.04

# 22.04 has no minisign package, so the download stage uses 24.04.
FROM ubuntu:24.04 AS fetch
ARG TARGETARCH
# A GitHub release tag (v0.5.0, nightly), or "latest" for the newest stable release.
ARG ITZTERM_RELEASE=latest
RUN apt-get update \
	&& apt-get install -y --no-install-recommends ca-certificates curl minisign \
	&& rm -rf /var/lib/apt/lists/*
COPY src-tauri/tauri.conf.json /tmp/tauri.conf.json
WORKDIR /fetch
RUN set -eu; \
	case "$TARGETARCH" in \
		amd64) asset=itzterm-server ;; \
		arm64) asset=itzterm-server-linux-aarch64 ;; \
		*) echo "unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
	esac; \
	if [ "$ITZTERM_RELEASE" = latest ]; then \
		base=https://github.com/latiefahmad/itzTerm/releases/latest/download; \
	else \
		base="https://github.com/latiefahmad/itzTerm/releases/download/$ITZTERM_RELEASE"; \
	fi; \
	curl -fsSL --proto '=https' --retry 3 -o itzterm-server "$base/$asset"; \
	curl -fsSL --proto '=https' --retry 3 -o itzterm-server.sig "$base/$asset.sig"; \
	sed -n 's/.*"pubkey": *"\([^"]*\)".*/\1/p' /tmp/tauri.conf.json | base64 -d > minisign.pub; \
	base64 -d itzterm-server.sig > itzterm-server.minisig; \
	minisign -V -m itzterm-server -x itzterm-server.minisig -p minisign.pub; \
	chmod 0755 itzterm-server

FROM ubuntu:${UBUNTU_VERSION}
ARG ITZTERM_RELEASE=latest
ARG ITZTERM_UID=1000
ARG ITZTERM_GID=1000

LABEL org.opencontainers.image.title="itzterm-server" \
	org.opencontainers.image.description="Standalone itzTerm server: browser terminals, git, and ACP agents over HTTP and WebSocket" \
	org.opencontainers.image.source="https://github.com/latiefahmad/itzTerm" \
	org.opencontainers.image.licenses="MIT" \
	org.opencontainers.image.version="${ITZTERM_RELEASE}"

# itzterm-server links WebKitGTK and GTK through the shared Tauri crate even
# though the server never opens a window. The ldd check below fails the build
# if a linked library is missing.
RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
		bash \
		ca-certificates \
		curl \
		git \
		less \
		libwebkit2gtk-4.1-0 \
		openssh-client \
		procps \
		tini \
	&& rm -rf /var/lib/apt/lists/* \
	&& groupadd --non-unique --gid "$ITZTERM_GID" itzterm \
	&& useradd --non-unique --uid "$ITZTERM_UID" --gid itzterm --create-home --shell /bin/bash itzterm \
	&& mkdir -p /home/itzterm/.local/bin /home/itzterm/.local/state/itzterm /workspace \
	&& chown -R itzterm:itzterm /home/itzterm /workspace

COPY --from=fetch /fetch/itzterm-server /usr/local/bin/itzterm-server
RUN if ldd /usr/local/bin/itzterm-server | grep 'not found'; then exit 1; fi

ENV HOME=/home/itzterm \
	PATH=/home/itzterm/.local/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
	LANG=C.UTF-8 \
	ITZTERM_SERVER_UPDATE_ENABLED=false

USER itzterm
WORKDIR /workspace
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
	CMD curl -fsS http://127.0.0.1:8080/health > /dev/null || exit 1

# A public bind with no ITZTERM_WEB_AUTH_TOKEN makes the server generate a token
# into <state dir>/web-auth-token and refuse unauthenticated clients.
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/itzterm-server"]
CMD ["--host", "0.0.0.0", "--port", "8080", "--state-dir", "/home/itzterm/.local/state/itzterm"]
