# 2026-09-07 — `ssh2` is imported as a default export

`ssh2` is CommonJS. `import { Client } from "ssh2"` type-checks and passes under vitest (vite's
CJS interop) but the shipped ESM binary throws "does not provide an export named" at load, which
the smoke test against the real relay caught. The tunnel modules destructure from the default
import; types come from `import type`. `ssh2` is also external in tsup because it probes for an
optional native crypto binding relative to its package directory.
