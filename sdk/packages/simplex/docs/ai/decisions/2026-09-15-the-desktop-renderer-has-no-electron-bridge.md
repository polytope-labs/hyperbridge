# 2026-09-15 — The desktop renderer has no Electron bridge

Decided: the Simplex desktop renderer receives no preload or IPC API. It is the existing browser SPA,
served only at the standard, secure `simplex://local` origin and proxied to the solver's Unix socket
or Windows named pipe. Context isolation, Chromium sandboxing, and web security are explicit; Node
integration and webviews are disabled. Production builds also disable DevTools.

The main document receives a desktop-only Content Security Policy. Scripts, workers, assets, and API
connections are local-only; frames, objects, media, and non-local form submissions are disabled.
Inline styles remain allowed because React style properties, Sonner, and Radix insert them at runtime;
scripts still disallow inline execution and evaluation, while images, fonts, and connections cannot
reach outside hosts. The HTTP UI keeps its existing headers, so this policy does not unexpectedly
break CLI deployments with different embedding requirements.

Renderer navigation is restricted to the exact `simplex://local` authority. Every requested child
window is denied. Known HTTPS links to HyperFX and the explorers compiled into the UI are instead
opened by the operating system browser after exact-host validation; arbitrary HTTPS, credentials,
ports, local files, and active URL schemes are rejected. Browser permissions are denied except for
sanitized clipboard writes initiated by the live trusted renderer, which keeps the dashboard's copy
buttons functional without exposing clipboard reads.

The no-bridge design is intentional: the SPA needs no native capability, so even a minimal preload
would create an API surface without a consumer. The security boundary remains the main-process
protocol proxy. The desktop launch arguments always choose `--ui-socket` and never `--ui`, including
when the outbound remote-access tunnel is enabled, so configuration cannot introduce a local TCP
listener.

The remaining trust boundary is the local account. A `0600` Unix socket excludes other users but not
malware running as the same user; Node's Windows named-pipe API cannot express an equivalent portable
owner-only DACL. CLI loopback TCP is machine-local rather than user-local, and the plaintext signing
config must be protected separately. These limits are documented for operators rather than hidden
behind a stronger claim than the platform provides.
