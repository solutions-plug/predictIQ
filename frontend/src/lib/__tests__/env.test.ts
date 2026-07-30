describe('env validation', () => {
  const original = process.env.NEXT_PUBLIC_API_URL;

  afterEach(() => {
    process.env.NEXT_PUBLIC_API_URL = original;
    jest.resetModules();
  });

  it('returns the parsed config when the URL is valid', () => {
    process.env.NEXT_PUBLIC_API_URL = 'http://localhost:3001';
    let mod: typeof import('../env');
    jest.isolateModules(() => {
      mod = require('../env');
    });
    expect(mod!.getEnvConfig().NEXT_PUBLIC_API_URL).toBe('http://localhost:3001');
    expect(mod!.validateEnvironment().NEXT_PUBLIC_API_URL).toBe('http://localhost:3001');
  });

  it('throws a descriptive error when NEXT_PUBLIC_API_URL is missing', () => {
    delete process.env.NEXT_PUBLIC_API_URL;
    expect(() => {
      jest.isolateModules(() => {
        require('../env');
      });
    }).toThrow(/Missing or invalid environment variables/);
  });

  it('throws when NEXT_PUBLIC_API_URL is not a valid URL', () => {
    process.env.NEXT_PUBLIC_API_URL = 'not-a-url';
    expect(() => {
      jest.isolateModules(() => {
        require('../env');
      });
    }).toThrow(/must be a valid URL/);
  });
});
