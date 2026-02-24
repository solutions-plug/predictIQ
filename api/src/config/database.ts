import { Pool } from 'pg';
import { config } from './index';

export const pool = new Pool({
  connectionString: config.database.url,
});

export const query = async (text: string, params?: any[]) => {
  return pool.query(text, params);
};
