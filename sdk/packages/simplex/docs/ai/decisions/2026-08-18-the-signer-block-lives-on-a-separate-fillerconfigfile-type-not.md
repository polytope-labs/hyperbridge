# 2026-08-18 — The signer block lives on a separate `FillerConfigFile` type, not on `SimplexConfig`

Chosen: `FillerTomlConfig` drops `simplex.signer`; a new `FillerConfigFile extends FillerTomlConfig` adds it back for the binary's file format. The CLI, setup API, TOML writer, wizard state and `UiServer`'s operator context are typed with the file shape; the library never is.

Alternatives considered: keeping the optional field on the shared type and documenting that the library ignores it; a structural `Omit<>` on `SimplexConfig` so a file object is a type error at `Simplex.start`.

Why: leaving the field on the library's type advertises an input the library refuses to read — the one thing a config field must never do. The `Omit` variant goes too far the other way: the binary hands the parsed file straight to `Simplex.start`, and it must, because `UiServer.persistConfig` regenerates the TOML from that same running object. Strip the block on the way in and the operator's signer disappears from their config file the first time someone edits a curve in the dashboard. So the extra key rides along at runtime and is simply absent from the type the library publishes.

The consequence to keep in mind: `Simplex.start`'s guard against a signer-carrying config is now a runtime property read, because the type says the field cannot be there. That is deliberate — the objects it protects against are parsed TOML, which the type system never saw.
