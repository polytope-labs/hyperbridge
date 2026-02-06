require("dotenv").config();

module.exports = {
    networks: {
        development: {
            // For tronbox/tre docker image
            // See https://hub.docker.com/r/tronbox/tre
            privateKey: process.env.PRIVATE_KEY,
            userFeePercentage: 0,
            feeLimit: 15000 * 1e6,
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
                evmVersion: "paris",
                viaIR: true,
            },
        },
    },
};
