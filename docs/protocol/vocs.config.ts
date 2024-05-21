import { defineConfig } from "vocs";
import rehypeKatex from "rehype-katex";
import rehypeStringify from "rehype-stringify";
import remarkMath from "remark-math";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import { VitePluginRadar } from "vite-plugin-radar";

export default defineConfig({
  title: 'Protocol Specification',
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
  sidebar: [
  {
    text: "Overview",
    link: "/",
  },
  {
    text: "Cryptographic Primitives",
    collapsed: true,
    link: "/cryptography",
    items: [
      {
          text: "Hash Functions",
          link: "/cryptography/hash-functions",
        },
      {
        text: "Merkle Trees",
        link: "/cryptography/merkle-trees",
        items: [
          {
            text: "Binary Merkle Trees",
            link: "/cryptography/merkle-trees/binary",
          },
          {
            text: "Merkle Mountain Ranges",
            link: "/cryptography/merkle-trees/mountain-range",
          },
          {
            text: "Merkle Paticia Tries",
            link: "/cryptography/merkle-trees/patricia",
          },
        ]
      },
      {
        text: "Abstract Algebra",
        link: "/cryptography/abstract-algebra",
      },
      {
        text: "Elliptic Curves",
        link: "/cryptography/elliptic-curves",
      },
      {
        text: "BLS Signatrues",
        link: "/cryptography/bls",
      },
      {
        text: "Polynomial Commitments",
        link: "/cryptography/polynomial-comitments",
      },
      {
        text: "Verkle Tries",
        link: "/cryptography/verkle-tries",
      },
      {
        text: "APK Proofs",
        link: "/cryptography/apk-proofs",
      },
    ],
  },
  {
    text: "Interoperability Proofs",
    collapsed: true,
    link: "/interoperability",
    items: [
      {
        text: "Consensus Proofs",
        link: "/interoperability/consensus-proofs",
      },
      {
        text: "State (Machine) Proofs",
        link: "/interoperability/state-machine-proofs",
      }
    ],
  },
  {
    text: "ISMP",
    collapsed: true,
    link: "/ismp",
    items: [
      {
        text: "Host Interface",
        link: "/ismp/host",

      },
      {
        text: "Consensus Client",
        link: "/ismp/consensus",
      },
      {
        text: "State Machine Client",
        link: "/ismp/state-machine",
      },
      {
        text: "Router",
        link: "/ismp/router",
      },
      {
        text: "Dispatcher",
        link: "/ismp/dispatcher",
      },
      {
        text: "Requests",
        link: "/ismp/requests",
      },

      {
        text: "Responses",
        link: "/ismp/responses",
      },

      {
        text: "Timeouts",
        link: "/ismp/timeouts",
      },

      {
        text: "Proxies",
        link: "/ismp/proxies",
      },
    ],
  },
  {
    text: "Consensus Algorithms",
    collapsed: true,
    link: "/algorithms/consensus",
    items: [
      {
        text: "BEEFY (Polkadot)",
        link: "/algorithms/consensus/ismp",
      },
      {
        text: "Sync Committee (Ethereum)",
        link: "/algorithms/consensus/host",
      },
      {
        text: "Casper FFG (Ethereum)",
        link: "/algorithms/consensus/host",
      },
      {
        text: "Parlia (Bsc)",
        link: "/algorithms/consensus/host",
      }
    ],
  },
  {
    text: "State Machine Algorithms",
    collapsed: true,
    link: "/algorithms/state-machine",
    items: [
      {
        text: "Parachain",
        link: "/algorithms/state-machine/parachain",
      },
      {
        text: "Fault Dispute Games (OP Stack)",
        link: "/algorithms/state-machine/fault-dispute-games",
      },
      {
        text: "L2 Oracle (OP Stack)",
        link: "/algorithms/state-machine/l2-oracle",
      },
      {
        text: "Orbit (Arbitrum)",
        link: "/algorithms/state-machine/orbit",
      }
    ],
  },
  {
    text: "Runtime Modules",
    collapsed: true,
    link: "/modules",
    items: [
      {
        text: "Pallet ISMP",
        link: "/modules/ismp",
      },
      {
        text: "ISMP Parachain",
        link: "/modules/ismp-parachain",
      },
      {
        text: "ISMP Sync Committee",
        link: "/modules/ismp-sync-committee",
      },
      {
        text: "Pallet Fishermen",
        link: "/modules/fishermen",
      },
      {
        text: "Pallet Host Executive",
        link: "/modules/host-executive",
      },
      {
        text: "Pallet ISMP Relayer",
        link: "/modules/relayer",
      }
    ],
  },
  ],
})
