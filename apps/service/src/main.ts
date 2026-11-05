import { createMockApp } from './app.js';
if (process.env.NT2_G0_MOCK !== '1' || process.env.NODE_ENV === 'production') throw new Error('G0 mock requires NT2_G0_MOCK=1 and refuses production');
const port = Number(process.env.NT2_G0_PORT ?? '4317');
if (!Number.isInteger(port) || port < 1024 || port > 65535) throw new Error('Invalid G0 port');
const {app} = await createMockApp();
const address = await app.listen({host:'127.0.0.1',port});
console.log(`G0 MOCK ONLY ${address}; provisional g0-json-1; state resets on restart`);
for (const signal of ['SIGINT','SIGTERM'] as const) process.once(signal, () => {void app.close().then(() => {process.exitCode = 0;});});
