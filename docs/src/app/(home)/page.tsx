import Link from "next/link";

export default function HomePage() {
    return (
        <div className="flex flex-col max-w-5xl mx-auto px-6 py-12">
            <section className="mb-16">
                <h1 className="text-5xl font-bold mb-6">
                    Hyperbridge Protocol
                </h1>
                <p className="text-lg text-muted-foreground mb-4">
                    <strong>Secure interoperability</strong> requires the
                    verification of various proofs, including consensus proofs,
                    consensus fault proofs, state proofs, and state transition
                    validity proofs. For blockchains to securely interoperate,
                    they must <strong>verify these proofs onchain</strong> to
                    confirm the finalized (irreversible) state of their
                    counterparty.
                </p>
                <p className="text-lg text-muted-foreground">
                    However, these verification processes are compute-intensive
                    and do not scale well, particularly when multiple
                    blockchains need to communicate. This limitation leads to
                    the proliferation of <em>attestation networks</em> which
                    employ the use of multi-sig committees who attest to the
                    state of a counterparty chain. These types of architectures
                    have resulted in a cumulative loss of $2 billion in crypto
                    assets. (sources{" "}
                    <a
                        href="https://defillama.com/hacks"
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        [1]
                    </a>
                    ,{" "}
                    <a
                        href="https://rekt.news/leaderboard/"
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        [2]
                    </a>
                    ).
                </p>
                <p className="text-lg text-muted-foreground">
                    The only solution to this problem is the coprocessor model,
                    where the verification operations are performed offchain and
                    the results are securely reported back onchain alongside
                    cryptographic proofs of correct execution.
                </p>
            </section>

            <section className="mb-16">
                <h2 className="text-3xl font-bold mb-8">Coprocessor Model</h2>
                <p className="text-base text-muted-foreground">
                    Hyperbridge is an example of such a coprocessor, more
                    specifically, a crypto-economic coprocessor. Hyperbridge
                    pioneers a new class of coprocessors that leverage their
                    consensus proofs to attest to the correctness of the{" "}
                    <strong>computations performed onchain</strong>.
                </p>
            </section>

            <section className="mb-16">
                <h2 className="text-3xl font-bold mb-8">Proof Aggregation</h2>
                <p className="text-base text-muted-foreground">
                    Hyperbridge scales trust-free interoperability to all chains
                    by verifying and aggregating the finalized states of all
                    chains into a single proof. This proof allows any blockchain
                    to receive all cross-chain messages aggregated by
                    Hyperbridge.
                </p>
            </section>

            <section className="mb-16">
                <h2 className="text-3xl font-bold mb-8">
                    Permissionless Relayers
                </h2>
                <p className="text-base text-muted-foreground">
                    Hyperbridge is the first cross-chain protocol of its kind
                    that leverages cryptographic proofs to power a decentralized
                    and permissionless network of relayers. These relayers,
                    which operate{" "}
                    <strong>
                        without the need for any whitelisting or staking
                    </strong>
                    , are tasked with transmitting messages across chains on
                    behalf of users and applications. They are fully
                    incentivized to relay messages by the fees paid by users who
                    wish to perform cross-chain operations.
                </p>
            </section>

            <section className="mb-16">
                <h2 className="text-3xl font-bold mb-6">Where to Start</h2>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <Link
                        href="/developers"
                        className="p-6 border rounded-lg hover:bg-muted/50 transition-colors"
                    >
                        <h3 className="text-xl font-semibold mb-2">
                            Hyperbridge For Developers
                        </h3>
                        <p className="text-sm text-muted-foreground">
                            Build cross-chain applications with the Hyperbridge
                            SDK
                        </p>
                    </Link>
                    <Link
                        href="/protocol"
                        className="p-6 border rounded-lg hover:bg-muted/50 transition-colors"
                    >
                        <h3 className="text-xl font-semibold mb-2">
                            Hyperbridge Protocol Specification
                        </h3>
                        <p className="text-sm text-muted-foreground">
                            Learn about the protocol architecture and
                            cryptographic primitives
                        </p>
                    </Link>
                </div>
            </section>

            <section className="mb-16">
                <h2 className="text-3xl font-bold mb-6">Useful Links</h2>
                <ul className="list-disc list-inside space-y-2 text-base text-muted-foreground">
                    <li>
                        <a
                            href="https://blog.hyperbridge.network/introducing-hyperbridge-interoperability-coprocessor"
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-brand-purple hover:underline"
                        >
                            Polytope Labs, "Introducing Hyperbridge: An
                            Interoperability Coprocessor"
                        </a>
                    </li>
                    <li>
                        <a
                            href="https://blog.hyperbridge.network/cryptoeconomic-coprocessors"
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-brand-purple hover:underline"
                        >
                            Polytope Labs, "Cryptoeconomic Coprocessors and
                            their applications"
                        </a>
                    </li>
                    <li>
                        <a
                            href="https://www.rob.tech/blog/coprocessor-competition/"
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-brand-purple hover:underline"
                        >
                            Rob Habermeier, "Coprocessor Market Structure:
                            Cryptoeconomic vs ZK"
                        </a>
                    </li>
                </ul>
            </section>

            <footer className="text-center text-sm text-muted-foreground border-t pt-8">
                <p>Copyright © 2026 Polytope Labs</p>
            </footer>
        </div>
    );
}
