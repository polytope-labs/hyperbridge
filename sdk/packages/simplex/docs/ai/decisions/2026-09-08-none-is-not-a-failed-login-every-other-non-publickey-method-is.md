# 2026-09-08 — `none` is not a failed login, every other non-publickey method is

Routing all non-publickey attempts through the failure counter was the review's suggestion, and
it is right for password and keyboard-interactive. It is wrong for `none`: that is the probe
every SSH client opens with to ask which methods the server accepts, so counting it would spend
one of three per-connection failures on the handshake itself and, worse, would push a device that
reconnects ten times in ten minutes past the per-source limit and lock it out. `none` is refused
without being counted; everything else counts.
