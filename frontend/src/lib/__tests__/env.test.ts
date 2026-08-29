describe('env validation', () => {
  const original = process.env.NEXT_PUBLIC_API_URL;
  const originalTts = process.env.NEXT_PUBLIC_TTS_API_URL;

  afterEach(() => {
    process.env.NEXT_PUBLIC_API_URL = original;
    process.env.NEXT_PUBLIC_TTS_API_URL = originalTts;
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

  // Issue #1307: TTS integration (#116) is now in scope, but no test
  // exercised NEXT_PUBLIC_TTS_API_URL's validation branch at all.
  it('parses a valid NEXT_PUBLIC_TTS_API_URL', () => {
    process.env.NEXT_PUBLIC_API_URL = 'http://localhost:3001';
    process.env.NEXT_PUBLIC_TTS_API_URL = 'http://localhost:3002';
    let mod: typeof import('../env');
    jest.isolateModules(() => {
      mod = require('../env');
    });
    expect(mod!.getEnvConfig().NEXT_PUBLIC_TTS_API_URL).toBe('http://localhost:3002');
  });

  it('allows NEXT_PUBLIC_TTS_API_URL to be unset — TTS features are optional', () => {
    process.env.NEXT_PUBLIC_API_URL = 'http://localhost:3001';
    delete process.env.NEXT_PUBLIC_TTS_API_URL;
    let mod: typeof import('../env');
    expect(() => {
      jest.isolateModules(() => {
        mod = require('../env');
      });
    }).not.toThrow();
    expect(mod!.getEnvConfig().NEXT_PUBLIC_TTS_API_URL).toBeUndefined();
  });

  it('allows NEXT_PUBLIC_TTS_API_URL to be an empty string', () => {
    process.env.NEXT_PUBLIC_API_URL = 'http://localhost:3001';
    process.env.NEXT_PUBLIC_TTS_API_URL = '';
    expect(() => {
      jest.isolateModules(() => {
        require('../env');
      });
    }).not.toThrow();
  });

  it('throws when NEXT_PUBLIC_TTS_API_URL is set but not a valid URL', () => {
    process.env.NEXT_PUBLIC_API_URL = 'http://localhost:3001';
    process.env.NEXT_PUBLIC_TTS_API_URL = 'not-a-url';
    expect(() => {
      jest.isolateModules(() => {
        require('../env');
      });
    }).toThrow(/NEXT_PUBLIC_TTS_API_URL.*must be a valid URL/s);
  });
});
