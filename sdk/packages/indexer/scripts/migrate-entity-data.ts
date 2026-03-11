#!/usr/bin/env node
import pg from 'pg';
import * as path from 'path';
import * as dotenv from 'dotenv';
import { getEnv } from '../src/configs';

const { Pool } = pg;

// Export types
export interface MigrationOptions {
  entities: string[];
  pool: pg.Pool;
  schema: string;
  logger?: {
    log: (...args: any[]) => void;
    error: (...args: any[]) => void;
  };
  limit?: number;
  dropSourceTables?: boolean;
}

export interface MigrationResult {
  entity: string;
  sourceTable: string;
  destTable: string;
  copiedRows: number;
  skippedColumns: string[];
  success: boolean;
  error?: string;
}

/**
 * Convert entity name to table name (snake_case)
 */
function entityToTableName(entityName: string): string {
  return entityName
    .replace(/([A-Z])/g, '_$1')
    .toLowerCase()
    .replace(/^_/, '');
}

/**
 * Get SubQuery table name (snake_case with plural suffix)
 * SubQuery creates tables with plural names (e.g., requests, responses)
 */
function getSubqueryTableName(entityName: string): string {
  const snakeCase = entityToTableName(entityName);
  return snakeCase + 's';
}

/**
 * Remove version suffix from entity name (V2, V3, etc.)
 */
function removeVersionSuffix(entityName: string): string {
  return entityName.replace(/V\d+$/, '');
}

/**
 * Extract version suffix from entity name
 */
function extractVersionSuffix(entityName: string): string | null {
  const match = entityName.match(/(V\d+)$/);
  return match ? match[1] : null;
}

/**
 * Get column names for a table
 */
async function getTableColumns(
  pool: pg.Pool,
  schema: string,
  tableName: string
): Promise<string[]> {
  const client = await pool.connect();
  try {
    const result = await client.query(
      `SELECT column_name
       FROM information_schema.columns
       WHERE table_schema = $1 AND table_name = $2
       ORDER BY ordinal_position`,
      [schema, tableName]
    );
    return result.rows.map(row => row.column_name);
  } finally {
    client.release();
  }
}

/**
 * Check if a table exists
 */
async function tableExists(
  pool: pg.Pool,
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
  pool: pg.Pool,
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
 * Drop a table
 */
async function dropTable(
  pool: pg.Pool,
  schema: string,
  tableName: string
): Promise<boolean> {
  const client = await pool.connect();
  try {
    await client.query(`DROP TABLE IF EXISTS ${schema}.${tableName} CASCADE`);
    return true;
  } catch (error) {
    throw error;
  } finally {
    client.release();
  }
}

/**
 * Copy data from source table to destination table
 */
async function copyTableData(
  pool: pg.Pool,
  schema: string,
  sourceTable: string,
  destTable: string,
  logger: MigrationOptions['logger'],
  batchSize: number = 1000,
  limit?: number,
  whereClause?: string
): Promise<{ copiedRows: number; skippedColumns: string[] }> {
  const client = await pool.connect();
  
  try {
    // Get columns for both tables
    const sourceColumns = await getTableColumns(pool, schema, sourceTable);
    const destColumns = await getTableColumns(pool, schema, destTable);
    
    // Find common columns (only columns that exist in destination)
    const commonColumns = sourceColumns.filter(col => destColumns.includes(col));
    const skippedColumns = sourceColumns.filter(col => !destColumns.includes(col));
    
    if (commonColumns.length === 0) {
      throw new Error(`No common columns found between ${sourceTable} and ${destTable}`);
    }
    
    logger?.log(`  Common columns (${commonColumns.length}): ${commonColumns.join(', ')}`);
    if (skippedColumns.length > 0) {
      logger?.log(`  Skipping columns (${skippedColumns.length}): ${skippedColumns.join(', ')}`);
    }
    
    // Get total row count (with where clause filter)
    const whereFilter = whereClause ? ` WHERE ${whereClause}` : '';
    const countResult = await client.query(
      `SELECT COUNT(*) as count FROM ${schema}.${sourceTable}${whereFilter}`
    );
    const totalRows = parseInt(countResult.rows[0].count);
    const rowsToProcess = limit ? Math.min(totalRows, limit) : totalRows;
    logger?.log(`  Total rows to copy: ${rowsToProcess}${limit ? ` (limited from ${totalRows})` : ''}${whereClause ? ` (filtered: ${whereClause})` : ''}`);
    
    if (rowsToProcess === 0) {
      logger?.log(`  No rows to copy from ${sourceTable}`);
      return { copiedRows: 0, skippedColumns };
    }
    
    // Copy data in batches using LIMIT/OFFSET for memory efficiency
    let copiedRows = 0;
    let skippedDuplicates = 0;
    const columnsList = commonColumns.map(col => `"${col}"`).join(', ');
    const placeholders = commonColumns.map((_, i) => `$${i + 1}`).join(', ');
    const idParamIndex = commonColumns.indexOf('id') + 1;
    
    // Order by _block_range DESC so the most recent version of each row is inserted first,
    // and older versions are skipped by the WHERE NOT EXISTS check
    const hasBlockRange = sourceColumns.includes('_block_range');
    const orderClause = hasBlockRange ? 'ORDER BY id, _block_range DESC' : 'ORDER BY id';
    if (hasBlockRange) {
      logger?.log(`  Using _block_range DESC ordering to pick latest version per row`);
    }
    
    await client.query('BEGIN');
    
    logger?.log(`  Processing ${rowsToProcess} rows in batches of ${batchSize}...`);
    
    // Fetch and process rows in batches using LIMIT/OFFSET
    let offset = 0;
    let processedInBatch = 0;
    
    while (offset < rowsToProcess) {
      // Fetch a batch of rows from source table
      const batchResult = await client.query(
        `SELECT ${columnsList} FROM ${schema}.${sourceTable}${whereFilter} ${orderClause} LIMIT $1 OFFSET $2`,
        [batchSize, offset]
      );
      
      if (batchResult.rows.length === 0) {
        break; // No more rows to process
      }
      
      // Insert each row, using WHERE NOT EXISTS to skip duplicates (no unique constraint needed)
      for (const row of batchResult.rows) {
        const values = commonColumns.map(col => {
          const v = row[col];
          // pg deserializes jsonb into JS objects/arrays, but re-serializes JS arrays
          // as PostgreSQL array literals instead of JSON. Stringify them so they're
          // sent as valid JSON for jsonb columns.
          if (v !== null && typeof v === 'object') return JSON.stringify(v);
          return v;
        });
        
        const insertResult = await client.query(
          `INSERT INTO ${schema}.${destTable} (${columnsList})
           SELECT ${placeholders}
           WHERE NOT EXISTS (
             SELECT 1 FROM ${schema}.${destTable} WHERE id = $${idParamIndex}
           )`,
          values
        );
        if (insertResult.rowCount && insertResult.rowCount > 0) {
          copiedRows++;
        } else {
          skippedDuplicates++;
        }
      }
      
      processedInBatch += batchResult.rows.length;
      offset += batchSize;
      
      // Progress update
      if (processedInBatch % 1000 === 0 || offset >= rowsToProcess) {
        const progress = Math.min(offset, rowsToProcess);
        logger?.log(`  Progress: ${progress}/${rowsToProcess} rows processed`);
      }
    }
    
    await client.query('COMMIT');
    
    logger?.log(`  ✓ Successfully copied ${copiedRows} rows${skippedDuplicates > 0 ? `, ${skippedDuplicates} duplicates skipped` : ''}`);
    
    return { copiedRows, skippedColumns };
  } catch (error) {
    await client.query('ROLLBACK');
    throw error;
  } finally {
    client.release();
  }
}

/**
 * Migrate a single entity
 */
async function migrateEntity(
  pool: pg.Pool,
  schema: string,
  versionedEntity: string,
  logger: MigrationOptions['logger'],
  limit?: number,
  whereClause?: string
): Promise<MigrationResult> {
  const suffix = extractVersionSuffix(versionedEntity);
  
  if (!suffix) {
    logger?.log(`⚠️  Skipping ${versionedEntity} - no version suffix found`);
    return {
      entity: versionedEntity,
      sourceTable: '',
      destTable: '',
      copiedRows: 0,
      skippedColumns: [],
      success: false,
      error: 'No version suffix found',
    };
  }
  
  const baseEntity = removeVersionSuffix(versionedEntity);
  const sourceTable = getSubqueryTableName(baseEntity);
  const destTable = getSubqueryTableName(versionedEntity);
  
  logger?.log(`\n📋 Processing: ${baseEntity} → ${versionedEntity}`);
  logger?.log(`  Source table: ${sourceTable}`);
  logger?.log(`  Destination table: ${destTable}`);
  
  // Check if source table exists
  const sourceExists = await tableExists(pool, schema, sourceTable);
  if (!sourceExists) {
    const msg = `Source table ${sourceTable} does not exist`;
    logger?.log(`  ⚠️  ${msg}, skipping`);
    return {
      entity: versionedEntity,
      sourceTable,
      destTable,
      copiedRows: 0,
      skippedColumns: [],
      success: false,
      error: msg,
    };
  }
  
  // Check if destination table exists
  const destExists = await tableExists(pool, schema, destTable);
  if (!destExists) {
    const msg = `Destination table ${destTable} does not exist`;
    logger?.log(`  ⚠️  ${msg}, skipping`);
    return {
      entity: versionedEntity,
      sourceTable,
      destTable,
      copiedRows: 0,
      skippedColumns: [],
      success: false,
      error: msg,
    };
  }
  
  try {
    const result = await copyTableData(pool, schema, sourceTable, destTable, logger, 1000, limit, whereClause);
    logger?.log(`  ✅ Migration completed for ${versionedEntity}`);
    logger?.log(`     Copied: ${result.copiedRows} rows`);
    if (result.skippedColumns.length > 0) {
      logger?.log(`     Skipped columns: ${result.skippedColumns.join(', ')}`);
    }
    
    return {
      entity: versionedEntity,
      sourceTable,
      destTable,
      copiedRows: result.copiedRows,
      skippedColumns: result.skippedColumns,
      success: true,
    };
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : 'Unknown error';
    logger?.error(`  ❌ Migration failed for ${versionedEntity}:`, error);
    return {
      entity: versionedEntity,
      sourceTable,
      destTable,
      copiedRows: 0,
      skippedColumns: [],
      success: false,
      error: errorMsg,
    };
  }
}

/**
 * Main migration function for programmatic use
 */
export async function migrateEntities(options: MigrationOptions): Promise<MigrationResult[]> {
  const { entities, pool, schema, logger = console, limit } = options;
  
  if (entities.length === 0) {
    throw new Error('No entities provided for migration');
  }
  
  logger.log('🔄 Starting entity data migration...\n');
  logger.log(`Schema: ${schema}`);
  logger.log(`Entities to migrate: ${entities.join(', ')}`);
  if (limit) {
    logger.log(`Row limit: ${limit} per entity`);
  }
  logger.log('');
  
  const results: MigrationResult[] = [];
  
  try {
    // Test database connection
    const client = await pool.connect();
    const result = await client.query('SELECT NOW()');
    logger.log(`✓ Connected to database at ${result.rows[0].now}\n`);
    client.release();
    
    // where clauses for filtering during migration
    const entityWhereClause: Record<string, string> = {
      'RelayerStatsPerChainV2': "chain LIKE 'EVM-%'",
    };

    // Process each entity
    for (const entity of entities) {
      const whereClause = entityWhereClause[entity];
      const result = await migrateEntity(pool, schema, entity, logger, limit, whereClause);
      results.push(result);
    }
    
    const successCount = results.filter(r => r.success).length;
    const failCount = results.filter(r => !r.success).length;
    
    logger.log('\n' + '='.repeat(60));
    logger.log(`✅ Migration completed: ${successCount} succeeded, ${failCount} failed`);
    logger.log('='.repeat(60) + '\n');
    
    // Drop source tables if requested and all migrations were successful
    if (options.dropSourceTables && failCount === 0 && successCount > 0) {
      logger.log('🗑️  Dropping source tables...\n');
      
      const droppedTables: string[] = [];
      const failedDrops: string[] = [];
      
      for (const result of results) {
        if (result.success && result.sourceTable) {
          try {
            logger.log(`  Dropping ${result.sourceTable}...`);
            await dropTable(pool, schema, result.sourceTable);
            droppedTables.push(result.sourceTable);
            logger.log(`  ✓ Dropped ${result.sourceTable}`);
          } catch (error) {
            const errorMsg = error instanceof Error ? error.message : 'Unknown error';
            logger.error(`  ⚠️  Failed to drop ${result.sourceTable}: ${errorMsg}`);
            failedDrops.push(result.sourceTable);
          }
        }
      }
      
      logger.log('\n' + '='.repeat(60));
      logger.log(`✅ Dropped ${droppedTables.length} source tables`);
      if (failedDrops.length > 0) {
        logger.log(`⚠️  Failed to drop ${failedDrops.length} tables: ${failedDrops.join(', ')}`);
      }
      logger.log('='.repeat(60) + '\n');
    } else if (options.dropSourceTables) {
      logger.log('⚠️  Skipping source table drop due to migration failures\n');
    }
    
    return results;
  } catch (error) {
    logger.error('\n❌ Migration failed:', error);
    throw error;
  }
}

// CLI interface (only runs when executed directly)
async function runCLI(): Promise<void> {
  // Load environment variables
  const root = process.cwd();
  const env = getEnv();
  dotenv.config({ path: path.resolve(root, `../../.env.${env}`) });

  // Database connection configuration
  const pool = new Pool({
    user: process.env.DB_USER || 'postgres',
    host: process.env.DB_HOST || 'localhost',
    database: process.env.DB_DATABASE || 'postgres',
    password: process.env.DB_PASS || 'postgres',
    port: parseInt(process.env.DB_PORT || '5432'),
  });

  // Schema configuration
  const DB_SCHEMA = process.env.DB_SCHEMA || 'app';
  
  const args = process.argv.slice(2);
  
  // Parse command line arguments
  const dropSourceIndex = args.indexOf('--drop-source');
  const dropSourceTables = dropSourceIndex !== -1;
  const entitiesToMigrate = dropSourceTables ? args.filter(arg => arg !== '--drop-source') : args;
  
  if (entitiesToMigrate.length === 0) {
    console.log('Usage: migrate-entity-data.ts <EntityV2> [EntityV3] ... [--drop-source]');
    console.log('Example: migrate-entity-data.ts RequestV2 ResponseV2 GetRequestV2');
    console.log('  --drop-source: Drop source tables after successful migration');
    process.exit(1);
  }
  
  console.log(`Environment: ${env}`);
  
  try {
    await migrateEntities({
      entities: entitiesToMigrate,
      pool,
      schema: DB_SCHEMA,
      logger: console,
      dropSourceTables,
    });
    await pool.end();
    process.exit(0);
  } catch (error) {
    console.error('Fatal error:', error);
    await pool.end();
    process.exit(1);
  }
}

// Run CLI if executed directly
// Check if this file is being run as the main module
if (process.argv[1] && process.argv[1].includes('migrate-entity-data.ts')) {
  runCLI();
}