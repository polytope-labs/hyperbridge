# Simplex Mac passkey signing and sign-in availability

Mac passkey signing accepts Associated Domains authorization as the scalar `"*"`,
an array containing `"*"`, or an array containing the configured
`webcredentials:<RP ID>`. Missing permissions and unrelated domains are rejected.

The profile authorizes capabilities; it does not define the app's claimed domains.
Generated app entitlements still contain only `webcredentials:passkeys.hyperfx.finance`
by default, or the explicitly configured RP ID. Team, application identifier,
expiration, and Developer ID distribution checks still apply.

The profile reader extracts required fields as XML before converting those fields
to JSON. This supports older macOS `plutil` versions when profiles contain binary
certificates, DER data, or date metadata outside the selected fields.

Desktop sign-in shows passkey actions only when they are available. Otherwise,
profiles with a password show password sign-in; passkey-only profiles retain
recovery access. Build signing and provisioning diagnostics are not rendered in
the sign-in screens.

Setup and sign-in use one short description for the selected method. Access cards
use compact spacing and grow with their content. Recovery is the primary action
when it is the profile's only available sign-in method.
