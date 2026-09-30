# 2026-09-07 — Device keys live in a plain `authorized_keys`

One OpenSSH line per device with the label URL-encoded in the comment (`simplex-device:<label>:<ms>`),
so an operator can read or edit the file with tools they already know, and a hand-added line still
works (its comment becomes the label). The private half is returned once from pairing and never
written anywhere by simplex.
