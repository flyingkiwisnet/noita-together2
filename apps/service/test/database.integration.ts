import test from 'node:test';
import assert from 'node:assert/strict';
import pg from 'pg';
test('isolated PostgreSQL fixture supports atomic rollback and uniqueness without permanent schema', async () => {
  const pool = new pg.Pool({host:'127.0.0.1',port:54327,database:'nt2_g0',user:'nt2_g0',password:'local-fixture-only',connectionTimeoutMillis:3000});
  const client = await pool.connect();
  try {
    const version = await client.query('SHOW server_version'); console.log(`G0 PostgreSQL ${version.rows[0].server_version}`);
    await client.query('BEGIN');
    await client.query('CREATE TEMP TABLE nt2_g0_fixture (id text PRIMARY KEY, value integer CHECK (value >= 0)) ON COMMIT DROP');
    await client.query("INSERT INTO nt2_g0_fixture VALUES ('receipt-1',10)");
    await client.query('SAVEPOINT boundary');
    await assert.rejects(client.query("INSERT INTO nt2_g0_fixture VALUES ('receipt-1',10)"),{code:'23505'});
    await client.query('ROLLBACK TO SAVEPOINT boundary');
    await assert.rejects(client.query('UPDATE nt2_g0_fixture SET value = -1'),{code:'23514'});
    await client.query('ROLLBACK TO SAVEPOINT boundary');
    assert.equal((await client.query('SELECT value FROM nt2_g0_fixture')).rows[0].value,10);
    await client.query('ROLLBACK');
    assert.equal((await client.query("SELECT to_regclass('pg_temp.nt2_g0_fixture') AS relation")).rows[0].relation,null);
  } finally { await client.query('ROLLBACK'); client.release(); await pool.end(); }
});
