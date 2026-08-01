import fs from 'fs';
import path from 'path';

const SRC_DIR = path.resolve(__dirname, '../../');

function findCssFiles(dir: string): string[] {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      return findCssFiles(fullPath);
    }
    return entry.name.endsWith('.css') ? [fullPath] : [];
  });
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
