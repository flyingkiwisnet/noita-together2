import pg from 'pg';
export const candidateDatabaseUrl='postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';
export async function databasePrerequisite(url=process.env.NT2_G2_DATABASE_URL??candidateDatabaseUrl,probe?:()=>Promise<void>){
 let pool:pg.Pool|undefined;
 try{
  if(probe)await probe();else{pool=new pg.Pool({connectionString:url,max:1,connectionTimeoutMillis:2000,query_timeout:2000});await pool.query('SELECT 1');}
  return {status:'healthy',canStartExistingCompose:false};
 }catch(error){
  const code=error&&typeof error==='object'&&'code'in error?error.code:undefined;
  return {status:code==='ECONNREFUSED'?'absent':'unhealthy',canStartExistingCompose:code==='ECONNREFUSED'&&url===candidateDatabaseUrl};
 }finally{await pool?.end();}
}
