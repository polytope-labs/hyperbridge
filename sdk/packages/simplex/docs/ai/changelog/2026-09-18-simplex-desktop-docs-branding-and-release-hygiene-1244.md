# Simplex Desktop docs, branding, and release identity (#1244)

Simplex documentation and the package README now lead with the native macOS, Windows, and Linux
application while retaining Docker and npm as advanced deployment paths. They name the installer
artifacts, explain the PWA's browser/remote-access role, document the desktop config locations and
plaintext key storage, and prescribe a private Unix socket with SSH forwarding or the authenticated
tunnel on shared hosts.

The shared desktop and browser setup wizard configures mainnet chains only. It no longer offers a
testnet choice or publishes testnet setup defaults; the separate CLI initializer retains its
advanced testnet path.

The released application identity is fixed as `Simplex` / `network.hyperbridge.simplex`, and desktop
builds continue to require the exact `@hyperbridge/simplex` version. Platform installer icons now use
the current PWA app icon at `.icns`, `.ico`, and Linux PNG sizes, preserving its multicolored center
at every supported size. Packaged resources include the Electron MIT license, Chromium's bundled
license page, and a third-party notice.
