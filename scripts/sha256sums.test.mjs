import { mkdtemp, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { test } from 'node:test';

const { version } = JSON.parse(await readFile(new URL('../package.json', import.meta.url)));
const script = new URL('./sha256sums.mjs', import.meta.url);
test('checksums reject empty input, retain both architectures, and exclude older versions', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'drag-share-checksums-'));
  const run = () => execFileSync(process.execPath, [script.pathname, directory], { stdio: 'pipe' });
  try {
    assert.throws(run, error => error.status === 1 && error.stderr.includes('No v'));
    const files = ['arm64', 'x64'].flatMap(arch => ['dmg', 'app.zip'].map(ext =>
      `drag-share-${version}-macos-${arch}.${ext}`));
    for (const name of files.slice(0, 2)) await writeFile(join(directory, name), name);
    run();
    assert.equal((await readFile(join(directory, 'SHA256SUMS.txt'), 'utf8')).trim().split('\n').length, 2);
    for (const name of files.slice(2)) await writeFile(join(directory, name), name);
    files.push(`drag-share-${version}-windows-x64-setup.exe`, `drag-share-${version}-windows-x64.zip`);
    for (const name of files.slice(-2)) await writeFile(join(directory, name), name);
    await writeFile(join(directory, 'drag-share-0.0.0-macos-arm64.dmg'), 'old');
    run();
    assert.equal(await readFile(join(directory, 'SHA256SUMS.txt'), 'utf8'), files.sort().map(name =>
      `${createHash('sha256').update(name).digest('hex')}  ${name}\n`).join(''));
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('packaging selects target paths and names, retains arm64, and rejects mismatched apps', async () => {
  const { mkdir, copyFile, chmod } = await import('node:fs/promises');
  const root = await mkdtemp(join(tmpdir(), 'drag-share-packaging-'));
  try {
    await mkdir(join(root, 'scripts'));
    await mkdir(join(root, 'bin'));
    for (const name of ['package-release.sh', 'sha256sums.mjs']) {
      await copyFile(new URL(name, import.meta.url), join(root, 'scripts', name));
    }
    await writeFile(join(root, 'package.json'), JSON.stringify({ version }));
    // Mock only the macOS tools; this verifies orchestration, not real signing or DMG integrity.
    const commands = {
      lipo: 'printf "%s\\n" "$MOCK_ARCH"',
      codesign: 'exit 0',
      hdiutil: 'exit 0',
      ditto: 'for last do :; done; printf zip > "$last"',
    };
    for (const [name, body] of Object.entries(commands)) {
      const path = join(root, 'bin', name);
      await writeFile(path, `#!/bin/sh\nset -eu\n${body}\n`);
      await chmod(path, 0o755);
    }
    const run = (target, arch) => execFileSync('sh', ['scripts/package-release.sh', target], {
      cwd: root, env: { ...process.env, PATH: `${root}/bin:${process.env.PATH}`, MOCK_ARCH: arch }, stdio: 'pipe',
    });
    for (const [target, arch, suffix] of [
      ['aarch64-apple-darwin', 'arm64', 'aarch64'],
      ['x86_64-apple-darwin', 'x86_64', 'x64'],
    ]) {
      const bundle = join(root, 'src-tauri/target', target, 'release/bundle');
      await mkdir(join(bundle, 'macos/SMB 共享助手.app/Contents/MacOS'), { recursive: true });
      await mkdir(join(bundle, 'dmg'), { recursive: true });
      await writeFile(join(bundle, `dmg/SMB 共享助手_${version}_${suffix}.dmg`), target);
      run(target, arch);
    }
    const sums = await readFile(join(root, 'release/SHA256SUMS.txt'), 'utf8');
    assert.equal(sums.trim().split('\n').length, 4);
    assert.match(sums, /macos-arm64.app.zip/);
    assert.match(sums, /macos-x64.app.zip/);
    assert.throws(() => run('x86_64-apple-darwin', 'arm64'), error =>
      error.status === 1 && error.stderr.includes('architecture does not match'));
    assert.equal(await readFile(join(root, 'release/SHA256SUMS.txt'), 'utf8'), sums);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
