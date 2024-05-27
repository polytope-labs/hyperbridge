import rehypeKatex from "rehype-katex";
import rehypeStringify from "rehype-stringify";
import remarkMath from "remark-math";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import { VitePluginRadar } from "vite-plugin-radar";
import { defineConfig } from "vocs";

export default defineConfig({
  title: "Hyperbridge Documentation",
  description:
    "Hyperbridge is a coprocessor for cryptographically secure interoperability",
  // ogImageUrl:
  //   "https://vocs.dev/api/og?logo=%logo&title=%title&description=%description",
  logoUrl: {
    light: '/logo-dark.svg',
    dark: '/logo-light.svg'
  },
  iconUrl: "/favicon.svg",
  socials: [
    {
      icon: "github",
      link: "https://github.com/polytope-labs/hyperbridge",
    },
    {
      icon: "x",
      link: "https://twitter.com/hyperbridge_",
    },
    {
      icon: "discord",
      link: "https://discord.gg/WYTUQrTR9y",
    },
    {
      icon: "telegram",
      link: "https://t.me/hyper_bridge",
    },
  ],
  vite: {
    server: {
      fs: {
        allow: [".."],
      },
    },
    plugins: [
      VitePluginRadar({
        // Google Analytics tag injection
        analytics: {
          id: process.env.GA_ID!,
        },
      }),
    ],
  },
  rootDir: ".",
  markdown: {
    remarkPlugins: [
      remarkParse,
      remarkMath,
      remarkRehype,
      rehypeKatex,
      rehypeStringify,
    ],
  },
  sidebar: {
    "/protocol": [
      {
        text: "Overview",
        link: "/protocol",
      },
      {
        text: "Cryptographic Primitives",
        collapsed: true,
        items: [
          {
            text: "Hash Functions",
            link: "/protocol/cryptography/hash-functions",
          },
          {
            text: "Merkle Trees",
            items: [
              {
                text: "Binary Merkle Trees",
                link: "/protocol/cryptography/merkle-trees/binary",
              },
              {
                text: "Merkle Mountain Ranges",
                link: "/protocol/cryptography/merkle-trees/mountain-range",
              },
              {
                text: "Merkle Paticia Tries",
                link: "/protocol/cryptography/merkle-trees/patricia-trie",
              },
            ],
          },
          {
            text: "Digital Signatrues",
            link: "/protocol/cryptography/digital-signatures",
          },
          {
            text: "Polynomial Commitments",
            link: "/protocol/cryptography/polynomial-commitments",
          },
          {
            text: "Verkle Tries",
            link: "/protocol/cryptography/verkle-tries",
          },
          {
            text: "APK Proofs",
            link: "/protocol/cryptography/apk-proofs",
          },
        ],
      },
      {
        text: "Interoperability Proofs",
        collapsed: true,
        items: [
          {
            text: "State (Machine) Proofs",
            link: "/protocol/interoperability/state-machine-proofs",
          },
          {
            text: "Consensus Proofs",
            link: "/protocol/interoperability/consensus-proofs",
          },
        ],
      },
      {
        text: "ISMP",
        collapsed: true,
        items: [
          {
            text: "Introduction",
            link: "/protocol/ismp",
          },
          {
            text: "Host Interface",
            link: "/protocol/ismp/host",
          },
          {
            text: "Consensus Client",
            link: "/protocol/ismp/consensus",
          },
          {
            text: "State Machine Client",
            link: "/protocol/ismp/state-machine",
          },
          {
            text: "Router",
            link: "/protocol/ismp/router",
          },
          {
            text: "Dispatcher",
            link: "/protocol/ismp/dispatcher",
          },
          {
            text: "Requests",
            link: "/protocol/ismp/requests",
          },

          {
            text: "Responses",
            link: "/protocol/ismp/responses",
          },

          {
            text: "Timeouts",
            link: "/protocol/ismp/timeouts",
          },

<<<<<<< HEAD
          {
            text: "Proxies",
            link: "/protocol/ismp/proxies",
          },

          {
            text: "Relayers",
            link: "/protocol/ismp/relayers",
          },
        ],
      },
      {
        text: "Consensus Algorithms",
        collapsed: true,
        items: [
          {
              text: "GRANDPA (Polkadot)",
              link: "/protocol/consensus/grandpa",
          },
          {
            text: "BEEFY (Polkadot)",
            link: "/protocol/consensus/beefy",
          },
          {
            text: "Sync Committee (Ethereum)",
            link: "/protocol/consensus/sync-committee",
          },
          {
            text: "Casper FFG (Ethereum)",
            link: "/protocol/consensus/casper-ffg",
          },
          {
            text: "Parlia (Bsc)",
            link: "/protocol/consensus/parlia",
          },
        ],
      },
      {
        text: "State Machine Algorithms",
        collapsed: true,
        items: [
          {
            text: "Parachain",
            link: "/protocol/state-machine/parachain",
          },
          {
            text: "Fault Dispute Games (OP Stack)",
            link: "/protocol/state-machine/fault-dispute-games",
          },
          {
            text: "L2 Oracle (OP Stack)",
            link: "/protocol/state-machine/l2-oracle",
          },
          {
            text: "Orbit (Arbitrum)",
            link: "/protocol/state-machine/arbitrum-orbit",
          },
        ],
      }
    ],
    "/developers": [
      {
        text: "Introduction",
        link: "/developers",
        items: [],
      },
      {
        text: "Solidity Sdk",
        collapsed: true,
        items: [
          {
            text: "Contracts",
            link: "/evm/integration",
          },
=======
        {
          text: "Proxies",
          link: "/protocol/proxies",
        },
      ],
    },
    {
      text: "Solidity Sdk",
      collapsed: true,
      items: [
        {
          text: "Overview",
          link: "/evm/solidity",
        },

        {
          text: "Integration",
          link: "/evm/integration",
        },

        {
          text: "Protocol Fees",
          link: "/evm/fees",
        },

        {
          text: "Contracts addresses",
          link: "/evm/contract-addresses",
        },

>>>>>>> 3183025 (wip: integration docs)

          // {
          //   text: "Protocol Fees",
          //   link: "/evm/fees",
          // },

<<<<<<< HEAD
          // {
          //   text: "Message delivery",
          //   link: "/evm/delivery",
          // },
=======
        // {
        //   text: "Supported Networks",
        //   link: "/evm/networks",
        // },
      ],
    },
    {
      text: "Polkadot Sdk",
      collapsed: true,
      items: [
        {
          text: "Integration",
          link: "/polkadot/integration",
        },

        {
          text: "Hyperbridge Integration - Parachains",
          link: "/polkadot/parachains",
        },

        // {
        //   text: "Integration - Solochains",
        //   link: "/polkadot/solochains",
        // },
        {
          text: "ISMP Modules",
          link: "/polkadot/modules",
        },
        {
          text: "RPC Interface",
          link: "/polkadot/rpc",
        },
        {
          text: "Protocol Fees",
          link: "/polkadot/fees",
        },
        // {
        //   text: "Message delivery",
        //   link: "/polkadot/delivery",
        // },
        // {
        //   text: "Supported Networks",
        //   link: "/polkadot/networks",
        // },
      ],
    },
    {
      text: "Network Operators",
      collapsed: true,
      items: [
        {
          text: "Running a Node",
          link: "/network/node",
        },
>>>>>>> 3183025 (wip: integration docs)

          // {
          //   text: "Supported Networks",
          //   link: "/evm/networks",
          // },
        ],
      },
      {
        text: "Polkadot Sdk",
        collapsed: true,
        items: [
          {
            text: "Integration - Parachains",
            link: "/polkadot/parachains",
          },
          {
            text: "Integration - Solochains",
            link: "/polkadot/solochains",
          },
          {
            text: "ISMP Modules",
            link: "/polkadot/modules",
          },
          {
            text: "RPC Interface",
            link: "/polkadot/rpc",
          },
          {
            text: "Protocol Fees",
            link: "/polkadot/fees",
          },
          {
            text: "Message delivery",
            link: "/polkadot/delivery",
          },
          {
            text: "Supported Networks",
            link: "/polkadot/networks",
          },
        ],
      },
      {
        text: "Network Operators",
        collapsed: true,
        items: [
          {
            text: "Running a Node",
            link: "/network/node",
          },

          {
            text: "Running a Relayer",
            link: "/network/relayer",
          },
        ],
      },
    ]
  }
});
