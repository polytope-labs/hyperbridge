# 2026-09-07 — Remote access pairing: paste the phone's public key by default

`TunnelKeyStore.addDevice(label, publicKey?)` accepts a pasted OpenSSH public-key line
(`normalizePublicKey`: rejects private keys and unparsable text, drops the app's comment, refuses a
key already paired) and returns no private key in that case; generation stays as the fallback.
`POST /api/tunnel/devices` takes an optional `publicKey`; `TunnelNewDeviceDto.privateKey` is now
optional. The Remote access sheet opens in paste mode with a "Generate a key pair instead" switch,
and the post-pairing panel omits the key/QR/acknowledgement when nothing secret was shown.

Files: `src/services/tunnel/{keys,TunnelService,index}.ts`, `src/services/server/{UiServer,dto}.ts`,
`ui/src/operator/RemoteAccess.tsx`, `src/tests/{tunnel,ui-server-tunnel}.test.ts`, `README.md`, `docs/ai/*`.
