-- Local game-side bounds, not a wire schema or server authority implementation.
local M={MAX_GHOSTS=32,MAX_BATCH=4,POSITION_LIMIT=2147483.647}
function M.integer(v,lo,hi) return type(v)=="number" and v==v and v>=lo and v<=hi and v%1==0 end
function M.number(v,limit) return type(v)=="number" and v==v and v>=-limit and v<=limit end
function M.id(v) return type(v)=="string" and #v>0 and #v<=128 and not v:find("[^%w:_%-%.]") end
function M.array(v,max)
    if type(v)~="table" or getmetatable(v) then return nil end
    local n=0
    for k in pairs(v) do n=n+1;if n>max or not M.integer(k,1,max) then return nil end end
    for i=1,n do if v[i]==nil then return nil end end
    return n
end
function M.pose(v)
    return type(v)=="table" and M.id(v.actor) and M.integer(v.sequence,1,4294967295)
        and M.number(v.x,M.POSITION_LIMIT) and M.number(v.y,M.POSITION_LIMIT)
        and M.number(v.vx,M.POSITION_LIMIT) and M.number(v.vy,M.POSITION_LIMIT)
        and M.number(v.rotation,math.pi*2) and (v.facing==1 or v.facing==-1)
end
return M

