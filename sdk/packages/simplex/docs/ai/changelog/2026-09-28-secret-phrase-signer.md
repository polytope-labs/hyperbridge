# Secret phrase signer

Simplex accepts a BIP-39 secret phrase as its EVM signer, beside `privateKey`, `mpcVault`, and
`turnkey`. It signs everything with one wallet derived from the phrase, exactly as it does with a
private key.

## Config

```toml
[simplex.signer]
type = "secretPhrase"
phrase = "word1 word2 word3 word4 word5 word6 word7 word8 word9 word10 word11 word12"
accountIndex = 0 # optional, default 0
```

- `phrase` is a mnemonic of 12, 15, 18, 21, or 24 words from the BIP-39 English wordlist with a
  valid checksum. It is trimmed, its whitespace collapsed to single spaces, and lower-cased before
  it is validated and used.
- `accountIndex` is an integer from 0 to 2147483647. The wallet is at
  `m/44'/60'/0'/0/accountIndex`.

`validateSignerConfig` applies both rules, so a bad phrase or index stops the binary at startup.

## Library interface

Exported from `@hyperbridge/simplex`:

| Export | Description |
|---|---|
| `secretPhraseSigner(config)` | Takes a `SecretPhraseSignerConfig` (`phrase`, optional `accountIndex`) and returns a `DerivingSigner`. Throws on an invalid phrase or index. |
| `DerivingSigner` | A `Signer` plus `accountIndex` and `derive(index)`. `derive` returns the `Signer` for the wallet at `m/44'/60'/0'/0/index`, under the same index bounds. |
| `canDerive(signer)` | Type guard that narrows a `Signer` to a `DerivingSigner`. |
| `validateSecretPhrase(phrase)` | Throws unless the phrase passes the rules above. |
| `SignerType.SecretPhrase` | The `"secretPhrase"` tag of the `SignerConfig` union. |

`mode` is `"secretPhrase"` on the signer and on every signer `derive` returns.

## Setup

The `simplex init` wizard and the browser setup wizard both offer "Secret phrase" with an optional
account index, and validate both fields before continuing.

`POST /api/setup/derive-evm-address` accepts either `{ privateKey }` or `{ phrase, accountIndex? }`
and returns `{ address }`. `accountIndex` must be a JSON number. A body carrying both `privateKey`
and `phrase`, or an invalid phrase or index, is answered with status 400 and `{ error }`.

## Secret handling

- The phrase is stored in the config file in plain text and is protected only by the file's
  permissions, the same as a private key.
- `GET /api/config` and `POST /api/setup/preview` show the phrase as the fixed placeholder `****`,
  whatever its length.
- Validation errors name what is wrong and never quote the phrase. The setup API answers a body
  that is not valid JSON with the fixed message `Invalid JSON body`.
