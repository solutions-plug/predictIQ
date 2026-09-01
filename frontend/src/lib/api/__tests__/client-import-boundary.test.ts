import { readdirSync, readFileSync, statSync } from 'fs';
import { join, relative, sep } from 'path';

const SRC_DIR = join(__dirname, '..', '..', '..');
const APP_DIR = join(SRC_DIR, 'app');

/**
 * Routes allowed to import the admin client directly:
 *   - anything under `src/app/admin/**` (the admin area)
 *   - the privileged `markets/<id>/resolve` route (market resolution is an
 *     admin/guardian action; it lives outside `admin/` only because of its URL shape)
 */
function isAllowed(relPath: string): boolean {
  const p = relPath.split(sep).join('/');
  return p.startsWith('app/admin/') || /^app\/markets\/[^/]+\/resolve\//.test(p);
}

function walk(dir: string, out: string[] = []): string[] {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) walk(full, out);
    else if (/\.(ts|tsx)$/.test(entry)) out.push(full);
  }
  return out;
}

const ADMIN_CLIENT_IMPORT = /from\s+['"][^'"]*\/api\/admin-client['"]/;

describe('module boundary: admin-client import sites (#1332)', () => {
  it('no non-admin route under src/app imports admin-client directly', () => {
    const offenders: string[] = [];
    for (const file of walk(APP_DIR)) {
      const relPath = relative(SRC_DIR, file);
      if (isAllowed(relPath)) continue;
      if (ADMIN_CLIENT_IMPORT.test(readFileSync(file, 'utf8'))) {
        offenders.push(relPath.split(sep).join('/'));
      }
    }
    expect(offenders).toEqual([]);
  });

  it('the public client never imports the admin client', () => {
    const publicSrc = readFileSync(join(__dirname, '..', 'public-client.ts'), 'utf8');
    expect(publicSrc).not.toMatch(/from\s+['"]\.\/admin-client['"]/);
  });

  it('the public client exposes no admin-only path prefixes', () => {
    const publicSrc = readFileSync(join(__dirname, '..', 'public-client.ts'), 'utf8');
    for (const prefix of ['/api/v1/admin', '/api/v1/audit', '/api/v1/email']) {
      expect(publicSrc).not.toContain(prefix);
    }
  });
});
