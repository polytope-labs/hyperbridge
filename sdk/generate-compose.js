#!/usr/bin/env node

const fs = require('fs');
const configs = require('./chain-configs.json');

const SUBSTRATE_IMAGE = 'subquerynetwork/subql-node-substrate:latest';
const EVM_IMAGE = 'subquerynetwork/subql-node-ethereum:v5.4.0';
const GRAPHQL_IMAGE = 'subquerynetwork/subql-query:v2.9.0';

const generateNodeServices = () => {
 return Object.entries(configs)
  .map(([chain, config]) => {
   const image = config.type === 'substrate' ? SUBSTRATE_IMAGE : EVM_IMAGE;
   return `
  subquery-node-${chain}:
    image: ${image}
    depends_on:
      'postgres':
        condition: service_healthy
    restart: unless-stopped
    environment:
      DB_USER: postgres
      DB_PASS: postgres
      DB_DATABASE: postgres
      DB_HOST: postgres
      DB_PORT: 5432
    volumes:
      - ./:/app
    command:
      - \${SUB_COMMAND:-}
      - -f=/app/${chain}.yaml
      - --db-schema=app
      - --workers=\${SUBQL_WORKERS:-1}
      - --batch-size=\${SUBQL_BATCH_SIZE:-${
       config.type === 'substrate' ? '5' : '3'
      }}
      - --multi-chain
      - --unsafe
      - --log-level=info
      - --historical=false${
       config.type === 'evm' ? '\n      - --block-confirmations=10' : ''
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
  .map(
   (chain) => `      'subquery-node-${chain}':
        condition: service_healthy`
  )
  .join('\n');
};

const dockerCompose = `version: '3'

services:
  postgres:
    build:
      context: .
      dockerfile: ./docker/pg-Dockerfile
    ports:
      - 5432:5432
    volumes:
      - postgres_data:/var/lib/postgresql/data
    environment:
      POSTGRES_PASSWORD: postgres
    healthcheck:
      test: ['CMD-SHELL', 'pg_isready -U postgres']
      interval: 5s
      timeout: 5s
      retries: 5
${generateNodeServices()}

  graphql-engine:
    image: ${GRAPHQL_IMAGE}
    ports:
      - 3000:3000
    depends_on:
      'postgres':
        condition: service_healthy
${generateDependencies()}
    restart: always
    environment:
      DB_USER: postgres
      DB_PASS: postgres
      DB_DATABASE: postgres
      DB_HOST: postgres
      DB_PORT: 5432
    command:
      - --name=app
      - --playground

volumes:
  postgres_data:`;

fs.writeFileSync('docker-compose.yml', dockerCompose);
console.log('Generated docker-compose.yml');
