// Real local SQL/TLS fixture; game observations are synthetic. No native run.
import test from'node:test';import assert from'node:assert/strict';import{b1Fixture}from'./b1-fixture-support.js';import{readRoomAdmissions}from'../src/g3/admission.js';
import{serializeAdmission}from'../src/g3/connection.js';
// Previous SQL transport shape only; authority policy is never duplicated.
// Feed its real pg Date rows through the same reader to compare serialization.
async function previousFacts(tx:any,room:string,rejoin:boolean){
 const r=(await tx.query('SELECT r.*,to_jsonb(n) AS admission_run,to_jsonb(x) AS admission_release FROM rooms r LEFT JOIN runs n ON n.id=r.current_run_id LEFT JOIN inspected_releases x ON x.run_id=n.id WHERE r.id=$1',[room])).rows[0],n=r.admission_run,x=r.admission_release;
 r.admission_owners=(await tx.query(`SELECT a.*,s.epoch AS game_epoch,s.revoked_at AS game_revoked,s.parent_id,
 d.epoch AS desktop_epoch,d.revoked_at AS desktop_revoked,d.family_id AS desktop_family,
 f.id AS family_id,f.account_id,f.device_id AS owner_device,f.expires_at AS family_until,f.revoked_at AS family_revoked,
 u.disabled,EXISTS(SELECT 1 FROM room_bans b WHERE b.room_id=$2 AND b.account_id=f.account_id) AS banned,p.display_name,p.kind AS principal_kind,p.guest_expires_at,
 m.presence_until AS desktop_until,m.selected,m.account_id AS member_account,w.body,w.revision,w.world_token,w.content_digest,w.captured_at,e.entered_playing_at
 FROM attachments a JOIN sessions s ON s.id=a.game_session_id AND s.kind='game'
 JOIN sessions d ON d.id=a.desktop_id AND d.kind='desktop' JOIN session_families f ON f.id=s.family_id
 JOIN accounts u ON u.id=f.account_id JOIN principal_profiles p ON p.principal_id=u.id
 LEFT JOIN memberships m ON m.desktop_id=d.id AND m.room_id=$2 LEFT JOIN world_observations w ON w.attachment_id=a.id
 LEFT JOIN authorization_play_entries e ON e.attachment_id=a.id AND e.authorization_uuid=$3
 WHERE a.run_id=$1 AND (a.revoked_at IS NULL${rejoin?' OR EXISTS(SELECT 1 FROM rejoin_participants current_owner WHERE current_owner.attachment_id=a.id)':''}) ORDER BY a.id LIMIT 33`,[n.id,r.id,x.authorization_uuid])).rows;
 r.admission_participants=rejoin?(await tx.query('SELECT * FROM rejoin_participants WHERE run_id=$1',[n.id])).rows:undefined;return r;
}
test('Canonical admission single statement preserves both reader modes, fresh mutation facts and absolute expiry',async()=>{
 const f=await b1Fixture({itemTransfers:true});try{const db=f.x.fixture.service.db,now=f.x.fixture.service.auth.now(),room=f.scope(0).roomId!,attachment=f.scope(0).attachmentId!;
  await db.transaction(async tx=>{
   const read=async(rejoin:boolean,time=now)=>{const view=Object.create(db);Object.defineProperty(view,'rejoinEnabled',{value:rejoin});const query=tx.query;let calls=0;const observed=(...args:any[])=>{calls++;return(query as any).apply(tx,args);};tx.query=observed as any;try{const result=await readRoomAdmissions(tx,view,f.x.fixture.profile.compatibility,time,room);assert.equal(calls,1);return result;}finally{tx.query=query;}};
   for(const mode of [false,true]){const r=await read(mode);assert.equal(r.worlds.length,2);assert.ok(r.worlds.every(w=>w.decision.playing&&w.until>now));
    const facts=await previousFacts(tx,room,mode),view=Object.create(db);Object.defineProperty(view,'rejoinEnabled',{value:mode});const old=await readRoomAdmissions({query:async()=>({rows:[facts]})}as any,view,f.x.fixture.profile.compatibility,now,room);
    assert.deepEqual(r,old);for(let i=0;i<r.worlds.length;i++)assert.equal(JSON.stringify(serializeAdmission(r.worlds[i]!,{credentialId:'fixture-credential'}as any,'fixture-parity')),JSON.stringify(serializeAdmission(old.worlds[i]!,{credentialId:'fixture-credential'}as any,'fixture-parity')));
   }
   await tx.query("UPDATE world_observations SET body=jsonb_set(body,'{protected}','true') WHERE attachment_id=$1",[attachment]);
   for(const mode of [false,true]){const w=(await read(mode)).worlds.find(w=>w.owner.attachmentId===attachment)!;assert.equal(w.decision.playing,false);assert.equal(w.currentReport.protected,true);}
   await tx.query("UPDATE world_observations SET body=jsonb_set(body,'{protected}','false') WHERE attachment_id=$1",[attachment]);
   for(const mode of [false,true]){assert.equal((await read(mode)).worlds.find(w=>w.owner.attachmentId===attachment)!.decision.playing,true);const w=(await read(mode,now+31001)).worlds.find(w=>w.owner.attachmentId===attachment)!;assert.equal(w.decision.playing,false);assert.equal(w.decision.reason,'lease_expired');assert.equal(w.decision.remainingMs,0);}
   for(const scenario of ['missing-run','missing-release','ended']){
    await tx.query('SAVEPOINT admission_facts');
    if(scenario==='missing-run')await tx.query('UPDATE rooms SET current_run_id=NULL WHERE id=$1',[room]);
    else if(scenario==='missing-release'){await tx.query('DELETE FROM authorization_play_entries WHERE authorization_uuid IN (SELECT authorization_uuid FROM inspected_releases WHERE run_id=$1)',[f.scope(0).runId]);await tx.query('DELETE FROM inspected_releases WHERE run_id=$1',[f.scope(0).runId]);}
    else await tx.query("UPDATE rooms SET state='ended' WHERE id=$1",[room]);
    for(const mode of [false,true]){const r=await read(mode);assert.equal(r.terminal,true);assert.equal(r.normalEnd,scenario==='ended');assert.equal(r.worlds.length,0);}
    await tx.query('ROLLBACK TO SAVEPOINT admission_facts');await tx.query('RELEASE SAVEPOINT admission_facts');
   }
  });
 }finally{await f.close();}
});
