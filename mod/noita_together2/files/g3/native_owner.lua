-- Game-thread only. Fixed native batch -> existing bounded local modules.
local M={}
function M.new(bridge,context,ffi,api,validate,movement_module,ghost_module)
    local scope=""
    for i=0,15 do scope=scope..string.format("%02x",context[i]) end
    local movement=movement_module.new(api.movement(),validate);movement:reset(scope)
    local ghost_api=api.ghosts();local ghosts=ghost_module.new(ghost_api,validate);ghosts:bind(scope,{})
    local debug=api.debug and api.debug(bridge,context,ffi) or nil
    local last_copied,last_roster,last_poses=0,0,0
    local reply=ffi.new("uint8_t[864]");local read_rejected=false
    local function pause()
        for _,ghost in pairs(ghosts.ghosts) do if ghost_api.alive(ghost.entity) then ghost_api.remove(ghost.entity) end end
        ghosts.ghosts={};ghosts.count=0 -- Keep roster/sequence watermarks across temporary holds.
    end
    local function reject()read_rejected=true;if debug then debug:error(6)end;pause()end
    local function integer(offset)
        local value=tonumber(reply[offset])+tonumber(reply[offset+1])*256+tonumber(reply[offset+2])*65536+tonumber(reply[offset+3])*16777216
        return value>=2147483648 and value-4294967296 or value
    end
    local function milli(value) return value<0 and -math.floor(-value*1000+0.5) or math.floor(value*1000+0.5) end
    local self={debug=debug,sampled=0,movement_accepted=0,peer_replies=0,nonempty_batches=0,clock_invalid=0,pose_invalid=0,player_or_sequence=0,last_diagnostic_frame=-300,holds=0,recoveries=0,hold_reason=0}
    function self:set_visibility(hidden,opacity)ghosts:set_visibility(hidden,opacity)end
    function self:set_names(names)return ghosts:set_names(scope,names)end
    function self:entities()
        local result={};for actor,g in pairs(ghosts.ghosts)do if g.entity and ghost_api.alive(g.entity)then result[actor]=g.entity end end;return result
    end
    function self:positions()
        local result={};local now=api.movement().time();for actor,g in pairs(ghosts.ghosts)do if now>=g.seconds and now-g.seconds<=ghost_module.STALE_SECONDS and ghosts.roster[actor]then result[actor]={x=g.x,y=g.y}end end;return result
    end
    local function count(name) self[name]=math.min(2147483647,self[name]+1) end
    local function heartbeat_reason()
        local reason=bridge.nt2_g3_heartbeat_diagnostic and tonumber(bridge.nt2_g3_heartbeat_diagnostic(context)) or 0
        return validate.integer(reason,0,9) and reason or 0
    end
    local function diagnostic(frame,playing)
        -- Observe before rate limiting: a brief hold may recover before emission.
        if self.last_diagnostic_playing~=playing then
            if not playing then
                count("holds");local reason=heartbeat_reason()
                if reason~=0 then self.hold_reason=reason end -- Last observed failure survives a later ACK.
            elseif self.last_diagnostic_playing==false then count("recoveries") end
            self.last_diagnostic_playing=playing;self.diagnostic_pending=true
        end
        if api.diagnostics and validate.integer(frame,0,2147483647) and frame-self.last_diagnostic_frame>=60
            and (frame%300==0 or self.diagnostic_pending) then
            self.last_diagnostic_frame=frame;self.diagnostic_pending=false
            self.heartbeat_failure=heartbeat_reason()
            api.diagnostics(playing,self,ghosts.count)
        end
    end
    local function tick(self,player,frame,playing)
        if not playing then pause();diagnostic(frame,false);return end
        local pose,reason=movement:sample(scope,"local",player,frame,true)
        if reason then count(reason) end
        if pose then count("sampled");if bridge.nt2_g3_move(context,pose.sequence,milli(pose.x),milli(pose.y),milli(pose.vx),milli(pose.vy),pose.facing<0 and 1 or 0)==1 then count("movement_accepted") end end
        diagnostic(frame,true)
        local copied=bridge.nt2_g3_peers(context,reply,864);last_copied=copied
        if copied==0 and not read_rejected and bridge.nt2_g3_status and bridge.nt2_g3_status(context)==2 then
            -- No new batch grants no new roster/pose/age. Existing playback
            -- retains its own unchanged receiver-clock staleness boundary.
            ghosts:tick(frame,true);return
        end
        if copied~=864 then reject();return end
        if reply[0]~=78 or reply[1]~=71 or reply[2]~=51 or reply[3]~=82 or reply[4]~=2 or reply[5]~=1 or reply[6]>4 or reply[7]~=0 then reject();return end
        for i=0,15 do if reply[8+i]~=context[i] then reject();return end end
        if integer(24)<1 or integer(24)>200 then reject();return end
        for i=29,31 do if reply[i]~=0 then reject();return end end
        local roster,seen={},{}
        if reply[28]>32 then reject();return end
        for index=0,31 do
            local actor="";for i=0,15 do actor=actor..string.format("%02x",reply[32+index*16+i]) end
            if index<tonumber(reply[28]) then
                if actor==string.rep("0",32) or seen[actor] then reject();return end
                seen[actor]=true;roster[#roster+1]=actor
            elseif actor~=string.rep("0",32) then reject();return end
        end
        -- Validate the whole bounded batch before creating any game entities.
        for index=0,tonumber(reply[6])-1 do
            local offset=560+index*80
            local actor="";for i=0,15 do actor=actor..string.format("%02x",reply[544+index*80+i]) end
            if not seen[actor] then reject();return end
            if reply[offset]~=78 or reply[offset+1]~=84 or reply[offset+2]~=71 or reply[offset+3]~=51
                or reply[offset+4]~=1 or reply[offset+5]~=0 or reply[offset+6]~=1 or reply[offset+7]~=0
                or reply[offset+44]>1 then reject();return end
            for i=0,15 do if reply[offset+8+i]~=context[i] then reject();return end end
            for i=45,63 do if reply[offset+i]~=0 then reject();return end end
        end
        if not ghosts:reconcile(scope,roster) then reject();return end
        read_rejected=false;last_roster=#roster;last_poses=tonumber(reply[6])
        count("peer_replies");if reply[6]>0 then count("nonempty_batches") end
        for index=0,tonumber(reply[6])-1 do
            local base=544+index*80;local actor=""
            for i=0,15 do actor=actor..string.format("%02x",reply[base+i]) end
            local offset=base+16
            local sequence=integer(offset+24);if sequence<0 then sequence=sequence+4294967296 end
            if reply[offset+44]<=1 then
                if not ghosts.roster[actor] then reject();return end
                ghosts:receive(scope,{actor=actor,sequence=sequence,x=integer(offset+28)/1000,y=integer(offset+32)/1000,
                    vx=integer(offset+36)/1000,vy=integer(offset+40)/1000,rotation=0,facing=reply[offset+44]==1 and -1 or 1},frame)
            end
        end
        ghosts:tick(frame,true)
    end
    function self:tick(player,frame,playing)
        tick(self,player,frame,playing)
        if debug then debug:observe(frame,playing,last_copied,last_roster,last_poses,ghosts.count)end
    end
    function self:close() pause();ghosts:clear() end
    return self
end
return M
