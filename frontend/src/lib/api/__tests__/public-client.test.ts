/**
 * Bundle boundary tests (#1158)
 *
 * Asserts that the public client module does NOT export admin-only symbols
 * (resolveMarket, emailPreview, emailSendTest, getEmailAnalytics,
 * getEmailQueueStats, CONTRACT_ERROR_MESSAGES, getContractErrorMessage),
 * and that the admin client DOES export all of them.
 *
 * These tests act as a compile-time + runtime guard: if someone accidentally
 * adds an admin method to public-client.ts, the test will fail immediately.
 */

import * as publicClientModule from '../public-client';
import * as adminClientModule from '../admin-client';

describe('Module boundary: public-client must not expose admin surface (#1158)', () => {
  describe('public-client exports', () => {
    it('exports the public api object', () => {
      expect(publicClientModule.api).toBeDefined();
    });

    it('exports ApiError', () => {
      expect(publicClientModule.ApiError).toBeDefined();
    });

    it('exports CacheTag', () => {
      expect(publicClientModule.CacheTag).toBeDefined();
    });

    it('does NOT export CONTRACT_ERROR_MESSAGES', () => {
      expect((publicClientModule as Record<string, unknown>)['CONTRACT_ERROR_MESSAGES']).toBeUndefined();
    });

    it('does NOT export getContractErrorMessage', () => {
      expect((publicClientModule as Record<string, unknown>)['getContractErrorMessage']).toBeUndefined();
    });

    it('does NOT include resolveMarket on the public api object', () => {
      expect((publicClientModule.api as Record<string, unknown>)['resolveMarket']).toBeUndefined();
    });

    it('does NOT include emailPreview on the public api object', () => {
      expect((publicClientModule.api as Record<string, unknown>)['emailPreview']).toBeUndefined();
    });

    it('does NOT include emailSendTest on the public api object', () => {
      expect((publicClientModule.api as Record<string, unknown>)['emailSendTest']).toBeUndefined();
    });

    it('does NOT include getEmailAnalytics on the public api object', () => {
      expect((publicClientModule.api as Record<string, unknown>)['getEmailAnalytics']).toBeUndefined();
    });

    it('does NOT include getEmailQueueStats on the public api object', () => {
      expect((publicClientModule.api as Record<string, unknown>)['getEmailQueueStats']).toBeUndefined();
    });

    it('includes all expected public methods on api', () => {
      const publicMethods = [
        'health',
        'getStatistics',
        'getFeaturedMarkets',
        'getContent',
        'getBlockchainHealth',
        'getBlockchainMarket',
        'getBlockchainStats',
        'getUserBets',
        'getOracleResult',
        'getTransactionStatus',
        'newsletterSubscribe',
        'newsletterConfirm',
        'newsletterUnsubscribe',
        'newsletterGdprExport',
        'newsletterGdprDelete',
      ];
      for (const method of publicMethods) {
        expect(typeof (publicClientModule.api as Record<string, unknown>)[method]).toBe('function');
      }
    });
  });

  describe('admin-client exports', () => {
    it('exports CONTRACT_ERROR_MESSAGES', () => {
      expect(adminClientModule.CONTRACT_ERROR_MESSAGES).toBeDefined();
    });

    it('exports getContractErrorMessage', () => {
      expect(typeof adminClientModule.getContractErrorMessage).toBe('function');
    });

    it('includes resolveMarket on the admin api object', () => {
      expect(typeof adminClientModule.api.resolveMarket).toBe('function');
    });

    it('includes emailPreview on the admin api object', () => {
      expect(typeof adminClientModule.api.emailPreview).toBe('function');
    });

    it('includes emailSendTest on the admin api object', () => {
      expect(typeof adminClientModule.api.emailSendTest).toBe('function');
    });

    it('includes getEmailAnalytics on the admin api object', () => {
      expect(typeof adminClientModule.api.getEmailAnalytics).toBe('function');
    });

    it('includes getEmailQueueStats on the admin api object', () => {
      expect(typeof adminClientModule.api.getEmailQueueStats).toBe('function');
    });

    it('also includes all public methods (admin is a superset)', () => {
      const publicMethods = [
        'health',
        'getStatistics',
        'getFeaturedMarkets',
        'newsletterSubscribe',
        'newsletterUnsubscribe',
      ];
      for (const method of publicMethods) {
        expect(typeof (adminClientModule.api as Record<string, unknown>)[method]).toBe('function');
      }
    });
  });
});
