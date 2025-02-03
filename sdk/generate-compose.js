#!/usr/bin/env node

const fs = require('fs');
const configs = require('./chain-configs.json');

const SUBSTRATE_IMAGE = 'subquerynetwork/subql-node-substrate:latest';
const EVM_IMAGE = 'subquerynetwork/subql-node-ethereum:v3.11.0';
const GRAPHQL_IMAGE = 'subquerynetwork/subql-query:v2.9.0';

const evmChains = [
 'ethereum-sepolia',
 'base-sepolia',
 'optimism-sepolia',
 'arbitrum-sepolia',
 'bsc-chapel',
];

const generateSubstrateServices = () => {
 return Object.keys(configs)
  .map(
   (chain) => `
  subquery-node-${chain}:
    image: ${SUBSTRATE_IMAGE}
    networks:
      - substrate-net
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
      - --batch-size=\${SUBQL_BATCH_SIZE:-5}
      - --multi-chain
      - --unsafe
      - --log-level=info
    healthcheck:
      test: ['CMD', 'curl', '-f', 'http://subquery-node-${chain}:3000/ready']
      interval: 3s
      timeout: 5s
      retries: 10`
  )
  .join('\n');
};

const generateEvmServices = () => {
 return evmChains
  .map(
   (chain) => `
  subquery-node-${chain}:
    image: ${EVM_IMAGE}
    networks:
      - evm-net
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
      - --batch-size=\${SUBQL_BATCH_SIZE:-3}
      - --multi-chain
      - --unsafe
      - --log-level=info
    healthcheck:
      test: ['CMD', 'curl', '-f', 'http://subquery-node-${chain}:3000/ready']
      interval: 3s
      timeout: 5s
      retries: 10`
  )
  .join('\n');
};

const generateSubstrateDependencies = () => {
 return Object.keys(configs)
  .map(
   (chain) => `      'subquery-node-${chain}':
        condition: service_healthy`
  )
  .join('\n');
};

const generateEvmDependencies = () => {
 return evmChains
  .map(
   (chain) => `      'subquery-node-${chain}':
        condition: service_healthy`
  )
  .join('\n');
};

const dockerCompose = `version: '3'

networks:
  substrate-net:
    driver: bridge
  evm-net:
    driver: bridge

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
    networks:
      - substrate-net
      - evm-net
    healthcheck:
      test: ['CMD-SHELL', 'pg_isready -U postgres']
      interval: 5s
      timeout: 5s
      retries: 5
${generateSubstrateServices()}
${generateEvmServices()}

  graphql-engine:
    image: ${GRAPHQL_IMAGE}
    networks:
      - substrate-net
      - evm-net
    ports:
      - 3000:3000
    depends_on:
      'postgres':
        condition: service_healthy
${generateSubstrateDependencies()}
${generateEvmDependencies()}
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
