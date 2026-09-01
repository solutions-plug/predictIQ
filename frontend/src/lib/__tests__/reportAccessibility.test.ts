// The React instance the harness passes through to @axe-core/react.
const fakeReact = { __marker: true } as unknown as typeof import('react');

jest.mock('@axe-core/react', () => ({ __esModule: true, default: jest.fn() }));
jest.mock('react-dom', () => ({ __isMockDOM: true }));

describe('reportAccessibility', () => {
  // Load fresh module state per test so the internal `initialized` singleton
  // guard resets between cases.
  const load = () => {
    jest.resetModules();
    const mod = require('../reportAccessibility') as typeof import('../reportAccessibility');
    const axeDefault = (require('@axe-core/react') as { default: jest.Mock }).default;
    axeDefault.mockClear();
    return { reportAccessibility: mod.reportAccessibility, axeDefault };
  };

  it('is a no-op outside a development build (production env)', async () => {
    const { reportAccessibility, axeDefault } = load();
    jest.replaceProperty(process.env, 'NODE_ENV', 'production');

    await reportAccessibility(fakeReact);

    expect(axeDefault).not.toHaveBeenCalled();
  });

  it('initializes @axe-core/react in a development build', async () => {
    const { reportAccessibility, axeDefault } = load();
    jest.replaceProperty(process.env, 'NODE_ENV', 'development');

    const config = { rules: { 'color-contrast': { enabled: true } } };
    await reportAccessibility(fakeReact, config);

    expect(axeDefault).toHaveBeenCalledTimes(1);
    const [reactArg, domArg, timeout, cfg] = axeDefault.mock.calls[0];
    expect(reactArg).toBe(fakeReact);
    expect(domArg.__isMockDOM).toBe(true);
    expect(timeout).toBe(1000);
    expect(cfg).toEqual(config);
  });

  it('only initializes once even when called multiple times', async () => {
    const { reportAccessibility, axeDefault } = load();
    jest.replaceProperty(process.env, 'NODE_ENV', 'development');

    await reportAccessibility(fakeReact);
    await reportAccessibility(fakeReact);
    await reportAccessibility(fakeReact);

    expect(axeDefault).toHaveBeenCalledTimes(1);
  });
});