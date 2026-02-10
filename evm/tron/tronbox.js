const path = require("path");
const dotenv = require("dotenv");

// ─── Resolve which .env file to load ─────────────────────────────────────────
//
// Priority:
//   1. TRON_ENV environment variable        (e.g. TRON_ENV=mainnet tronbox migrate ...)
//   2. --network flag from the CLI args     (e.g. tronbox migrate --network mainnet)
//   3. Falls back to .env
//
// Mapping:
//   mainnet              → .env.mainnet
//   shasta / nile / development → .env.testnet
//   (anything else)      → .env

function resolveNetwork() {
    if (process.env.TRON_ENV) return process.env.TRON_ENV;

    const idx = process.argv.indexOf("--network");
    if (idx !== -1 && process.argv[idx + 1]) return process.argv[idx + 1];

    return "";
}

function envFileForNetwork(network) {
    switch (network) {
        case "mainnet":
            return ".env.mainnet";
        case "shasta":
        case "nile":
        case "development":
            return ".env.testnet";
        default:
            return ".env";
    }
}

const network = resolveNetwork();
const envFile = envFileForNetwork(network);
const envPath = path.resolve(__dirname, envFile);

// Load the network-specific file first, then .env as a fallback for any
// variables that are not set in the network-specific file.
dotenv.config({ path: envPath });
dotenv.config({ path: path.resolve(__dirname, ".env"), override: false });

module.exports = {
    networks: {
        development: {
            // For tronbox/tre docker image
            // See https://hub.docker.com/r/tronbox/tre
            privateKey: process.env.PRIVATE_KEY,
            userFeePercentage: 0,
            feeLimit: 1000 * 1e6,
            fullHost: "http://127.0.0.1:9090",
            network_id: "*",
        },

        shasta: {
            // Obtain test TRX at https://shasta.tronex.io/
            privateKey: process.env.PRIVATE_KEY,
            userFeePercentage: 100,
            feeLimit: 15000 * 1e6,
            fullHost: "https://api.shasta.trongrid.io",
            network_id: "2",
        },

        nile: {
            // Obtain test TRX at https://nileex.io/join/getJoinPage
            privateKey: process.env.PRIVATE_KEY,
            userFeePercentage: 100,
            feeLimit: 15000 * 1e6,
            fullHost: "https://nile.trongrid.io",
            network_id: "3",
        },

        mainnet: {
            privateKey: process.env.PRIVATE_KEY,
            userFeePercentage: 100,
            feeLimit: 15000 * 1e6,
            fullHost: "https://api.trongrid.io",
            network_id: "1",
        },
    },

    compilers: {
        solc: {
            version: "0.8.25",
            settings: {
                optimizer: {
                    enabled: true,
                    runs: 200,
                },
                evmVersion: "istanbul",
                viaIR: true,
            },
        },
    },
};
