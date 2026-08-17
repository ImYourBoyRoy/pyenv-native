export default {
  schemaVersion: 1,
  project: { name: 'pyenv-native-gui', root: '.' },
  profile: 'wcag22-aa',
  outputDirectory: 'wcag-audit',
  adapters: [
    {
      id: 'frontend-browser-mode',
      type: 'playwright-axe',
      baseURL: 'http://127.0.0.1:8766',
      webServer: {
        command: 'python3',
        args: ['-m', 'http.server', '8766', '--bind', '127.0.0.1'],
        cwd: '../../crates/pyenv-gui/ui',
        url: 'http://127.0.0.1:8766/index.html',
        timeoutMs: 15000,
        reuseExistingServer: true,
      },
      scenarios: [
        { name: 'dashboard', path: '/index.html', steps: [] },
        {
          name: 'install-runtimes',
          path: '/index.html',
          steps: [
            { action: 'click', selector: '[data-view="view-available"]' },
            { action: 'expectVisible', selector: '#view-available' },
          ],
        },
        {
          name: 'shell-diagnostics',
          path: '/index.html',
          steps: [
            { action: 'click', selector: '[data-view="view-shell"]' },
            { action: 'expectVisible', selector: '#view-shell' },
          ],
        },
      ],
      probes: {
        targetSizeEnhanced: { enabled: false, minimum: 44 },
        focusIndicatorReview: { enabled: true, maxTabs: 80 },
      },
    },
  ],
  reporters: [
    { type: 'console' },
    { type: 'json', file: 'latest.json' },
    { type: 'results', file: 'results.html' },
  ],
};
