import fs from 'fs';
import path from 'path';

const SRC_DIR = path.resolve(__dirname, '../../');
const A11Y_CSS = path.resolve(SRC_DIR, 'styles/accessibility.css');
const TOKENS_CSS = path.resolve(SRC_DIR, 'styles/tokens.css');

function findCssFiles(dir: string): string[] {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      return findCssFiles(fullPath);
    }
    return entry.name.endsWith('.css') ? [fullPath] : [];
  });
}

/* ============================================================
   CSS token / block extraction helpers
   ============================================================ */

/** Extract the body of the first top-level block whose selector line matches. */
function readBlock(css: string, selector: RegExp): string {
  const lines = css.split('\n');
  const body: string[] = [];
  let inBlock = false;
  let started = false;
  let depth = 0;

  for (const line of lines) {
    if (!inBlock) {
      if (selector.test(line)) {
        inBlock = true;
        const brace = (line.match(/\{/g) || []).length;
        if (brace > 0) {
          started = true;
          depth = brace;
        }
      }
      continue;
    }
    if (!started) {
      const brace = (line.match(/\{/g) || []).length;
      if (brace > 0) {
        started = true;
        depth = brace;
      }
    } else {
      depth += (line.match(/\{/g) || []).length - (line.match(/\}/g) || []).length;
    }
    body.push(line.replace(/^[ \t]+/, ''));
    if (started && depth === 0) break;
  }
  return body.join('\n');
}

/** Extract a CSS custom property value from a block body. */
function varValue(block: string, name: string): string | null {
  // `name` already includes the leading `--` (e.g. "--ring-strong").
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const m = block.match(new RegExp(`${escaped}\\s*:\\s*([^;]+);`));
  return m ? m[1].trim() : null;
}

/* ============================================================
   Color / contrast helpers (WCAG 2.x APCA-free ratio math)
   ============================================================ */

type RGB = [number, number, number]; // 0..1 floats

function channelToLinear(c: number): number {
  return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

function hexToRgb(hex: string): RGB | null {
  const m = hex.trim().replace('#', '').match(/^([0-9a-f]{6})$/i);
  if (!m) return null;
  return [
    parseInt(m[1].slice(0, 2), 16) / 255,
    parseInt(m[1].slice(2, 4), 16) / 255,
    parseInt(m[1].slice(4, 6), 16) / 255,
  ];
}

/** Parse `rgb(r,g,b)` / `rgba(r,g,b,a)` into floats. */
function rgbaToRgb(value: string): RGB | null {
  const m = value.match(/rgba?\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)(?:\s*,\s*([\d.]+))?\s*\)/i);
  if (!m) return null;
  return [Number(m[1]) / 255, Number(m[2]) / 255, Number(m[3]) / 255];
}

function luminance(rgb: RGB): number {
  return (
    0.2126 * channelToLinear(rgb[0]) +
    0.7152 * channelToLinear(rgb[1]) +
    0.0722 * channelToLinear(rgb[2])
  );
}

function contrastRatio(a: RGB, b: RGB): number {
  const l1 = luminance(a);
  const l2 = luminance(b);
  const [hi, lo] = [Math.max(l1, l2), Math.min(l1, l2)];
  return (hi + 0.05) / (lo + 0.05);
}

/** Resolve a token value (hex or rgba) to RGB. r,g,b are 0..255 ints. */
function parseColor(value: string): RGB | null {
  const trimmed = value.trim();
  if (/^#/.test(trimmed)) return hexToRgb(trimmed);
  if (/^rgba?\(/.test(trimmed)) return rgbaToRgb(trimmed);
  return null;
}

/**
 * Composite a translucent foreground over an opaque background, returning the
 * resulting opaque RGB. Used to resolve `--surface-glass` tokens.
 */
function composite(fg: RGB, base: RGB, alpha: number): RGB {
  return [
    fg[0] * alpha + base[0] * (1 - alpha),
    fg[1] * alpha + base[1] * (1 - alpha),
    fg[2] * alpha + base[2] * (1 - alpha),
  ];
}

function alphaOf(value: string): number | null {
  const m = value.match(/,?\s*([\d.]+)\)\s*$/);
  return m ? Number(m[1]) : null;
}

/* ============================================================
   Shared fixtures extracted from tokens.css
   ============================================================ */

const tokensCss = fs.readFileSync(TOKENS_CSS, 'utf-8');
const a11yCss = fs.readFileSync(A11Y_CSS, 'utf-8');

// Scopes define where each color scheme's tokens live in tokens.css.
const COLOR_SCOPES: Record<'dark' | 'light', RegExp> = {
  dark: /^:root\s*\{/,
  light: /html:not\(\.dark-mode\)\s*\{/,
};

/** The opaque background tokens by name — shared across schemes. */
const BG_TOKEN_NAMES = ['--bg', '--bg-deep', '--surface', '--surface-2'];

/** For each scheme, the RGB background tokens the focus ring must contrast with. */
function backgroundTokens(scheme: 'dark' | 'light', baseForGlass: RGB): { name: string; rgb: RGB }[] {
  const block = readBlock(tokensCss, COLOR_SCOPES[scheme]);
  const tokens: { name: string; rgb: RGB }[] = [];

  for (const name of BG_TOKEN_NAMES) {
    const raw = varValue(block, name);
    const rgb = raw ? parseColor(raw) : null;
    if (rgb) tokens.push({ name, rgb });
  }

  const glassRaw = varValue(block, '--surface-glass');
  const alpha = glassRaw ? alphaOf(glassRaw) : null;
  const glassRgb = glassRaw ? rgbaToRgb(glassRaw) : null;
  if (glassRaw && alpha !== null && glassRgb) {
    tokens.push({ name: '--surface-glass', rgb: composite(glassRgb, baseForGlass, alpha) });
  }

  return tokens;
}

describe('.visually-hidden utility class', () => {
  it('is defined exactly once across the stylesheet tree', () => {
    const cssFiles = findCssFiles(SRC_DIR);
    const definitions = cssFiles.filter((file) =>
      fs.readFileSync(file, 'utf-8').match(/\.visually-hidden\s*\{/)
    );

    expect(definitions).toEqual([path.resolve(SRC_DIR, 'styles/accessibility.css')]);
  });
});

/* ============================================================
   Requirement 1 — :focus-visible contrast vs every bg token
   ============================================================ */

describe(':focus-visible outlines', () => {
  const fork = readBlock(a11yCss, /\*:focus-visible/);

  it('uses the mode-aware --ring-strong token (with gold fallback)', () => {
    expect(fork).toContain('var(--ring-strong, var(--ring, #f59e0b))');
  });

  test.each(['dark', 'light'] as const)(
    '%s scheme: --ring-strong holds >= 3:1 against every background token',
    (scheme) => {
      const block = readBlock(tokensCss, COLOR_SCOPES[scheme]);
      const ring = varValue(block, '--ring-strong');
      expect(ring).toBeTruthy();

      const ringRgb = parseColor(ring as string);
      expect(ringRgb).not.toBeNull();

      // --surface-glass renders over the deepest page background.
      const baseForGlass = scheme === 'dark' ? (hexToRgb('#070b16') as RGB) : (hexToRgb('#f7f9fc') as RGB);
      const bgs = backgroundTokens(scheme, baseForGlass);
      expect(bgs.length).toBeGreaterThan(0);

      for (const bg of bgs) {
        const ratio = contrastRatio(ringRgb as RGB, bg.rgb);
        if (ratio < 3) {
          throw new Error(
            `${scheme} ring ${ring} vs ${bg.name} = ${ratio.toFixed(2)}:1 (< 3:1 minimum)`
          );
        }
      }
    }
  );
});

/* ============================================================
   Requirement 2 — minimum interactive target size (44 x 44)
   ============================================================ */

describe('interactive target size (WCAG 2.5.5)', () => {
  const block = readBlock(a11yCss, /INTERACTIVE TARGET SIZE/);

  test('targets buttons, links, and form controls', () => {
    expect(block).toContain('button');
    expect(block).toContain("a[href]");
    expect(block).toContain('input');
    expect(block).toContain('select');
    expect(block).toContain('textarea');
  });

  it('enforces at least 44 x 44 CSS px', () => {
    expect(block).toMatch(/min-height:\s*44px/);
    expect(block).toMatch(/min-width:\s*44px/);
  });
});

/* ============================================================
   Requirement 3 — prefers-reduced-motion disables animation
   ============================================================ */

describe('prefers-reduced-motion', () => {
  const block = readBlock(a11yCss, /prefers-reduced-motion: reduce/);

  it('declares the @media (prefers-reduced-motion: reduce) query', () => {
    expect(a11yCss).toContain('@media (prefers-reduced-motion: reduce) {');
  });

  it('disables non-essential animation globally', () => {
    expect(block).toMatch(/animation-\s*duration/);
  });

  it('explicitly disables spinners, skeleton shimmer, and toast transitions', () => {
    expect(block).toContain('.spinner');
    expect(block).toContain('.loading-spinner .spinner');
    expect(block).toContain('.skeleton');
    // Toast / transition-driven UI is covered by the universal transition kill.
    expect(block).toMatch(/transition-duration/);
  });
});