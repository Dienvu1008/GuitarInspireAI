// FF-33 (Mermaid part): parse every ```mermaid block under the given directory.
// Exit code 1 if any diagram fails to parse. Usage: node check.mjs <dir>
import { JSDOM } from 'jsdom';
import fs from 'node:fs';
import path from 'node:path';

const dom = new JSDOM('<!DOCTYPE html><body></body>');
globalThis.window = dom.window;
globalThis.document = dom.window.document;
globalThis.DOMParser = dom.window.DOMParser;
globalThis.Element = dom.window.Element;
globalThis.HTMLElement = dom.window.HTMLElement;

const { default: mermaid } = await import('mermaid');
mermaid.initialize({ startOnLoad: false });

const root = process.argv[2];
if (!root) {
  console.error('usage: node check.mjs <dir>');
  process.exit(2);
}

const files = [];
const walk = (dir) => {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p);
    else if (p.endsWith('.md')) files.push(p);
  }
};
walk(root);

let total = 0;
let failed = 0;
for (const file of files.sort()) {
  const text = fs.readFileSync(file, 'utf8');
  const re = /```mermaid\r?\n([\s\S]*?)```/g;
  let m;
  let i = 0;
  while ((m = re.exec(text))) {
    i += 1;
    total += 1;
    try {
      await mermaid.parse(m[1]);
    } catch (err) {
      failed += 1;
      const msg = String(err?.message ?? err).split('\n').slice(0, 3).join(' | ');
      console.log(`FAIL ${path.relative(root, file)} diagram ${i}: ${msg}`);
    }
  }
}
console.log(`FF-33 mermaid: ${total} diagrams, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
