import NodeCache from 'node-cache';
import { config } from '../config';

export const cache = new NodeCache({
  stdTTL: config.cache.ttl,
  checkperiod: 60,
});
