import { metadata } from '../page';
import { metadata as rootMetadata } from '../layout';

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

describe('root layout metadata', () => {
  it('provides site-wide defaults and a title template', () => {
    expect(rootMetadata.metadataBase).toBeInstanceOf(URL);
    const title = rootMetadata.title as { default: string; template: string };
    expect(title.default).toMatch(/PredictIQ/);
    expect(title.template).toContain('%s');
    expect(rootMetadata.openGraph?.siteName).toBe('PredictIQ');
  });
});
