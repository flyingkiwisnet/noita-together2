local M={}
-- API is injected; sampling is game-thread only and carries no credentials.
function M.new(api,validate)
    local self={scope=nil,last_frame=-6,last_time=-math.huge,sequence=0}
    function self:reset(scope) self.scope=scope;self.last_frame=-6;self.last_time=-math.huge;self.sequence=0 end
    function self:sample(scope,actor,player,frame,playing)
        if not playing or not self.scope or scope~=self.scope or not validate.id(actor)
            or not validate.integer(frame,0,2147483647) or frame-self.last_frame<6 then return nil end
        if not api.alive(player) or self.sequence>=4294967295 then return nil,"player_or_sequence" end
        local timed,seconds=pcall(api.time)
        if not timed or not validate.number(seconds,1000000000) then return nil,"clock_invalid" end
        if seconds-self.last_time<0.1 then return nil end
        local ok,x,y,rotation,facing,vx,vy=pcall(api.position,player)
        local pose={actor=actor,sequence=self.sequence+1,x=x,y=y,rotation=rotation,facing=facing,vx=vx,vy=vy}
        if not ok or not validate.pose(pose) then return nil,"pose_invalid" end
        self.last_frame=frame;self.last_time=seconds;self.sequence=pose.sequence
        return pose
    end
    return self
end
return M

