import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(fileURLToPath(new URL('.', import.meta.url)), '../docs/prototypes');
const basePort = Number(process.env.PORT) || 8765;
const maxAttempts = 20;
const types = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
};

function createApp() {
  return createServer(async (req, res) => {
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
  });
}

function tryListen(port, attempt) {
  const server = createApp();
  server.once('error', (err) => {
    if (err.code === 'EADDRINUSE' && attempt < maxAttempts) {
      console.warn(`Port ${port} in use, trying ${port + 1}...`);
      tryListen(port + 1, attempt + 1);
      return;
    }
    console.error(err);
    process.exit(1);
  });
  server.listen(port, () => {
    console.log(`Prototype: http://localhost:${port}/v0.6-skill-hub-prototype-v2.html`);
  });
}

tryListen(basePort, 0);
