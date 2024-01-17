import { defineConfig } from "vocs";
import rehypeKatex from 'rehype-katex'
import rehypeStringify from 'rehype-stringify'
import remarkMath from 'remark-math'
import remarkParse from 'remark-parse'
import remarkRehype from 'remark-rehype'

export default defineConfig({
  title: "Hyperbridge",
  vite: {
    server: {
      fs: {
        allow: [".."]
      }
    }
  },
  markdown: {
    remarkPlugins: [remarkParse, remarkMath, remarkRehype, rehypeKatex, rehypeStringify],
  },
  sidebar: [
    {
      text: "Introduction",
      link: "/",
    },
    {
      text: "Protocol",
      collapsed: true,
      items: [
        {
          text: "ISMP",
          link: "/protocol/ismp",
        },
        {
          text: "Host Interface",
          link: "/protocol/host",
        },
        {
          text: "Consensus Client",
          link: "/protocol/consensus-client",
        },
        {
          text: "State Machine Client",
          link: "/protocol/state-machine-client",
        },
        {
          text: "Router",
          link: "/protocol/router",
        },
        {
          text: "Dispatcher",
          link: "/protocol/dispatcher",
        },
        {
          text: "Requests",
          link: "/protocol/requests",
        },

        {
          text: "Responses",
          link: "/protocol/responses",
        },

        {
          text: "Timeouts",
          link: "/protocol/timeouts",
        },
      ],
    },
    {
      text: "Solidity Sdk",
      collapsed: true,
      items: [
        {
          text: "Integration",
          link: "/evm/config",
        },

        {
          text: "Protocol Fees",
          link: "/evm/config",
        },

        {
          text: "Message delivery",
          link: "/evm/config",
        },

        {
          text: "Supported Networks",
          link: "/evm/config",
        },
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
          link: "/polkadot/solocahins",
        },
        {
          text: "ISMP Modules",
          link: "/polkadot/modules",
        },
        {
          text: "RPC Interface",
          link: "/polkadot/modules",
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
          link: "/polkadot/network",
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
  ],
});
