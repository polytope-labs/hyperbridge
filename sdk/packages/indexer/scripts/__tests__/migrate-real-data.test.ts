import { Pool } from 'pg';
import * as dotenv from 'dotenv';
import * as path from 'path';
import * as fs from 'fs';
import { migrateEntities, MigrationResult } from '../migrate-entity-data';

// Load environment variables
const root = process.cwd();
const env = process.env.ENV || 'local';
dotenv.config({ path: path.resolve(root, `../../.env.${env}`) });

// Database configuration
const DB_CONFIG = {
  user: process.env.DB_USER || 'postgres',
  host: process.env.DB_HOST || 'localhost',
  database: process.env.DB_DATABASE || 'postgres',
  password: process.env.DB_PASS || 'postgres',
  port: parseInt(process.env.DB_PORT || '5432'),
};

const DB_SCHEMA = process.env.DB_SCHEMA || 'app';

// Entities to migrate (from schema.graphql)
const ENTITIES_TO_MIGRATE = [
  'RequestV2',
  'ResponseV2',
  'GetRequestV2',
  'AssetTeleportedV2',
  'TokenGatewayAssetTeleportedV2',
  'RelayerV2',
  'RelayerStatsPerChainV2',
];

// GraphQL to PostgreSQL type mapping
const TYPE_MAPPING: Record<string, string> = {
  'ID': 'TEXT PRIMARY KEY',
  'String': 'TEXT',
  'BigInt': 'NUMERIC',
  'Int': 'INTEGER',
  'Boolean': 'BOOLEAN',
  'Float': 'DOUBLE PRECISION',
  'Date': 'TIMESTAMP WITH TIME ZONE',
  'Bytes': 'BYTEA',
};

interface GraphQLField {
  name: string;
  type: string;
  required: boolean;
  isArray: boolean;
  isDerived: boolean;
}

interface GraphQLEntity {
  name: string;
  fields: GraphQLField[];
}

/**
 * Parse GraphQL schema file and extract V2 entity definitions
 */
function parseGraphQLSchema(schemaPath: string): Map<string, GraphQLEntity> {
  const schemaContent = fs.readFileSync(schemaPath, 'utf-8');
  const entities = new Map<string, GraphQLEntity>();

  // Split into type definitions
  // Updated regex to allow directives between @entity and {
  const typeRegex = /type\s+(\w+)\s*@entity[^{]*\{([^}]+)\}/g;
  let match;

  while ((match = typeRegex.exec(schemaContent)) !== null) {
    const typeName = match[1];
    const fieldsBlock = match[2];

    // Parse fields
    const fields: GraphQLField[] = [];
    const fieldLines = fieldsBlock.split('\n').map(line => line.trim()).filter(line => line && !line.startsWith('#'));

    for (const line of fieldLines) {
      // Skip comments and directives
      if (line.startsWith('#') || line.startsWith('@') || !line.includes(':')) {
        continue;
      }

      // Parse field: name: Type! @directives
      const fieldMatch = line.match(/^(\w+):\s*(\w+)(!)?(\[\])?\s*(.*)/);
      if (fieldMatch) {
        const fieldName = fieldMatch[1];
        const fieldType = fieldMatch[2];
        const isRequired = fieldMatch[3] === '!';
        const isArray = line.includes('[');
        const isDerived = line.includes('@derivedFrom');

        // Skip derived fields (they're virtual)
        if (isDerived) {
          continue;
        }

        fields.push({
          name: fieldName,
          type: fieldType,
          required: isRequired,
          isArray: isArray,
          isDerived: isDerived,
        });
      }
    }

    if (fields.length > 0) {
      entities.set(typeName, { name: typeName, fields });
    }
  }

  return entities;
}

/**
 * Convert GraphQL type to PostgreSQL type
 */
function graphqlToPostgresType(graphqlType: string, isArray: boolean): string {
  if (isArray) {
    return 'JSONB';
  }

  const mapped = TYPE_MAPPING[graphqlType];
  return mapped || 'TEXT';
}

/**
 * Convert a PascalCase/camelCase name to snake_case,
 * matching how SubQuery maps entity names to table names
 * and field names to column names.
 */
function toSnakeCase(name: string): string {
  return name
    .replace(/([A-Z])/g, '_$1')
    .toLowerCase()
    .replace(/^_/, '');
}

/**
 * Get SubQuery table name (snake_case with plural suffix)
 * SubQuery creates tables with plural names (e.g., requests, responses)
 */
function getSubqueryTableName(entityName: string): string {
  return toSnakeCase(entityName) + 's';
}

/**
 * Generate CREATE TABLE statement from GraphQL entity
 */
function generateCreateTable(entity: GraphQLEntity, schema: string): string {
  const columnDefs: string[] = [];

  for (const field of entity.fields) {
    const pgType = graphqlToPostgresType(field.type, field.isArray);
    const columnName = toSnakeCase(field.name);
    let columnDef = `"${columnName}" ${pgType}`;

    columnDefs.push(columnDef);
  }

  const tableName = getSubqueryTableName(entity.name);
  return `
    CREATE TABLE IF NOT EXISTS ${schema}.${tableName} (
      ${columnDefs.join(',\n      ')}
    );
  `;
}

/**
 * Create V2 tables from GraphQL schema
 */
async function createV2TablesFromSchema(
  pool: Pool,
  schema: string,
  schemaPath: string
): Promise<void> {
  console.log('📖 Parsing GraphQL schema...');
  const entities = parseGraphQLSchema(schemaPath);

  const client = await pool.connect();

  try {
    for (const entityName of ENTITIES_TO_MIGRATE) {
      const entity = entities.get(entityName);

      if (!entity) {
        console.log(`⚠️  Entity ${entityName} not found in schema, skipping`);
        continue;
      }

      // Check if table already exists
      const tableName = getSubqueryTableName(entityName);
      const exists = await tableExists(pool, schema, tableName);
      if (exists) {
        console.log(`✓ Table ${schema}.${tableName} already exists`);
        continue;
      }

      // Generate and execute CREATE TABLE
      const createSQL = generateCreateTable(entity, schema);
      await client.query(createSQL);
      console.log(`✓ Created table ${schema}.${tableName} from GraphQL schema`);
    }
  } finally {
    client.release();
  }
}

/**
 * Check if a table exists
 */
async function tableExists(
  pool: Pool,
  schema: string,
  tableName: string
): Promise<boolean> {
  const client = await pool.connect();
  try {
    const result = await client.query(
      `SELECT EXISTS (
        SELECT 1
        FROM information_schema.tables
        WHERE table_schema = $1 AND table_name = $2
      )`,
      [schema, tableName]
    );
    return result.rows[0].exists;
  } finally {
    client.release();
  }
}

/**
 * Get row count for a table
 */
async function getRowCount(
  pool: Pool,
  schema: string,
  tableName: string
): Promise<number> {
  const client = await pool.connect();
  try {
    const result = await client.query(
      `SELECT COUNT(*) as count FROM ${schema}.${tableName}`
    );
    return parseInt(result.rows[0].count);
  } finally {
    client.release();
  }
}

/**
 * Get column information for a table
 */
async function getTableColumns(
  pool: Pool,
  schema: string,
  tableName: string
): Promise<{ column_name: string; data_type: string }[]> {
  const client = await pool.connect();
  try {
    const result = await client.query(
      `SELECT column_name, data_type
       FROM information_schema.columns
       WHERE table_schema = $1 AND table_name = $2
       ORDER BY ordinal_position`,
      [schema, tableName]
    );
    return result.rows;
  } finally {
    client.release();
  }
}

describe('Real Data Migration from GraphQL Schema', () => {
  let pool: Pool;

  beforeAll(async () => {
    pool = new Pool(DB_CONFIG);

    // Test database connection
    const client = await pool.connect();
    const result = await client.query('SELECT NOW()');
    console.log(`\n✓ Connected to database at ${result.rows[0].now}`);
    console.log(`  Database: ${DB_CONFIG.database}`);
    console.log(`  Schema: ${DB_SCHEMA}`);
    console.log(`  Environment: ${env}\n`);
    client.release();
  });

  afterAll(async () => {
    await pool.end();
  });

  test('should migrate real data from local database', async () => {
    // Path to GraphQL schema
    const schemaPath = path.join(__dirname, '../../src/configs/schema.graphql');
    console.log(`📄 GraphQL schema path: ${schemaPath}`);

    // Create V2 tables from GraphQL schema
    await createV2TablesFromSchema(pool, DB_SCHEMA, schemaPath);

    console.log('\n🔄 Running migration for', ENTITIES_TO_MIGRATE.length, 'entities...\n');

    // Run migration
    const results = await migrateEntities({
      entities: ENTITIES_TO_MIGRATE,
      pool,
      schema: DB_SCHEMA,
      logger: console,
      limit: 3000,
      dropSourceTables: true,
    });

    console.log('\n📊 Migration Results:');
    console.table(results.map(r => ({
      Entity: r.entity,
      'Source Table': r.sourceTable,
      'Dest Table': r.destTable,
      'Rows Copied': r.copiedRows,
      'Skipped Columns': r.skippedColumns.join(', '),
      Success: r.success ? '✓' : '✗',
      Error: r.error || '-',
    })));

    // Verify results
    console.log('\n🔍 Verifying migration results...\n');

    const successCount = results.filter(r => r.success).length;
    const failCount = results.filter(r => !r.success).length;

    console.log(`✅ Successful migrations: ${successCount}`);
    if (failCount > 0) {
      console.log(`❌ Failed migrations: ${failCount}`);
    }

    // Show row counts for successful migrations
    console.log('\n📈 Row Counts:');
    for (const result of results) {
      if (result.success) {
        const destCount = await getRowCount(pool, DB_SCHEMA, result.destTable);

        console.log(`  ${result.entity}:`);
        console.log(`    Destination (${result.destTable}): ${destCount} rows`);

      }
    }

    // Assertions
    expect(results.length).toBe(ENTITIES_TO_MIGRATE.length);
    expect(successCount).toBeGreaterThan(0);

    // Verify successful migrations have data in destination tables
    // Note: Destination count may be less than copiedRows due to duplicate IDs with ON CONFLICT DO NOTHING
    for (const result of results) {
      if (result.success && result.copiedRows > 0) {
        const destCount = await getRowCount(pool, DB_SCHEMA, result.destTable);
        expect(destCount).toBeGreaterThan(0);
        expect(destCount).toBeLessThanOrEqual(result.copiedRows);
      }
    }

    // Verify source tables were dropped after successful migration
    console.log('\n🔍 Verifying source tables were dropped...\n');
    for (const result of results) {
      if (result.success && result.sourceTable) {
        const sourceExists = await tableExists(pool, DB_SCHEMA, result.sourceTable);
        console.log(`  ${result.sourceTable}: ${sourceExists ? '❌ Still exists' : '✓ Dropped'}`);
        expect(sourceExists).toBe(false);
      }
    }
  });
});
