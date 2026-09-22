# Private-key verification

The setup wizard’s signer step now validates the private key as it is typed. Empty, malformed, checking, invalid, and verified states are shown inline; the EVM address is derived automatically once the key has 64 hexadecimal characters, stale async results are discarded, and Continue stays disabled until verification succeeds. The local validation request never persists the credential.
