-- Local effect guard only. Server owns reservation, ledger, expiry and compensation rules.
local M={MAX_RECEIPTS=256,MAX_GOLD=2147483647}
function M.new(api,journal,validate)
    local self={receipts={},count=0,scope=nil}
    function self:bind(scope,restored)
        self.scope=scope;self.receipts={};self.count=0
        if type(scope)~="string" or #scope==0 or #scope>512 or not validate.array(restored or {},M.MAX_RECEIPTS) then self.scope=nil;return false end
        for _,record in ipairs(restored or {}) do
            if type(record)~="table" or self.count>=M.MAX_RECEIPTS or not validate.id(record.id) or record.scope~=scope or self.receipts[record.id]
                or type(record.digest)~="string" or #record.digest~=64 or record.digest:find("[^a-f0-9]") then self.scope=nil;return false end
            -- Restored app evidence alone never proves the loaded save contains this effect.
            self.receipts[record.id]={id=record.id,digest=record.digest,scope=scope,state="uncertain"};self.count=self.count+1
        end
        return true
    end
    function self:apply(op,authorized)
        if not self.scope or type(op)~="table" or op.scope~=self.scope or authorized~=true
            or not validate.id(op.id) or type(op.digest)~="string" or #op.digest~=64 or op.digest:find("[^a-f0-9]")
            or not validate.integer(op.amount,1,1000000) or (op.direction~="deposit" and op.direction~="withdraw") then return "rejected" end
        local prior=self.receipts[op.id]
        if prior then return prior.digest==op.digest and prior.state or "conflict" end
        if self.count>=M.MAX_RECEIPTS then return "receipt_limit" end
        local ok,before=pcall(api.balance)
        if not ok or not validate.integer(before,0,M.MAX_GOLD) then return "rejected" end
        local after=op.direction=="deposit" and before-op.amount or before+op.amount
        if not validate.integer(after,0,M.MAX_GOLD) then return "rejected" end
        local receipt={id=op.id,digest=op.digest,scope=op.scope,direction=op.direction,amount=op.amount,before=before,after=after,state="intent"}
        self.receipts[op.id]=receipt;self.count=self.count+1
        local wrote,result=pcall(journal.write,receipt)
        if not wrote or result~=true then receipt.state="blocked_no_effect";return receipt.state end
        receipt.state="uncertain"
        local changed,result=pcall(api.set_balance,after)
        if not changed or result==false then return receipt.state end
        local read,balance=pcall(api.balance)
        if not read or balance~=after then return receipt.state end
        local applied={id=receipt.id,digest=receipt.digest,scope=receipt.scope,direction=receipt.direction,amount=receipt.amount,before=before,after=after,state="applied_session"}
        local saved,ack=pcall(journal.write,applied)
        if saved and ack==true then receipt.state="applied_session" end
        return receipt.state
    end
    -- Production asynchronous handshake: snapshot first, native fsync second,
    -- then one guarded mutation. No disk/network work runs in a game callback.
    function self:offer(op,attempt)
        if not self.scope or type(op)~='table'or op.scope~=self.scope or not validate.id(op.id)
            or type(op.digest)~='string'or #op.digest~=64 or op.digest:find('[^a-f0-9]')
            or not validate.integer(op.amount,1,1000000)or (op.direction~='deposit'and op.direction~='withdraw')then return nil end
        local prior=self.receipts[op.id];if prior then return prior.state=='prepared'and prior or nil end
        if self.count>=M.MAX_RECEIPTS then return nil end
        local ok,snapshot=pcall(api.snapshot);if not ok or not snapshot or not validate.integer(snapshot.balance,0,M.MAX_GOLD)then return nil end
        local after=op.direction=='deposit'and snapshot.balance-op.amount or snapshot.balance+op.amount
        if not validate.integer(after,0,M.MAX_GOLD)then return nil end
        local receipt={id=op.id,digest=op.digest,scope=op.scope,direction=op.direction,amount=op.amount,before=snapshot.balance,after=after,snapshot=snapshot,attempt=attempt,state='prepared'}
        self.receipts[op.id]=receipt;self.count=self.count+1;return receipt
    end
    function self:permit(op)
        local r=op and self.receipts[op.id]
        if not r or r.digest~=op.digest or r.attempt~=op.attempt or r.scope~=self.scope or r.state~='prepared'
            or r.direction~=op.direction or r.amount~=op.amount then return nil end
        local ready,now=pcall(api.snapshot)
        if not ready or not now or not validate.integer(now.balance,0,M.MAX_GOLD)then return nil end
        if now.player~=r.snapshot.player or now.wallet~=r.snapshot.wallet then
            if type(api.rebind_snapshot)~='function'or api.rebind_snapshot(r.snapshot,now)~=true then return nil end
        end
        local after=r.direction=='deposit'and now.balance-r.amount or now.balance+r.amount
        if not validate.integer(after,0,M.MAX_GOLD)then return nil end
        r.snapshot=now;r.before=now.balance;r.after=after
        -- Set uncertainty BEFORE invoking any engine write, even if it throws.
        r.state='uncertain'
        local ok,value=pcall(api.commit,r.snapshot,r.after)
        if ok and value==true then r.state='applied_session'end
        return r
    end
    return self
end
return M

