import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(fileURLToPath(new URL('.', import.meta.url)), '../docs/prototypes');
const preferredPort = Number(process.env.PORT) || 8765;
const types = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
};

const server = createServer(async (req, res) => {
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

function listen(port) {
  server.listen(port, () => {
    console.log(`Prototype: http://localhost:${port}/v0.4-gitlab-sources-prototype.html`);
  });
}

server.on('error', (err) => {
  if (err.code === 'EADDRINUSE' && preferredPort === Number(process.env.PORT || 8765)) {
    const next = preferredPort + 1;
    console.warn(`Port ${preferredPort} in use, trying ${next}...`);
    listen(next);
    return;
  }
  throw err;
});

listen(preferredPort);
