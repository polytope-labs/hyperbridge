import rehypeKatex from "rehype-katex";
import rehypeStringify from "rehype-stringify";
import remarkMath from "remark-math";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import { VitePluginRadar } from "vite-plugin-radar";
import { defineConfig } from "vocs";

export default defineConfig({
  title: "Protocol Specification",
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
      items: [
        {
          text: "Hash Functions",
          link: "/cryptography/hash-functions",
        },
        {
          text: "Merkle Trees",
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
              link: "/cryptography/merkle-trees/patricia-trie",
            },
          ],
        },
        {
          text: "Digital Signatrues",
          link: "/cryptography/digital-signatures",
        },
        {
          text: "Polynomial Commitments",
          link: "/cryptography/polynomial-commitments",
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
      items: [
        {
          text: "State (Machine) Proofs",
          link: "/interoperability/state-machine-proofs",
        },
        {
          text: "Consensus Proofs",
          link: "/interoperability/consensus-proofs",
        },
      ],
    },
    {
      text: "ISMP",
      collapsed: true,
      items: [
        {
          text: "Introduction",
          link: "/ismp",
        },
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

        {
          text: "Relayers",
          link: "/ismp/relayers",
        },
      ],
    },
    {
      text: "Consensus Algorithms",
      collapsed: true,
      items: [
        {
            text: "GRANDPA (Polkadot)",
            link: "/consensus/grandpa",
        },
        {
          text: "BEEFY (Polkadot)",
          link: "/consensus/beefy",
        },
        {
          text: "Sync Committee (Ethereum)",
          link: "/consensus/sync-committee",
        },
        {
          text: "Casper FFG (Ethereum)",
          link: "/consensus/casper-ffg",
        },
        {
          text: "Parlia (Bsc)",
          link: "/consensus/parlia",
        },
      ],
    },
    {
      text: "State Machine Algorithms",
      collapsed: true,
      items: [
        {
          text: "Parachain",
          link: "/state-machine/parachain",
        },
        {
          text: "Fault Dispute Games (OP Stack)",
          link: "/state-machine/fault-dispute-games",
        },
        {
          text: "L2 Oracle (OP Stack)",
          link: "/state-machine/l2-oracle",
        },
        {
          text: "Orbit (Arbitrum)",
          link: "/state-machine/arbitrum-orbit",
        },
      ],
    }
  ],
});
