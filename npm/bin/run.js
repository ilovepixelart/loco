#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import path          from 'node:path';
import fs            from 'node:fs';

const ext    = process.platform === 'win32' ? '.exe' : '';
const binary = path.join(import.meta.dirname, `loco${ext}`);

if (!fs.existsSync(binary)) {
  console.error('[loco-mcp] binary not found — try reinstalling: npm install -g loco-mcp');
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), {
  stdio: 'inherit',
  env:   process.env,
});

process.exit(result.status ?? 1);
