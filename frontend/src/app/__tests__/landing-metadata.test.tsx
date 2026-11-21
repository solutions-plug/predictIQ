import { metadata } from '../page';
import { metadata as rootMetadata } from '../layout';

// AppShell pulls in a hook unrelated to metadata; stub it so layout metadata can load.
jest.mock('../../components/AppShell', () => ({ AppShell: () => null }));

/**
 * #1347 - the landing route emits SEO metadata (title/description/OG/Twitter) via
 * the Next.js Metadata API, and the root layout carries the site-wide defaults.
 * The JSON-LD is asserted structurally.
 */

describe('landing page metadata (#1347)', () => {
  it('sets a descriptive title and description', () => {
    expect(String(metadata.title)).toMatch(/PredictIQ/);
    expect(metadata.description).toEqual(expect.stringContaining('prediction markets'));
  });

  it('has Open Graph and Twitter card metadata with an image', () => {
    expect(metadata.openGraph?.type).toBe('website');
    expect(metadata.openGraph?.images).toBeTruthy();
    expect(metadata.twitter?.card).toBe('summary_large_image');
  });

  it('resolves relative asset URLs via metadataBase', () => {
    expect(metadata.metadataBase).toBeInstanceOf(URL);
    expect(metadata.alternates?.canonical).toBe('/');
  });
});

describe('landing page canonical and Open Graph (#1930)', () => {
  it('has an absolute canonical URL once resolved against metadataBase', () => {
    const canonical = metadata.alternates?.canonical as string;
    expect(canonical).toBeTruthy();
    const resolved = new URL(canonical, metadata.metadataBase as URL);
    expect(resolved.protocol).toMatch(/^https?:$/);
    expect(resolved.href).toBe(new URL('/', metadata.metadataBase as URL).href);
  });

  it('emits core Open Graph fields matching the page title', () => {
    const og = metadata.openGraph as Record<string, unknown>;
    expect(og.type).toBe('website');
    expect(og.title).toBe(metadata.title);
    expect(typeof og.description).toBe('string');
    expect((og.description as string).length).toBeGreaterThan(0);
    // OG description is a shortened form of the page description.
    expect(String(metadata.description)).toContain(String(og.description).replace(/\.$/, ''));
    expect(og.url).toBe((metadata.metadataBase as URL).href.replace(/\/$/, '') || og.url);
  });
});

describe('root layout metadata', () => {
  it('provides site-wide defaults and a title template', () => {
    expect(rootMetadata.metadataBase).toBeInstanceOf(URL);
    const title = rootMetadata.title as { default: string; template: string };
    expect(title.default).toMatch(/PredictIQ/);
    expect(title.template).toContain('%s');
    expect(rootMetadata.openGraph?.siteName).toBe('PredictIQ');
  });
});
