# 2026-08-31 — createSigningRequest gets no retry

Chosen: only execute retries; a create failure propagates immediately. Review asked whether create
should retry symmetrically, since the production timing alone did not prove which RPC failed.

Rejected because the retry exists to absorb a read-after-write race on a freshly issued uuid, and
create references nothing freshly issued — its only uuid is the constant vault uuid, so an
INVALID_ARGUMENT there is deterministic and three attempts just triple the latency of a real config
error. Retrying create on ambiguous errors is actively harmful: each ambiguous failure may have
created a request, so retries would leak pending signing requests — the exact orphaning the
abandoned-request cleanup exists to prevent. If the create-is-fine assumption is wrong, failures now
name their RPC, so the next live failure identifies create and the decision gets revisited with
evidence.
