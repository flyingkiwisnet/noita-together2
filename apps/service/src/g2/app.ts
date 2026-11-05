import {createG1App,type Options} from '../g1/app.js';
// One auth/database/room authority; only the explicit wire adapter/profile changes.
export function createG2App(options:Omit<Options,'protocolG2'>){return createG1App({...options,protocolG2:true});}
