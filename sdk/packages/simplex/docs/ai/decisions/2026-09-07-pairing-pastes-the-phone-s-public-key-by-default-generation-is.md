# 2026-09-07 — Pairing pastes the phone's public key by default; generation is the fallback

The first cut generated every device key in simplex and showed the private half on the desktop
screen for the phone to import. That puts a money key on a screen and a clipboard. Seun asked for
the reverse as the default: the phone's SSH app makes the key and the operator pastes the `.pub`
line, so the private half never exists anywhere but the phone. Generation stays behind a switch
for apps that cannot create keys. Pasted keys are normalised to `<type> <base64>` (comment
dropped, private keys and duplicates refused) so the `authorized_keys` line format stays uniform.
