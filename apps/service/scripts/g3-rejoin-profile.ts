import {materializeRejoinComposition} from '../src/rejoin/composition-profile.js';
const args=process.argv.slice(2);if(args.length!==4||args[0]!=='--input'||args[2]!=='--sha256'||!/^[0-9a-f]{64}$/.test(args[3]!))throw Error('Usage: g3-rejoin-profile --input apps/desktop/current-input.json --sha256 SHA256');
const r=await materializeRejoinComposition({path:args[1]!,sha256:args[3]!});
console.log(JSON.stringify({runtimeApproval:r.runtimeApproval,desktopApproval:r.desktopApproval,runtimeId:r.id,runtimeFiles:r.requiredLocalFiles.length,nativeAbi:r.localProtocol.brokerAbiVersion,compositionOnly:true,installApproved:false,actualNoitaEvidence:false}));
