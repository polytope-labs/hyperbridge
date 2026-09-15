# Simplex desktop security model (#1242)

The Electron shell now treats `simplex://local` as its only renderer origin. Its
window is sandboxed with context isolation and web security, without Node,
preload, IPC, or webview access. A desktop-only CSP limits scripts and API
connections to that origin, permissions default to denied, external navigation
is blocked, and only the SPA's manifest-backed HTTPS destinations can open in
the system browser. Packaged builds disable DevTools.

Desktop launches remain socket or named-pipe only, including when the outbound
tunnel is enabled. First-run configuration continues to use the CLI's atomic
writer and Unix mode `0600`; Electron E2E coverage verifies the warning, masked
configuration response, absence of key material elsewhere in the profile, and
that an untrusted page cannot read or mutate the custom-protocol API. The
documented trust boundary remains the operating-system user, with the weaker
Windows named-pipe and shared-machine limitations called out explicitly.
