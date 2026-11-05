// Remove only inactive, generated client fault-fixture namespaces. Never public/server fixtures.
import pg from 'pg';
const pool=new pg.Pool({connectionString:'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1',max:1});
const tx=await pool.connect();let removed=0;
try{
    const rows=(await tx.query("SELECT nspname FROM pg_namespace WHERE nspname LIKE 'client_g1_%' OR nspname LIKE 'client_g2_%'")).rows;
    for(const {nspname:name}of rows){if(!/^client_g[12]_[a-f0-9]{32}$/.test(name))continue;const key='nt2-g1-service:'+name;
        if(!(await tx.query('SELECT pg_try_advisory_lock(hashtext($1)) AS held',[key])).rows[0].held)continue;
        try{const identity=(await tx.query(`SELECT service_id FROM "${name}".service_identity`)).rows;if(identity.length!==1||identity[0].service_id!==(name.startsWith('client_g2_')?'nt2-client-g2-test':'nt2-client-g1-test'))continue;await tx.query(`DROP SCHEMA "${name}" CASCADE`);removed++;}
        finally{await tx.query('SELECT pg_advisory_unlock(hashtext($1))',[key]);}
    }
}finally{tx.release();await pool.end();}
console.log(`Removed ${removed} inactive generated client test namespaces; shared service data preserved.`);
