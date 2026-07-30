/**
 * Legacy barrel — re-exports everything from the admin client.
 *
 * Existing imports of `../lib/api/client` continue to work unchanged.
 * For new code, prefer importing from the appropriate module directly:
 *
 *   Public pages  →  import { api } from './public-client'
 *   Admin pages   →  import { api } from './admin-client'
 */
export {
  api,
  ApiError,
  CacheTag,
  CONTRACT_ERROR_MESSAGES,
  getContractErrorMessage,
} from './admin-client';
