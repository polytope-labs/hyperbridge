# 2026-08-18 — The signer requirement moved from `validateConfig` to boot

Chosen: `validateConfig` neither requires the `[simplex.signer]` block nor looks at it — the block is not part of the config type it validates. A present block is validated where it is consumed: `signerFromToml` on the binary path, the wizard's write step, and the setup API's gate (which also mirrors run's watch-only exemption for an absent block). `bootFiller` rejects a missing `options.signer` unless every resolved chain is watch-only, and the CLI keeps its own `[simplex.signer]`-worded check so a file-driven run still fails at parse time with a file-oriented message.

Alternative considered: keeping the check in `validateConfig` and passing a "a signer was supplied" flag through it.

Why: `validateConfig` is exported for consumers to gate a config before starting, and boot calls the same function — leaving the rule there would have made every library consumer's valid config throw, since the signer is no longer in it. Threading a flag through would keep a config validator asking about an argument that is not config. The duplicated CLI check is deliberate: it costs three lines and preserves the error the binary's users already know.
