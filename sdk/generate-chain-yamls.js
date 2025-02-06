#!/usr/bin/env node
require('dotenv').config();

const fs = require('fs');
const currentEnv = process.env.CURRENT_ENV || 'test';
const configs = require(`./chain-configs-${currentEnv}.json`);

const getChainTypesPath = (chain) => {
 // Extract base chain name before the hyphen
 const baseChainName = chain.split('-')[0];

 const chainTypesMap = {
  hyperbridge: './dist/substrate-chaintypes/hyperbridge.js',
  bifrost: './dist/substrate-chaintypes/bifrost.js',
 };

 return chainTypesMap[baseChainName.toLowerCase()] || null;
};

// Generate chain-specific YAML files
const generateSubstrateYaml = (chain, config) => {
 const chainTypesConfig = getChainTypesPath(chain);
 const envKey = chain.replace(/-/g, '_').toUpperCase();
 const endpoint = process.env[envKey];
 const endpoints = `    - '${endpoint}'`;

 const chainTypesSection = chainTypesConfig
  ? `\n  chaintypes:\n    file: ${chainTypesConfig}`
  : '';

 return `# // Auto-generated , DO NOT EDIT
specVersion: 1.0.0
version: 0.0.1
name: ${chain}-chain
description: ${chain.charAt(0).toUpperCase() + chain.slice(1)} Chain Indexer
runner:
  node:
    name: '@subql/node'
    version: '>=4.0.0'
  query:
    name: '@subql/query'
    version: '*'
schema:
  file: ./schema.graphql
network:
  chainId: '${config.chainId}'
  endpoint:
${endpoints}${chainTypesSection}
dataSources:
  - kind: substrate/Runtime
    startBlock: ${config.startBlock}
    mapping:
      file: ./dist/index.js
      handlers:
        - handler: handleIsmpStateMachineUpdatedEvent
          kind: substrate/EventHandler
          filter:
            module: ismp
            method: StateMachineUpdated
        - handler: handleSubstrateRequestEvent
          kind: substrate/EventHandler
          filter:
            module: ismp
            method: Request
        - handler: handleSubstrateResponseEvent
          kind: substrate/EventHandler
          filter:
            module: ismp
            method: Response
        - handler: handleSubstratePostRequestHandledEvent
          kind: substrate/EventHandler
          filter:
            module: ismp
            method: PostRequestHandled
        - handler: handleSubstratePostResponseHandledEvent
          kind: substrate/EventHandler
          filter:
            module: ismp
            method: PostResponseHandled
        - handler: handleSubstratePostRequestTimeoutHandledEvent
          kind: substrate/EventHandler
          filter:
            module: ismp
            method: PostRequestTimeoutHandled
        - handler: handleSubstratePostResponseTimeoutHandledEvent
          kind: substrate/EventHandler
          filter:
            module: ismp
            method: PostResponseTimeoutHandled
            
repository: 'https://github.com/polytope-labs/hyperbridge'`;
};

const generateEvmYaml = (chain, config) => {
 const envKey = chain.replace(/-/g, '_').toUpperCase();
 const endpoint = process.env[envKey];
 const endpoints = `    - '${endpoint}'`;

 return `# // Auto-generated , DO NOT EDIT
specVersion: 1.0.0
version: 0.0.1
name: ${chain}
description: ${chain.charAt(0).toUpperCase() + chain.slice(1)} Indexer
runner:
  node:
    name: '@subql/node-ethereum'
    version: '>=3.0.0'
  query:
    name: '@subql/query'
    version: '*'
schema:
  file: ./schema.graphql
network:
  chainId: '${config.chainId}'
  endpoint:
${endpoints}
dataSources:
  - kind: ethereum/Runtime
    startBlock: ${config.startBlock}
    options:
      abi: ethereumHost
      address: '${config.contracts.ethereumHost}'
    assets:
      ethereumHost:
        file: ./abis/EthereumHost.abi.json
      chainLinkAggregatorV3:
        file: ./abis/ChainLinkAggregatorV3.abi.json
    mapping:
      file: ./dist/index.js
      handlers:
        - kind: ethereum/LogHandler
          handler: handlePostRequestEvent
          filter:
            topics:
              - 'PostRequestEvent(string,string,address,bytes,uint256,uint256,bytes,uint256)'
        - kind: ethereum/LogHandler
          handler: handlePostResponseEvent
          filter:
            topics:
              - 'PostResponseEvent(string,string,address,bytes,uint256,uint256,bytes,bytes,uint256,uint256)'
        - kind: ethereum/LogHandler
          handler: handlePostRequestHandledEvent
          filter:
            topics:
              - 'PostRequestHandled(bytes32,address)'
        - kind: ethereum/LogHandler
          handler: handlePostResponseHandledEvent
          filter:
            topics:
              - 'PostResponseHandled(bytes32,address)'
        - kind: ethereum/LogHandler
          handler: handlePostRequestTimeoutHandledEvent
          filter:
            topics:
              - 'PostRequestTimeoutHandled(bytes32,string)'
        - kind: ethereum/LogHandler
          handler: handlePostResponseTimeoutHandledEvent
          filter:
            topics:
              - 'PostResponseTimeoutHandled(bytes32,string)'
  - kind: ethereum/Runtime
    startBlock: ${config.startBlock}
    options:
      abi: erc6160ext20
      address: '${config.contracts.erc6160ext20}'
    assets:
      erc6160ext20:
        file: ./abis/ERC6160Ext20.abi.json
    mapping:
      file: ./dist/index.js
      handlers:
        - kind: ethereum/LogHandler
          handler: handleTransferEvent
          filter:
            topics:
              - 'Transfer(address indexed from, address indexed to, uint256 amount)'
  # - kind: ethereum/Runtime
  #   startBlock: 21535312
  #   options:
  #     abi: handlerV1
  #     address: '0xA801da100bF16D07F668F4A49E1f71fc54D05177'
  #   assets:
  #     handlerV1:
  #       file: ./abis/HandlerV1.abi.json
  #   mapping:
  #     file: ./dist/index.js
  #     handlers:
  #       - handler: handlePostRequestTransactionHandler
  #         kind: ethereum/TransactionHandler
  #         function: >-
  #           handlePostRequests(address,(((uint256,uint256),bytes32[],uint256),((bytes,bytes,uint64,bytes,bytes,uint64,bytes),uint256,uint256)[]))
  #       - handler: handlePostResponseTransactionHandler
  #         kind: ethereum/TransactionHandler
  #         function: >-
  #           handlePostResponses(address,(((uint256,uint256),bytes32[],uint256),(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64),uint256,uint256)[]))

repository: 'https://github.com/polytope-labs/hyperbridge'`;
};

const generateMultichainYaml = () => {
 const projects = Object.keys(configs)
  .map((chain) => `  - ./${chain}.yaml`)
  .join('\n');

 const yaml = `specVersion: 1.0.0
query:
  name: '@subql/query'
  version: '*'
projects:
${projects}`;

 fs.writeFileSync('subquery-multichain.yaml', yaml);
 console.log('Generated subquery-multichain.yaml');
};

Object.entries(configs).forEach(([chain, config]) => {
 const yaml =
  config.type === 'substrate'
   ? generateSubstrateYaml(chain, config)
   : generateEvmYaml(chain, config);

 fs.writeFileSync(`${chain}.yaml`, yaml);
 console.log(`Generated ${chain}.yaml`);
});

generateMultichainYaml();
