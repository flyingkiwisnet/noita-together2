// Public /v1/info envelope from g2-next/app.ts and candidate-loot01.ts.
// Uses the Server-owned semantics declaration and actual private runtime/profile.
// This fixture starts no service/database and contains no credentials.
import {readFileSync} from 'node:fs';
import {declareRoomSemantics} from '../../../packages/contracts/src/room-semantics.js';
const runtime=JSON.parse(readFileSync(process.env.NT2_B1_INFO_RUNTIME,'utf8').replace(/^\uFEFF/,''));
const schema=JSON.parse(readFileSync(new URL('../../../packages/contracts/g2-next/schema.json',import.meta.url),'utf8'));
process.stdout.write(JSON.stringify({major:1,revision:schema.revision,connectionFlowRevision:'g3-flow-1',serviceId:runtime.serviceId,serviceEpoch:'b1-public-info-consumer',compatibility:runtime.compatibility,capabilities:runtime.compatibility.capabilities,roomSemantics:declareRoomSemantics(runtime.compatibility),mockProvider:true,contractStatus:'candidate-not-accepted',profile:runtime.buildRegistry}));
