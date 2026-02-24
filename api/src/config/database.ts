import { Pool } from 'pg';
import { config } from '../config';
import { logger } from '../utils/logger';

export const pool = new Pool({
  connectionString: config.database.url,
});

export const connectDatabase = async (): Promise<void> => {
  try {
    const client = await pool.connect();
    logger.info('Database connected successfully');
    client.release();
  } catch (error) {
    logger.error('Database connection failed:', error);
    throw error;
  }
};

export const queryDatabase = async (text: string, params?: any[]) => {
  const start = Date.now();
  const res = await pool.query(text, params);
  const duration = Date.now() - start;
  logger.debug({ text, duration, rows: res.rowCount }, 'Executed query');
  return res;
};
