#!/usr/bin/env node
import https from 'node:https';
import fs    from 'node:fs';
import path  from 'node:path';
import { createRequire } from 'node:module';

const require  = createRequire(import.meta.url);
const pkg      = require('../package.json');
const VERSION  = pkg.version;
const REPO     = pkg.repository.url.replace(/^.*github\.com\//, '').replace(/\.git$/, '');

const TARGETS = {
  darwin: { arm64: 'aarch64-apple-darwin',        x64: 'x86_64-apple-darwin'       },
  linux:  { arm64: 'aarch64-unknown-linux-musl',  x64: 'x86_64-unknown-linux-musl' },
  win32:  { x64:   'x86_64-pc-windows-msvc'                                         },
};

const target = TARGETS[process.platform]?.[process.arch];
if (!target) {
  console.error(`[loco-mcp] unsupported platform: ${process.platform}-${process.arch}`);
  process.exit(1);
}

const ext    = process.platform === 'win32' ? '.exe' : '';
const dest   = path.join(import.meta.dirname, '..', 'bin', `loco${ext}`);
const url    = `https://github.com/${REPO}/releases/download/v${VERSION}/loco-${target}${ext}`;

if (fs.existsSync(dest)) process.exit(0);

console.log(`[loco-mcp] downloading binary for ${process.platform}-${process.arch}...`);

function download(url, dest, redirects = 5) {
  if (redirects === 0) throw new Error('too many redirects');

  return new Promise((resolve, reject) => {
    https.get(url, { headers: { 'User-Agent': 'loco-mcp-installer' } }, (res) => {
      if (res.statusCode === 301 || res.statusCode === 302) {
        return download(res.headers.location, dest, redirects - 1).then(resolve).catch(reject);
      }
      if (res.statusCode !== 200) {
        return reject(new Error(`HTTP ${res.statusCode} from ${url}`));
      }

      fs.mkdirSync(path.dirname(dest), { recursive: true });
      const file = fs.createWriteStream(dest, { mode: 0o755 });
      res.pipe(file);
      file.on('finish', () => file.close(resolve));
      file.on('error', reject);
    }).on('error', reject);
  });
}

download(url, dest)
  .then(() => console.log('[loco-mcp] ready'))
  .catch((err) => {
    console.error(`[loco-mcp] download failed: ${err.message}`);
    console.error(`[loco-mcp] manually download from: ${url}`);
    process.exit(1);
  });
