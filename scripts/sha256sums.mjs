import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { readdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
const { version } = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'));

const directory = process.argv[2] ?? 'release';
const names = (await readdir(directory)).filter(name =>
  ['arm64', 'x64'].some(arch =>
    ['dmg', 'app.zip'].some(ext => name === `drag-share-${version}-macos-${arch}.${ext}`)
  )
).sort();
if (!names.length) throw new Error(`No v${version} macOS packages in ${directory}`);
const lines = [];
for (const name of names) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(join(directory, name))) hash.update(chunk);
  lines.push(`${hash.digest('hex')}  ${name}\n`);
}
await writeFile(join(directory, 'SHA256SUMS.txt'), lines.join(''));
