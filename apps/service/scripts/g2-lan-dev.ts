import {resolve} from 'node:path';
import {lanConfig} from '../src/g2/lan.js';
const config=lanConfig(process.argv[2],Number(process.argv[3]??4352));
process.env.NT2_G2_MOCK='1';process.env.NT2_G2_PROFILE='candidate';
process.env.NT2_G2_LAN_CONFIG=resolve(`secrets/g2-lan-${config.ip.replaceAll('.','-')}-${config.port}/config.json`);
await import('../src/g2/main.js');
