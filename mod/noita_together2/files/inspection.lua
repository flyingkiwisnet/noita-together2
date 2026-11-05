-- Candidate actual-game observation collector. Never called by a native worker.
local M={}
local MOD_LIMIT,ID_LIMIT,BODY_LIMIT=256,128,32768
local function valid_utf8(s)
    local i=1
    while i<=#s do
        local c=s:byte(i);local n,min,max=0,128,191
        if c<128 then if c<32 or c==127 then return false end;n=0
        elseif c>=194 and c<=223 then n=1
        elseif c>=224 and c<=239 then n=2;if c==224 then min=160 elseif c==237 then max=159 end
        elseif c>=240 and c<=244 then n=3;if c==240 then min=144 elseif c==244 then max=143 end
        else return false end
        if i+n>#s then return false end
        if c==194 and s:byte(i+1)<=159 then return false end -- Unicode C1 controls
        for j=1,n do local b=s:byte(i+j);if b<(j==1 and min or 128) or b>(j==1 and max or 191) then return false end end
        i=i+n+1
    end
    return true
end
local function observed_seed()
    local ok,value=pcall(StatsGetValue,"world_seed")
    if not ok then return nil end
    if type(value)=="number" then
        if value<0 or value~=math.floor(value) or value>9007199254740991 then return nil end
        value=string.format("%.0f",value)
    end
    if type(value)~="string" or #value==0 or #value>20 or not value:match("^%d+$") or (#value>1 and value:sub(1,1)=="0") then return nil end
    if #value==20 and value>"18446744073709551615" then return nil end
    return value
end
local function observed_mods()
    local ok,values=pcall(ModGetActiveModIDs)
    if not ok or type(values)~="table" then return {},"mods_unavailable" end
    local count=0
    for key in pairs(values) do
        if type(key)~="number" or key<1 or key~=math.floor(key) then return {},"invalid_inventory" end
        if key>MOD_LIMIT then return {},"inventory_limit" end
        count=count+1
    end
    local mods={}
    for index=1,count do
        local id=rawget(values,index)
        if type(id)~="string" or #id==0 or #id>ID_LIMIT or not valid_utf8(id) then return {},"invalid_inventory" end
        mods[index]=id
    end
    return mods,nil
end
function M.collect(loaded,protected)
    local seed=observed_seed();local mods,failure=observed_mods()
    if not seed then failure="seed_unavailable" end
    local beta_ok,beta=pcall(GameIsBetaBuild)
    if not beta_ok or type(beta)~="boolean" then beta=nil end
    local report={actualSeed=seed,mods=mods,complete=failure==nil,error=failure,beta=beta,loaded=loaded==true,protected=protected==true,modVersion="nt2-g2-inspect-1",bridgeVersion="g2-inspect-1"}
    if type(NT2_RACE_DETECTORS)=='table'then
        report.raceClassicAvailable=NT2_RACE_DETECTORS.classic==true;report.raceMapAvailable=NT2_RACE_DETECTORS.map==true
        local ok,value=pcall(SessionNumbersGetValue,'NEW_GAME_PLUS_COUNT');report.raceNg0=ok and(value=='0'or value==0)or false
    end
    if GlobalsGetValue and GlobalsGetValue('nt2_race_active','0')=='1'and GlobalsGetValue('nt2_race_detector_failure','')~=''then report.complete=false;report.error='collection_failed'end
    return report
end
local function quote(value)
    return '"'..value:gsub('[%z\1-\31\\"]',function(c)
        if c=='"' then return '\\"' elseif c=='\\' then return '\\\\' else return string.format('\\u%04x',c:byte()) end
    end)..'"'
end
function M.encode(report)
    local mods={};for i,id in ipairs(report.mods) do mods[i]=quote(id) end
    local encoded='{"actualSeed":'..(report.actualSeed and quote(report.actualSeed) or 'null')..',"mods":['..table.concat(mods,',')..'],"complete":'..tostring(report.complete)..',"error":'..(report.error and quote(report.error) or 'null')..',"beta":'..(report.beta==nil and 'null' or tostring(report.beta))..',"loaded":'..tostring(report.loaded)..',"protected":'..tostring(report.protected)..',"modVersion":'..quote(report.modVersion)..',"bridgeVersion":'..quote(report.bridgeVersion)..'}'
    if report.raceNg0~=nil then encoded=encoded:sub(1,-2)..',"raceClassicAvailable":'..tostring(report.raceClassicAvailable)..',"raceMapAvailable":'..tostring(report.raceMapAvailable)..',"raceNg0":'..tostring(report.raceNg0)..'}'end
    if #encoded>BODY_LIMIT then
        -- The complete inventory cannot fit. Send a distinct ineligible failure, never a truncated success.
        report={actualSeed=report.actualSeed,mods={},complete=false,error="inventory_limit",beta=report.beta,loaded=report.loaded,protected=report.protected,modVersion=report.modVersion,bridgeVersion=report.bridgeVersion}
        return M.encode(report),false
    end
    return encoded,true
end
function M.content(report)
    -- Stable generation/inventory identity; expected protected-to-playing phase is separate.
    return M.encode({actualSeed=report.actualSeed,mods=report.mods,complete=true,error=nil,beta=report.beta,loaded=true,protected=true,modVersion=report.modVersion,bridgeVersion=report.bridgeVersion})
end
return M
