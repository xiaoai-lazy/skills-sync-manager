import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

/** Rust AppConfig fields intentionally absent from TS (none today). */
const IGNORE_RUST_FIELDS = new Set<string>([]);

/** TS AppConfig fields intentionally absent from Rust (none today). */
const IGNORE_TS_FIELDS = new Set<string>([]);

function snakeToCamel(snake: string): string {
  return snake.replace(/_([a-z])/g, (_, c: string) => c.toUpperCase());
}

function extractRustAppConfigFields(source: string): string[] {
  const start = source.indexOf('pub struct AppConfig {');
  if (start < 0) throw new Error('AppConfig struct not found in models.rs');
  const brace = source.indexOf('{', start);
  let depth = 0;
  let end = brace;
  for (let i = brace; i < source.length; i++) {
    if (source[i] === '{') depth++;
    else if (source[i] === '}') {
      depth--;
      if (depth === 0) {
        end = i;
        break;
      }
    }
  }
  const body = source.slice(brace + 1, end);
  const fields: string[] = [];
  for (const line of body.split(/\r?\n/)) {
    const m = line.match(/^\s*pub\s+([a-z][a-z0-9_]*)\s*:/);
    if (m) fields.push(m[1]);
  }
  return fields;
}

function extractTsAppConfigFields(source: string): string[] {
  const start = source.indexOf('export interface AppConfig {');
  if (start < 0) throw new Error('AppConfig interface not found in types.ts');
  const brace = source.indexOf('{', start);
  let depth = 0;
  let end = brace;
  for (let i = brace; i < source.length; i++) {
    if (source[i] === '{') depth++;
    else if (source[i] === '}') {
      depth--;
      if (depth === 0) {
        end = i;
        break;
      }
    }
  }
  const body = source.slice(brace + 1, end);
  const fields: string[] = [];
  for (const line of body.split(/\r?\n/)) {
    const m = line.match(/^\s*([a-zA-Z][a-zA-Z0-9]*)\??\s*:/);
    if (m) fields.push(m[1]);
  }
  return fields;
}

describe('AppConfig field alignment', () => {
  it('keeps Rust and TypeScript AppConfig keys in sync (snake ↔ camel)', () => {
    const rustPath = resolve(__dirname, '../../src-tauri/src/models.rs');
    const tsPath = resolve(__dirname, '../model/types.ts');
    const rustFields = extractRustAppConfigFields(readFileSync(rustPath, 'utf8'))
      .filter((f) => !IGNORE_RUST_FIELDS.has(f))
      .map(snakeToCamel)
      .sort();
    const tsFields = extractTsAppConfigFields(readFileSync(tsPath, 'utf8'))
      .filter((f) => !IGNORE_TS_FIELDS.has(f))
      .sort();

    expect(tsFields).toEqual(rustFields);
  });
});
