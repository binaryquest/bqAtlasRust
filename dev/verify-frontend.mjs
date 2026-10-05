import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
const root = new URL('../samples/crm/web/vendor/', import.meta.url);
const manifest = JSON.parse(await readFile(new URL('manifest.json', root), 'utf8'));
for (const artifact of manifest.artifacts) {
  const digest = createHash('sha256').update(await readFile(new URL(artifact.file, root))).digest('hex');
  if (digest !== artifact.sha256) throw new Error(`Frontend artifact checksum mismatch: ${artifact.file}`);
}
console.log(`Verified ${manifest.artifacts.length} shared frontend archives at ${manifest.sourceRevision}`);
