# 2026-09-09 — Windows named pipes are NOT equivalent to a 0600 socket (verified)

Finding, verified rather than assumed, because the security argument for this listen mode rests on it.

`libuv/src/win/pipe.c` `pipe_alloc_accept` makes the pipe with
`CreateNamedPipeW(..., PIPE_UNLIMITED_INSTANCES, 65536, 65536, 0, NULL)` — the final
`lpSecurityAttributes` is `NULL`, and there is no libuv or Node API to supply one. Microsoft documents
what `NULL` means, verbatim: "the named pipe gets a default security descriptor ... The ACLs in the
default security descriptor for a named pipe grant full control to the LocalSystem account,
administrators, and the creator owner. They also grant read access to members of the Everyone group and
the anonymous account."

So the pipe is **not owner-only**. Everyone and anonymous get read access, and Administrators get full
control, in a machine-global namespace (`\\.\pipe\`) rather than a directory the owner controls.

Practically, driving the HTTP API needs write access to send a request, which Everyone does not get, so
a non-administrator local user cannot reach `/api/send` over the pipe. Administrators can — but an
administrator already reads `filler-config.toml` and takes the signing key without touching any API, so
that is not a boundary this could have held. libuv also passes `FILE_FLAG_FIRST_PIPE_INSTANCE`, so a
hostile process cannot pre-create the name and impersonate the daemon; our bind fails loudly instead.

The gap cannot be closed from Node. The only pipe-ACL control Node surfaces is `listen({ readableAll,
writableAll })`, which reaches `uv_pipe_chmod`; that sets an ACE for the Everyone SID and Node invokes
it only when one of those flags is set, so the single knob available *widens* access and none narrows
it. `restrictSocketToOwner` therefore returns early on win32 rather than pretending.

Stated plainly so it is not papered over: on Unix the kernel is the access control; on Windows the
claim is weaker — read-open by any local user, full control for administrators — and an embedder that
needs owner-only on Windows must supply the pipe from native code, not from Node.
