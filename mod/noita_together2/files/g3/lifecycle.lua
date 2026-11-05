local M={}
function M.new(movement,ghosts,transport,validate)
    local self={scope=nil,playing=false,generation=0}
    function self:stop()
        self.generation=self.generation+1;self.scope=nil;self.playing=false
        movement:reset(nil);ghosts:clear();transport:reset(nil)
    end
    -- Scope/release/roster come from existing fenced Start, not from a remote peer.
    function self:start(scope,roster,matching_release)
        if self.scope==scope and self.playing and matching_release==true then return true end
        self:stop()
        if matching_release~=true or not ghosts:bind(scope,roster) then return false end
        self.scope=scope;self.playing=true;movement:reset(scope);transport:reset(scope);return true
    end
    function self:receive(scope,poses)
        if not self.playing or scope~=self.scope or not validate.array(poses,validate.MAX_BATCH) then return false end
        local seen={}
        for _,pose in ipairs(poses) do
            if not validate.pose(pose) or not ghosts.roster[pose.actor] or seen[pose.actor] then return false end
            seen[pose.actor]=true
        end
        for _,pose in ipairs(poses) do if not transport:pose(scope,pose) then return false end end
        return true
    end
    function self:tick(frame)
        if not self.playing then return end
        local scope=self.scope
        transport:drain(scope,function(pose) ghosts:receive(scope,pose,frame) end)
        ghosts:tick(frame,true)
    end
    return self
end
return M
