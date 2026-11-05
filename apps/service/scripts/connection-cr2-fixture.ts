import {startG3Fixture,provisionG3Fixture} from './g3-loot01-fixture.js';
import {schema} from '../../../packages/contracts/src/g3-connection.js';
// In-process Client composition seam. No secrets printed, no kit/profile repin.
// The caller owns finally { await result.fixture.close(); }. All engine facts in
// scenario are synthetic: start()/playing() are fixture controls, not engine evidence.
export async function startConnectionCr2Fixture(options:{now?:()=>number}={}){
 const fixture=await startG3Fixture({databaseUrl:'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1',rejoin:true,loot01Runtime:true,live:true,tls:true,...options});
 try{return {revision:schema.revision,fixture,scenario:await provisionG3Fixture(fixture)};}
 catch(e){await fixture.close();throw e;}
}
