-- Internal bounded scheduling. Generated wire/authentication adapter remains separate.
local M={}
function M.new(validate)
    local self={scope=nil,poses={},pose_count=0,controls={},control_count=0,errors=0}
    function self:reset(scope) self.scope=scope;self.poses={};self.pose_count=0;self.controls={};self.control_count=0;self.last_actor=nil end
    function self:pose(scope,pose)
        if not self.scope or scope~=self.scope or not validate.pose(pose) then return false end
        local old=self.poses[pose.actor]
        if old and pose.sequence<=old.sequence then return false end
        if not old and self.pose_count>=validate.MAX_GHOSTS then return false end
        self.poses[pose.actor]={actor=pose.actor,sequence=pose.sequence,x=pose.x,y=pose.y,rotation=pose.rotation,facing=pose.facing,vx=pose.vx,vy=pose.vy}
        if not old then self.pose_count=self.pose_count+1 end
        return true
    end
    -- Functions originate in fixed local handlers, never network payloads.
    function self:control(scope,handler)
        if not self.scope or scope~=self.scope or type(handler)~="function" or self.control_count>=32 then return false end
        self.controls[#self.controls+1]=handler;self.control_count=self.control_count+1;return true
    end
    function self:drain(scope,on_pose)
        if not self.scope or scope~=self.scope then return 0 end
        local count=0
        for i=1,math.min(2,self.control_count) do
            local handler=table.remove(self.controls,1);self.control_count=self.control_count-1
            if not pcall(handler) then self.errors=math.min(self.errors+1,2147483647) end
            count=count+1
        end
        local actors={};for actor in pairs(self.poses) do actors[#actors+1]=actor end;table.sort(actors)
        local first=1
        if self.last_actor then for i,actor in ipairs(actors) do if actor>self.last_actor then first=i;break end end end
        for i=1,math.min(validate.MAX_BATCH,#actors) do
            local actor=actors[(first+i-2)%#actors+1];local pose=self.poses[actor];self.poses[actor]=nil;self.pose_count=self.pose_count-1;self.last_actor=actor
            if not pcall(on_pose,pose) then self.errors=math.min(self.errors+1,2147483647) end
            count=count+1
        end
        return count
    end
    return self
end
return M

