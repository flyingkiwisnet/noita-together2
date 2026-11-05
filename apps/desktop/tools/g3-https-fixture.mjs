// Client test orchestration around the SERVER-owned disposable factory; simulated game facts only.
import {writeFile, rm} from 'node:fs/promises';
import {join} from 'node:path';
import {createInterface} from 'node:readline';
import {startG3Fixture, provisionG3Fixture, privateG3Directory} from '../../service/scripts/g3-fixture.ts';
const fixture = await startG3Fixture({tls: true,social:process.argv.includes('--social')});
let directory;
try {
  const state = await provisionG3Fixture(fixture);
  directory = await privateG3Directory('nt2-g3-rust-fixture-');
  const bundlePath = join(directory, 'credentials.json');
  const publish = async () => {
    await writeFile(bundlePath, JSON.stringify({evidenceStatus:'simulated-game-facts', origin:fixture.origin,
      namespace:fixture.namespace, caPath:fixture.caPath, serviceId:fixture.service.db.serviceId,
      serviceEpoch:fixture.service.db.epoch, desktops:state.desktops, games:state.games, scopes:state.scopes}), {mode:0o600});
  };
  await publish();
  console.log(JSON.stringify({event:'ready', bundlePath, namespace:fixture.namespace}));
  const input = createInterface({input:process.stdin});
  let finish; const done = new Promise(resolve => { finish = resolve; });
  const timer = setTimeout(finish, 180000); timer.unref(); input.on('close', finish);
  let commands = Promise.resolve();
  input.on('line', line => { commands = commands.then(async () => {
    switch (line.trim()) {
      case 'start': await state.start(); break;
      case 'playing': await state.playing(); break;
      case 'end': await state.rpc(state.host, {endRun:{}}); break;
      case 'restart': await fixture.restart(); await publish(); break;
      case 'quit': finish(); return;
      default: throw Error('Unsupported client fixture command');
    }
    console.log(JSON.stringify({event:line.trim()}));
  }).catch(() => { console.log(JSON.stringify({event:'failed'})); finish(); }); });
  await done; clearTimeout(timer); input.close(); await commands;
} finally {
  try { await fixture.close(); } finally { if (directory) await rm(directory, {recursive:true, force:true}); }
}
