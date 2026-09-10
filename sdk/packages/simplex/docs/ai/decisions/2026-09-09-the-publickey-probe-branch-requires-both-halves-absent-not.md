# 2026-09-09 — The publickey probe branch requires both halves absent, not either

Decided: `ctx.signature === undefined && ctx.blob === undefined` selects the unsigned-probe path;
anything with exactly one of the two is refused as malformed. Previously the test was `||`, so a
half-populated request reached `ctx.accept()` — which authenticates rather than sending PK_OK once a
signature is present.

Why change something not currently reachable: `ssh2@1.17.0` always populates both fields together,
so no wire input produces the dangerous shape today. But the safety of the guard came entirely from
that internal parser detail, not from anything in this file, and the version is a floating `^1.17.0`.
The bypass fixed earlier the same day had the identical shape — correct-looking code whose safety
depended on an undocumented ssh2 behaviour (`verify()` returning a boolean) that turned out not to
hold. Making the fallback refusal costs nothing and removes the dependency on that invariant.

Rejected: asserting the invariant instead (e.g. throwing if ssh2 ever hands over one without the
other). It is the same branch either way, and refusing an unverifiable request is the behaviour we
actually want; a thrown error inside the authentication handler would be a less predictable path
through ssh2 than a plain rejection.

Rejected: pinning ssh2 to an exact version to preserve the invariant. Version pinning is worth doing
on its own merits, but it protects this guard only until the next deliberate upgrade, and it does
not make the authentication path correct in isolation.
