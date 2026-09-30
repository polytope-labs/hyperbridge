# 2026-09-05 — Browser favicon is the Hyperbridge mark, not the HyperFX wordmark

Chosen: `ui/public/favicon.ico` is a byte-for-byte copy of `docs/public/favicon.ico`, the Hyperbridge
mark used by the docs site. The brandbars keep the HyperFX wordmark; only browser chrome changes.
Requested by the maintainer after reviewing the redesign. The service worker cache name was bumped
so installed PWAs drop the precached HyperFX icon on their next activation.

Alternatives rejected: rendering `docs/public/favicon.svg` into a new `.ico` would produce a
different (white-on-transparent) look from the mark the docs site already ships; keeping the
HyperFX favicon (the 2026-09-03 decision below) was overridden by the maintainer.
