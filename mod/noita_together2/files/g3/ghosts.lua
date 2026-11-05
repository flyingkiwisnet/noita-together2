-- Receiver-clock playback: no claim of synchronized or per-frame source timestamps.
local M={DISPLAY_DELAY=0.18,MAX_SAMPLES=8,GAP_RESET=0.5,TELEPORT_DISTANCE=256,STALE_SECONDS=2}
function M.new(api,validate)
    local self={scope=nil,roster={},names={},ghosts={},count=0,sequences={},retired={},hidden=false,opacity=1}
    function self:set_names(scope,names)
        if scope~=self.scope or type(names)~='table'then return false end
        local current,count={},0;for actor,name in pairs(names)do count=count+1;if count>validate.MAX_GHOSTS or not validate.id(actor)or type(name)~='string'or #name>396 then return false end;current[actor]=name end
        self.names=current;return true
    end
    function self:set_visibility(hidden,opacity)
        self.hidden=hidden==true;if type(opacity)=='number' and opacity==opacity then self.opacity=math.max(0,math.min(1,opacity))end
        for _,g in pairs(self.ghosts)do if self.hidden and g.entity then if api.alive(g.entity)then api.remove(g.entity)end;g.entity=nil end end
    end
    function self:clear()
        for _,g in pairs(self.ghosts) do if api.alive(g.entity) then api.remove(g.entity) end end
        self.ghosts={};self.count=0;self.scope=nil;self.roster={};self.names={};self.sequences={};self.retired={}
    end
    -- Complete roster from the authenticated native adapter, distinct from pose gaps.
    function self:reconcile(scope,roster)
        if scope~=self.scope or not validate.array(roster,validate.MAX_GHOSTS) then return false end
        local seen={};for _,id in ipairs(roster) do if not validate.id(id) or seen[id] then return false end;seen[id]=true end
        for id in pairs(self.roster) do if not seen[id] then
            local g=self.ghosts[id];if g then if api.alive(g.entity) then api.remove(g.entity) end;self.ghosts[id]=nil;self.count=self.count-1 end
            self.retired[#self.retired+1]=id
        end end
        self.roster=seen
        while #self.retired>validate.MAX_GHOSTS do local id=table.remove(self.retired,1);if not seen[id] then self.sequences[id]=nil end end
        return true
    end
    function self:bind(scope,roster)
        if type(scope)~="string" or #scope==0 or #scope>512 or not validate.array(roster,validate.MAX_GHOSTS) then return false end
        local seen={}
        for _,id in ipairs(roster) do if not validate.id(id) or seen[id] then return false end;seen[id]=true end
        if self.scope==scope then
            -- A repeated binding cannot erase sequence history. Roster replacement
            -- needs a new owner context; a peer update cannot mint that context.
            for id in pairs(self.roster) do if not seen[id] then return false end end
            for id in pairs(seen) do if not self.roster[id] then return false end end
            return true
        end
        self:clear()
        self.scope=scope;self.roster=seen;return true
    end
    -- Caller must be the authenticated generated-contract adapter. No sender-controlled paths.
    function self:receive(scope,pose,frame)
        if scope~=self.scope or not self.scope or not validate.pose(pose) or not self.roster[pose.actor]
            or not validate.integer(frame,0,2147483647) then return false end
        local timed,seconds=pcall(api.time)
        if not timed or not validate.number(seconds,1000000000) then return false end
        if self.sequences[pose.actor] and pose.sequence<=self.sequences[pose.actor] then return false end
        local g=self.ghosts[pose.actor]
        if not g then
            if self.count>=validate.MAX_GHOSTS then return false end
            local entity=not self.hidden and api.spawn(pose.x,pose.y)or nil
            if not self.hidden and (not entity or not api.alive(entity)) then return false end
            g={entity=entity,x=pose.x,y=pose.y,samples={}};self.ghosts[pose.actor]=g;self.count=self.count+1
        end
        local samples=g.samples;local previous=samples[#samples]
        local distance=(pose.x-g.x)^2+(pose.y-g.y)^2
        if distance>M.TELEPORT_DISTANCE^2 then
            -- Teleports deliberately reset playback rather than sweeping through terrain.
            samples={};g.samples=samples;g.x=pose.x;g.y=pose.y
        elseif previous and (seconds-previous.at>M.GAP_RESET or seconds-M.DISPLAY_DELAY>previous.at) then
            -- Hold across a missing-data gap, then resume from the displayed point.
            samples={{at=seconds-M.DISPLAY_DELAY,x=g.x,y=g.y,rotation=g.rotation or 0,facing=g.facing or pose.facing}};g.samples=samples
        end
        local at=seconds;local index=#samples+1
        if #samples>0 and at-samples[#samples].at<0.01 then
            -- Same receiver-frame bursts coalesce, without inventing source time.
            index=#samples;at=samples[index].at
        end
        samples[index]={at=at,x=pose.x,y=pose.y,rotation=pose.rotation,facing=pose.facing}
        while #samples>M.MAX_SAMPLES do table.remove(samples,1) end
        g.sequence=pose.sequence;g.tx=pose.x;g.ty=pose.y;g.rotation=pose.rotation;g.facing=pose.facing;g.frame=frame;g.seconds=seconds
        self.sequences[pose.actor]=pose.sequence
        return true
    end
    function self:tick(frame,playing)
        if not playing then self:clear();return end
        local timed,seconds=pcall(api.time)
        if not timed or not validate.number(seconds,1000000000) then self:clear();return end
        for id,g in pairs(self.ghosts) do
            if frame<g.frame or seconds<g.seconds or (g.render_at and seconds<g.render_at) or seconds-g.seconds>M.STALE_SECONDS or g.entity and not api.alive(g.entity) then
                if api.alive(g.entity) then api.remove(g.entity) end
                self.ghosts[id]=nil;self.count=self.count-1
            else
                local target=seconds-M.DISPLAY_DELAY;local samples=g.samples
                while #samples>2 and samples[2].at<=target do table.remove(samples,1) end
                local a,b=samples[1],samples[2]
                if a then
                    local fraction=b and math.max(0,math.min(1,(target-a.at)/(b.at-a.at))) or 0
                    g.x=a.x+((b and b.x or a.x)-a.x)*fraction;g.y=a.y+((b and b.y or a.y)-a.y)*fraction
                    local choice=b and fraction>=0.5 and b or a;g.rotation=choice.rotation;g.facing=choice.facing
                end
                if not self.hidden then
                    if not g.entity then g.entity=api.spawn(g.x,g.y)end
                    if g.entity and api.alive(g.entity)then api.position(g.entity,g.x,g.y,g.rotation,g.facing);if api.name then pcall(api.name,g.entity,self.names[id])end;if api.opacity then api.opacity(g.entity,self.opacity)end end
                end
                g.render_at=seconds
            end
        end
    end
    return self
end
return M
