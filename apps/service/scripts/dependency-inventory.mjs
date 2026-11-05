import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
const root = new URL('../../../', import.meta.url);
const lockBytes = readFileSync(new URL('package-lock.json',root));
const lock = JSON.parse(lockBytes);
const dependencies = Object.entries(lock.packages)
  .filter(([path,entry]) => path.includes('node_modules/') && !entry.link)
  .map(([path,entry]) => {
    const metadataUrl = new URL(`${path}/package.json`,root);
    const installed = existsSync(metadataUrl);
    const metadata = installed ? JSON.parse(readFileSync(metadataUrl,'utf8')) : entry;
    if (!installed && !entry.optional) throw new Error(`Missing required installed package: ${path}`);
    return {path,name:metadata.name ?? path.split('node_modules/').at(-1),version:entry.version,license:metadata.license ?? 'REVIEW_REQUIRED',integrity:entry.integrity,installed,optional:entry.optional === true,developmentOnly:entry.dev === true};
  });
const gate=process.argv.includes('--g2')?'g2':process.argv.includes('--g1')?'g1':'g0';
writeFileSync(new URL(`docs/${gate}/dependency-inventory.json`,root), JSON.stringify({
  status:`${gate.toUpperCase()} locked npm dependency inventory with installed/platform-optional flags; not a shipping SBOM or transitive security audit`,
  lockfileSha256:createHash('sha256').update(lockBytes).digest('hex'), dependencies
},null,2) + '\n');
console.log(`Recorded ${dependencies.length} locked npm packages (${dependencies.filter(d => d.installed).length} installed) and lockfile digest`);
