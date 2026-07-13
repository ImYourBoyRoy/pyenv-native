#!/usr/bin/env node
// ./scripts/gui-a11y/check-a11y.mjs
/**
 * Static accessibility audit for the pyenv-native GUI markup.
 * Parses index.html in jsdom (without executing app.js) and runs axe-core.
 *
 * Usage: npm run check (from scripts/gui-a11y)
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { JSDOM } from 'jsdom';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '../..');
const uiDir = path.join(repoRoot, 'crates/pyenv-gui/ui');
const htmlPath = path.join(uiDir, 'index.html');

const html = fs.readFileSync(htmlPath, 'utf8');
const staticHtml = html
    .replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '')
    .replace(/<script\b[^>]*\/>/gi, '')
    .replace(/<link[^>]+href="styles\.css"[^>]*>/i, '');

const dom = new JSDOM(staticHtml, {
    url: 'http://localhost/',
    pretendToBeVisual: true,
});
const { window } = dom;
const { document } = window;

global.window = window;
global.document = document;
global.Node = window.Node;
global.Element = window.Element;
global.HTMLElement = window.HTMLElement;

const axe = (await import('axe-core')).default;

const results = await axe.run(document.documentElement, {
    runOnly: {
        type: 'tag',
        values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'best-practice'],
    },
    rules: {
        'color-contrast': { enabled: false },
    },
});

const violations = results.violations || [];
if (violations.length > 0) {
    console.error(`GUI accessibility check failed with ${violations.length} violation(s):\n`);
    violations.forEach((violation, index) => {
        console.error(`${index + 1}. [${violation.impact}] ${violation.id}: ${violation.help}`);
        violation.nodes.slice(0, 3).forEach((node) => {
            console.error(`   - ${node.html}`);
        });
        if (violation.nodes.length > 3) {
            console.error(`   … and ${violation.nodes.length - 3} more node(s)`);
        }
    });
    process.exit(1);
}

console.log(`GUI accessibility check passed (${results.passes.length} rules passed).`);
