# 2026-09-03 — Installation stays desktop-first, generic, and permanently discoverable

Chosen: keep one three-step install guide with illustrated controls and a persistent install action
in both the setup header and operator navigation. Use the native install prompt when the platform
offers it, retain the same guide as the fallback, and report cancellation through a toast. The PWA
uses the established FX brand mark and caches only its application shell; live operator data still
comes from the local API when connectivity returns.

Alternatives rejected: browser/device selectors add choices to a desktop-only product without
changing the action; inline cancellation text changes the dialog geometry for a transient event;
caching API responses could display stale operational or financial state as current.
