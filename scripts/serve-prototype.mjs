import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(fileURLToPath(new URL('.', import.meta.url)), '../docs/prototypes');
const port = 8765;
const types = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
};

createServer(async (req, res) => {
  let path = (req.url ?? '/').split('?')[0];
  if (path === '/') path = '/index.html';

  try {
    const filePath = join(root, path);
    const data = await readFile(filePath);
    res.writeHead(200, { 'Content-Type': types[extname(path)] ?? 'application/octet-stream' });
    res.end(data);
  } catch {
    res.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' });
    res.end(`404 Not Found: ${path}`);
  }
}).listen(port, () => {
  console.log(`Prototype: http://localhost:${port}/v0.3-skill-hub-prototype.html`);
});
