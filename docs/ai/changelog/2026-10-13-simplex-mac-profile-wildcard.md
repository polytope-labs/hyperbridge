# Simplex Mac provisioning-profile wildcard authorization

Mac passkey signing accepts Associated Domains authorization as the scalar `"*"`,
an array containing `"*"`, or an array containing the configured
`webcredentials:<RP ID>`. Missing permissions and unrelated domains are rejected.

The profile authorizes capabilities; it does not define the app's claimed domains.
Generated app entitlements still contain only `webcredentials:passkeys.hyperfx.finance`
by default, or the explicitly configured RP ID. Team, application identifier,
expiration, and Developer ID distribution checks still apply.
