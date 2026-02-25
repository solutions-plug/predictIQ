import { query } from '../config/database';
import { cache } from '../utils/cache';
import { logger } from '../utils/logger';
import { marked } from 'marked';

export interface ContentData {
  [key: string]: any;
}

export interface ContentVersion {
  id: number;
  section: string;
  content: ContentData;
  version: number;
  created_by: number;
  created_at: Date;
}

export class ContentService {
  private getCacheKey(section: string): string {
    return `content:${section}`;
  }

  async getContent(section: string): Promise<ContentVersion | null> {
    const cacheKey = this.getCacheKey(section);
    const cached = cache.get<ContentVersion>(cacheKey);
    
    if (cached) {
      logger.debug(`Cache hit for section: ${section}`);
      return cached;
    }

    const result = await query(
      `SELECT id, section, content, version, created_by, created_at 
       FROM content 
       WHERE section = $1 AND is_active = true 
       ORDER BY version DESC LIMIT 1`,
      [section]
    );

    if (result.rows.length === 0) {
      return null;
    }

    const content = result.rows[0];
    cache.set(cacheKey, content);
    return content;
  }

  async updateContent(
    section: string,
    content: ContentData,
    userId: number
  ): Promise<ContentVersion> {
    const current = await this.getContent(section);
    const newVersion = current ? current.version + 1 : 1;

    // Deactivate old version
    if (current) {
      await query(
        'UPDATE content SET is_active = false WHERE section = $1 AND is_active = true',
        [section]
      );
    }

    // Insert new version
    const result = await query(
      `INSERT INTO content (section, content, version, created_by, is_active) 
       VALUES ($1, $2, $3, $4, true) 
       RETURNING id, section, content, version, created_by, created_at`,
      [section, JSON.stringify(content), newVersion, userId]
    );

    // Log change
    await query(
      'INSERT INTO content_audit_log (section, version, action, user_id) VALUES ($1, $2, $3, $4)',
      [section, newVersion, 'update', userId]
    );

    // Clear cache
    cache.del(this.getCacheKey(section));

    return result.rows[0];
  }

  async getVersionHistory(section: string, limit: number = 10): Promise<ContentVersion[]> {
    const result = await query(
      `SELECT id, section, content, version, created_by, created_at 
       FROM content 
       WHERE section = $1 
       ORDER BY version DESC 
       LIMIT $2`,
      [section, limit]
    );

    return result.rows;
  }

  async getVersion(section: string, version: number): Promise<ContentVersion | null> {
    const result = await query(
      `SELECT id, section, content, version, created_by, created_at 
       FROM content 
       WHERE section = $1 AND version = $2`,
      [section, version]
    );

    return result.rows[0] || null;
  }

  async previewContent(section: string, content: ContentData): Promise<any> {
    // Process markdown if present
    const processed = { ...content };
    
    for (const key in processed) {
      if (typeof processed[key] === 'string' && processed[key].includes('**')) {
        processed[key] = marked(processed[key]);
      }
    }

    return processed;
  }

  validateContent(section: string, content: ContentData): { valid: boolean; errors: string[] } {
    const errors: string[] = [];
    const schemas: Record<string, string[]> = {
      hero: ['headline', 'subheadline', 'ctaPrimary', 'ctaSecondary'],
      features: ['items'],
      faq: ['items'],
      testimonials: ['items'],
      announcements: ['message', 'type'],
    };

    const required = schemas[section];
    if (!required) {
      errors.push(`Unknown section: ${section}`);
      return { valid: false, errors };
    }

    for (const field of required) {
      if (!(field in content)) {
        errors.push(`Missing required field: ${field}`);
      }
    }

    return { valid: errors.length === 0, errors };
  }
}
