# 2026-09-07 — Remote access terminates the phone's SSH session inside simplex, not sshd

The phone's SSH session ends in an `ssh2` server embedded in the process, fed by the relay's
forwarded channels through `injectSocket`, so no port is opened and the host's sshd is never
exposed. Alternatives: exposing sshd (off by default on macOS/Windows, and a full shell on the
operator's box for whoever holds the key) or having the relay terminate the session (then the
relay sees the UI traffic, which moves funds). The embedded server accepts only paired keys and
`direct-tcpip` to the UI bind; the UI's loopback binding and Host guard stay as they are.
