import { defineConfig } from "vocs";
import rehypeKatex from "rehype-katex";
import rehypeStringify from "rehype-stringify";
import remarkMath from "remark-math";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import { VitePluginRadar } from "vite-plugin-radar";

export default defineConfig({
  title: "Hyperbridge",
  description:
    "Hyperbridge is a coprocessor for cryptographically secure interoperability",
  ogImageUrl:
    "https://vocs.dev/api/og?logo=%logo&title=%title&description=%description",
  logoUrl: "/logotype.svg",
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
  sidebar: [
    {
      text: "Introduction",
      link: "/",
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

        // {
        //   text: "Protocol Fees",
        //   link: "/evm/fees",
        // },

        // {
        //   text: "Message delivery",
        //   link: "/evm/delivery",
        // },

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
    {
      text: "Runtime Modules",
      collapsed: true,
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
          link: "/modules/sync-committee",
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
        },
      ],
    },
  ],
});
