# Hide the PWA install control in native desktop

The shared Simplex renderer now shows `Install app` only when served by a browser. The Electron
desktop renderer uses the `simplex:` protocol, so it omits the setup and operator install controls
and does not register browser PWA install listeners. Browser and PWA installation behavior is
unchanged.
