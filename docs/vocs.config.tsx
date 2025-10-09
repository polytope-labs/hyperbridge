import rehypeKatex from "rehype-katex";
import rehypeStringify from "rehype-stringify";
import remarkMath from "remark-math";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import { VitePluginRadar } from "vite-plugin-radar";
import { defineConfig } from "vocs";
import type { UserOptions } from "sitemap-ts";
import { generateSitemap } from "sitemap-ts";
import { glob } from "glob";

function Sitemap(options: UserOptions = {}) {
  return {
    name: "vite-plugin-sitemap",
    async closeBundle() {
      const paths = (
        await glob("./**/*.mdx", { ignore: "node_modules/**" })
      ).map((f) => {
        f = f.replace("/index.mdx", "");
        f = f.replace(".mdx", "");
        f = f.replace("pages", "");

        return f;
      });
      options.dynamicRoutes = paths;
      generateSitemap(options);
    },
    transformIndexHtml() {
      return [
        {
          tag: "link",
          injectTo: "head",
          attrs: {
            rel: "sitemap",
            type: "application/xml",
            title: "Sitemap",
            href: "/sitemap.xml",
          },
        },
      ];
    },
  };
}

export default defineConfig({
  title: "Hyperbridge Documentation",
  description:
    "Hyperbridge is a coprocessor for cryptographically secure interoperability",
  ogImageUrl: "https://docs.hyperbridge.network/og.png",
  logoUrl: {
    light: "/logo_black.svg",
    dark: "/logo_white.svg",
  },
  head() {
    return (
      <>
        <link
          href="https://cdn.jsdelivr.net/npm/katex@0.16.8/dist/katex.min.css"
          rel="stylesheet"
        />
        <link
          rel="stylesheet"
          href="https://cdn.jsdelivr.net/npm/pseudocode@2.4.1/build/pseudocode.min.css"
        />
      </>
    );
  },
  baseUrl: "https://docs.hyperbridge.network",
  editLink: {
    pattern:
      "https://github.com/polytope-labs/hyperbridge/blob/main/docs/pages/:path",
    text: "Suggest changes to this page",
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
      Sitemap({
        hostname: "https://docs.hyperbridge.network",
      }) as any,
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
            text: "Digital Signatures",
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
      },
    ],
    "/developers": [
      {
        text: "Introduction",
        link: "/developers",
        items: [],
      },
      {
        text: "Explore",
        collapsed: true,
        items: [
          {
            text: "Protocol Overview",
            link: "/developers/explore",
          },
          {
            text: "Permissionless Relayers",
            link: "/developers/explore/relayer",
            items: [
              {
                text: "Messaging Relayers",
                link: "/developers/explore/relayers/messaging-relayers",
              },
              {
                text: "Consensus Relayers",
                link: "/developers/explore/relayers/consensus-relayers",
              },
            ],
          },
          {
            text: "Fishermen",
            link: "/developers/explore/fishermen",
          },
          {
            text: "Hyperbridge Nexus",
            link: "/developers/explore/nexus",
            items: [
              {
                text: "ISMP",
                link: "/developers/explore/modules/ismp",
              },
              {
                text: "Host Executive",
                link: "/developers/explore/modules/host-executive",
              },
              {
                text: "Relayer",
                link: "/developers/explore/modules/relayer",
              },
            ],
          },
          {
            text: "Configurations",
            collapsed: false,
            items: [
              {
                text: "Mainnet",
                link: "/developers/explore/configurations/mainnet",
              },
              {
                text: "Testnet",
                link: "/developers/explore/configurations/testnet",
              },
            ],
          },
        ],
      },
      {
        text: "Solidity Sdk",
        collapsed: true,
        items: [
          {
            text: "Getting Started",
            link: "/developers/evm/getting-started",
          },
          {
            text: "Dispatching Messages",
            link: "/developers/evm/dispatching",
          },
          {
            text: "Fees",
            link: "/developers/evm/fees",
          },
          {
            text: "Receiving Messages",
            link: "/developers/evm/receiving",
          },

          {
            text: "Intent Gateway",
            link: "/developers/evm/intent-gateway",
          },

          {
            text: "Intent Gateway Filler",
            link: "/developers/evm/filler",
          },
        ],
      },
      {
        text: "Polkadot Sdk",
        collapsed: true,
        items: [
          {
            text: "Getting Started",
            link: "/developers/polkadot/getting-started",
          },

          {
            text: "Pallet ISMP",
            link: "/developers/polkadot/pallet-ismp",
            items: [
              {
                text: "Runtime API",
                link: "/developers/polkadot/pallet-ismp-runtime-api",
              },
              {
                text: "RPC Interface",
                link: "/developers/polkadot/pallet-ismp-rpc",
              },
            ],
          },

          {
            text: "Parachains",
            link: "/developers/polkadot/parachains",
            items: [
              {
                text: "Runtime API",
                link: "/developers/polkadot/ismp-parachain-runtime-api",
              },
              {
                text: "Inherent Provider",
                link: "/developers/polkadot/ismp-parachain-inherent",
              },
            ],
          },

          {
            text: "Solochains (GRANDPA)",
            link: "/developers/polkadot/solochains",
          },

          // {
          //   text: "Integration - Solochains",
          //   link: "/developers/polkadot/solochains",
          // },

          {
            text: "Dispatching Messages",
            link: "/developers/polkadot/dispatching",
          },

          {
            text: "Fees",
            link: "/developers/polkadot/fees",
          },

          {
            text: "Receiving Messages",
            link: "/developers/polkadot/receiving",
          },

          {
            text: "Token Gateway",
            link: "/developers/polkadot/token-gateway",
          },

          // {
          //   text: "Supported Networks",
          //   link: "/developers/polkadot/networks",
          // },
        ],
      },
      {
        text: "Network Operators",
        collapsed: true,
        items: [
          {
            text: "Running a Node",
            link: "/developers/network/node",
          },

          {
            text: "Running a Messaging Relayer",
            link: "/developers/network/relayer/messaging/relayer",
            items: [
              {
                text: "Common Errors",
                link: "/developers/network/relayer/messaging/errors",
              },
            ],
          },

          {
            text: "Running a Consensus Relayer",
            link: "/developers/network/relayer/consensus/relayer",
          },

          {
            text: "Becoming a Collator",
            link: "/developers/network/collator",
          },
        ],
      },
    ],
  },
});
