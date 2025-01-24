#!/usr/bin/env node

const fs = require('fs');
const configs = require('./chain-configs.json');

// Generate chain-specific YAML files
Object.entries(configs).forEach(([chain, config]) => {
 const endpoints = config.endpoints
  .map((endpoint) => `    - '${endpoint}'`)
  .join('\n');

 const yaml = `# // Auto-generated , DO NOT EDIT
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
${endpoints}
  chaintypes:
    file: ./dist/substrate-chaintypes.js
dataSources:
  - kind: substrate/Runtime
    startBlock: 695
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

repository: 'https://github.com/polytope-labs/hyperbridge'`.trim();

 fs.writeFileSync(`${chain}.yaml`, yaml);
 console.log(`Generated ${chain}.yaml`);
});
