# Setup completion failures

`Review.tsx` posts the validated config to `/api/setup/save-and-start`; `setup-api.ts` atomically
writes it, returns immediately, and boots the filler while the browser polls start status. A failed
all-chain EIP-7702 delegation remains a real failed start, but `formatSetupStartError` replaces the
internal restart wording with the affected network labels and explicit stablecoin, endpoint, retry,
and image-version checks. The configuration remains on disk, the action changes to Retry startup,
and unrelated failures pass through unchanged.
