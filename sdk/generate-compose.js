#!/usr/bin/env node

require('dotenv').config();

const fs = require('fs');
const currentEnv = process.env.CURRENT_ENV || 'test';
const configs = require(`./chain-configs-${currentEnv}.json`);

const SUBSTRATE_IMAGE = 'subquerynetwork/subql-node-substrate:latest';
const EVM_IMAGE = 'subquerynetwork/subql-node-ethereum:v5.4.0';
const GRAPHQL_IMAGE = 'subquerynetwork/subql-query:v2.9.0';

const generateNodeServices = () => {
 return Object.entries(configs)
  .filter(([chain]) => {
   const envKey = chain.replace(/-/g, '_').toUpperCase();
   return !!process.env[envKey];
  })
  .map(([chain, config]) => {
   const image = config.type === 'substrate' ? SUBSTRATE_IMAGE : EVM_IMAGE;
   return `
  subquery-node-${chain}:
    image: ${image}
    restart: unless-stopped
    env_file:
      - .env
    environment:
      DB_USER: \${DB_USER}
      DB_PASS: \${DB_PASS}
      DB_DATABASE: \${DB_DATABASE}
      DB_HOST: \${DB_HOST}
      DB_PORT: \${DB_PORT}

    volumes:
      - ./:/app
    command:
      - \${SUB_COMMAND:-}
      - -f=/app/${chain}.yaml
      - --db-schema=app
      - --workers=\${SUBQL_WORKERS:-2}
      - --batch-size=\${SUBQL_BATCH_SIZE:-${
       config.type === 'substrate' ? '5' : '3'
      }}
      - --multi-chain
      - --unsafe
      - --log-level=info
      - --historical=false${
       config.type === 'evm' ? '\n      - --block-confirmations=5' : ''
      }
    healthcheck:
      test: ['CMD', 'curl', '-f', 'http://subquery-node-${chain}:3000/ready']
      interval: 3s
      timeout: 5s
      retries: 10`;
  })
  .join('\n');
};

const generateDependencies = () => {
 return Object.keys(configs)
  .filter(([chain]) => {
   const envKey = chain.replace(/-/g, '_').toUpperCase();
   return !!process.env[envKey];
  })
  .map(
   (chain) => `      'subquery-node-${chain}':
        condition: service_healthy`
  )
  .join('\n');
};

const dockerCompose = `services:
${generateNodeServices()}

  graphql-engine:
    image: ${GRAPHQL_IMAGE}
    ports:
      - 3000:3000
${generateDependencies()}
    restart: always
    env_file:
      - .env
    environment:
      DB_USER: \${DB_USER}
      DB_PASS: \${DB_PASS}
      DB_DATABASE: \${DB_DATABASE}
      DB_HOST: \${DB_HOST}
      DB_PORT: \${DB_PORT}
    command:
      - --name=app
      - --playground

volumes:
  postgres_data:`;

fs.writeFileSync('docker-compose.yml', dockerCompose);
console.log('Generated docker-compose.yml');
