# Simplex side-panel chain defaults

New chains in the operator's Chains panel start with their catalog's `defaultRpcUrls`, matching
the setup wizard. Chains without bundled endpoints start with one empty RPC field. Configured
chains retain their saved RPC endpoints, including custom chains outside the catalog.

Alchemy prefill sets bundlers only, preserving the entire public or saved RPC list. The derived
Alchemy RPC URL serves as the bundler fallback when no separate bundler URL is returned, matching
the wizard. Chains without public defaults still require an RPC URL from the operator. Editing
the bundler clears its Alchemy badge; editing an RPC does not.
