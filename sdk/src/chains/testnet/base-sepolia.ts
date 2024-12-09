import { EthereumDatasourceKind, EthereumHandlerKind, EthereumProject } from '@subql/types-ethereum';

const project: EthereumProject = {
  specVersion: '1.0.0',
  version: '0.0.1',
  name: 'base-sepolia',
  description: 'Base Sepolia Chain Indexer',
  runner: {
    node: {
      name: '@subql/node-ethereum',
      version: '>=3.0.0',
    },
    query: {
      name: '@subql/query',
      version: '*',
    },
  },
  schema: {
    file: './schema.graphql',
  },
  network: {
    chainId: '84532',
    endpoint: [
      'https://base-sepolia.blockpi.network/v1/rpc/public',
      'https://base-sepolia-rpc.publicnode.com',
    ],
  },
  dataSources: [
    {
      kind: EthereumDatasourceKind.Runtime,
      startBlock: 13464107,
      options: {
        abi: 'ethereumHost',
        address: '0xe7C43500e07E0Bb5fC0987db95fE57Ce29B9bb80',
      },
      assets: new Map([
        ['ethereumHost', { file: './abis/EthereumHost.abi.json' }],
        ['chainLinkAggregatorV3', { file: './abis/ChainLinkAggregatorV3.abi.json' }],
      ]),
      mapping: {
        file: './dist/index.js',
        handlers: [
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handlePostRequestEvent',
            filter: {
              topics: [
                'PostRequestEvent(bytes,bytes,bytes,bytes,uint256,uint256,bytes,uint256)',
              ],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handlePostResponseEvent',
            filter: {
              topics: [
                'PostResponseEvent(bytes,bytes,bytes,bytes,uint256,uint256,bytes,bytes,uint256,uint256)',
              ],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handlePostRequestHandledEvent',
            filter: {
              topics: ['PostRequestHandled(bytes32, address)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handlePostResponseHandledEvent',
            filter: {
              topics: ['PostResponseHandled(bytes32,address)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handlePostRequestTimeoutHandledEvent',
            filter: {
              topics: ['PostRequestTimeoutHandled(bytes32, bytes)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handlePostResponseTimeoutHandledEvent',
            filter: {
              topics: ['PostResponseTimeoutHandled(bytes32, bytes)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleGetRequestHandledEvent',
            filter: {
              topics: ['GetRequestHandled(bytes32,address)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleGetRequestTimeoutHandledEvent',
            filter: {
              topics: ['GetRequestTimeoutHandled(bytes32, bytes)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleStateMachineUpdatedEvent',
            filter: {
              topics: ['StateMachineUpdated(bytes,uint256)'],
            },
          },
        ],
      },
    },
    {
      kind: EthereumDatasourceKind.Runtime,
      startBlock: 13464107,
      options: {
        abi: 'erc6160ext20',
        address: '0x83aF3a8a53bf0E379c17A8611AD697401A5970fD',
      },
      assets: new Map([
        ['erc6160ext20', { file: './abis/ERC6160Ext20.abi.json' }],
      ]),
      mapping: {
        file: './dist/index.js',
        handlers: [
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleTransferEvent',
            filter: {
              topics: ['Transfer(address indexed from, address indexed to, uint256 amount)'],
            },
          },
        ],
      },
    },
    {
      kind: EthereumDatasourceKind.Runtime,
      startBlock: 13464107,
      options: {
        abi: 'handlerV1',
        address: '0x0FAC5FfFa3C7F0C22CF0aa644ae68eEF8Db2456E',
      },
      assets: new Map([
        ['handlerV1', { file: './abis/HandlerV1.abi.json' }],
      ]),
      mapping: {
        file: './dist/index.js',
        handlers: [
          {
            handler: 'handlePostRequestTransactionHandler',
            kind: EthereumHandlerKind.Call,
            filter: {
              function: '0x9d38eb35',
            },
          },
          {
            handler: 'handlePostResponseTransactionHandler',
            kind: EthereumHandlerKind.Call,
            filter: {
              function: '0x72becccd',
            },
          },
        ],
      },
    },
    {
      kind: EthereumDatasourceKind.Runtime,
      startBlock: 13464107,
      options: {
        abi: 'tokenGateway',
        address: '0x41867Dd678E3055649c04Fa10180ba90229cAd9F',
      },
      assets: new Map([
        ['tokenGateway', { file: './abis/TokenGateway.abi.json' }],
      ]),
      mapping: {
        file: './dist/index.js',
        handlers: [
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleBidPlacedEvent',
            filter: {
              topics: ['BidPlaced(bytes32,bytes32,uint256,address)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleBidRefundedEvent',
            filter: {
              topics: ['BidRefunded(bytes32,bytes32,address)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleRequestFulfilledEvent',
            filter: {
              topics: ['RequestFulfilled(address,uint256,bytes32)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleAssetReceivedEvent',
            filter: {
              topics: ['AssetReceived(bytes32,address,address,uint256,bytes32)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleAssetTeleportedEvent',
            filter: {
              topics: ['AssetTeleported(bytes32,address,bytes32,uint256,bytes32,bool)'],
            },
          },
        ],
      },
    },
  ],
  repository: 'https://github.com/polytope-labs/hyperbridge',
};

export default project;