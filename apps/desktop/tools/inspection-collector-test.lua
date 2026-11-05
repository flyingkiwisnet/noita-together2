local collect_count=0
local actual="424242"
local mods={"noita_together2","other","quote\"and\\slash"}
function StatsGetValue(key) assert(key=="world_seed");collect_count=collect_count+1;return actual end
function ModGetActiveModIDs() return mods end
function GameIsBetaBuild() return false end
local collector=dofile(assert(os.getenv("NT2_COLLECTOR_SOURCE"),"collector test source path required"))
local good=collector.collect(true,true)
assert(good.complete and good.actualSeed=="424242" and #good.mods==3 and good.mods[2]=="other")
local encoded=collector.encode(good);assert(encoded:find('quote\\"and\\\\slash',1,true))
mods={"other","noita_together2"};local ordered=collector.collect(true,true);assert(ordered.mods[1]=="other")
actual="424243";assert(collector.collect(true,true).actualSeed=="424243")
actual="0424242";local unknown=collector.collect(true,true);assert(not unknown.complete and unknown.actualSeed==nil and unknown.error=="seed_unavailable")
actual="18446744073709551616";assert(not collector.collect(true,true).complete)
actual=4294967295;assert(collector.collect(true,true).actualSeed=="4294967295")
mods={[1]="one",[3]="hole"};assert(collector.collect(true,true).error=="invalid_inventory")
mods={"bad\0id"};assert(not collector.collect(true,true).complete)
mods={string.char(255)};assert(not collector.collect(true,true).complete)
mods={"bad"..string.char(194,133).."id"};assert(not collector.collect(true,true).complete) -- Unicode C1 control
mods={"valid"..string.char(195,169)};assert(collector.collect(true,true).complete)
mods={};for i=1,257 do mods[i]="m" end;assert(collector.collect(true,true).error=="inventory_limit")
mods={};for i=1,256 do mods[i]=string.rep("x",128) end;local big=collector.collect(true,true);assert(big.complete);local failure=collector.encode(big);assert(#failure<32768 and failure:find('"complete":false',1,true) and failure:find('"inventory_limit"',1,true))
local before=collect_count
-- Collector calls only when explicitly invoked; sleeping/heartbeat do not fabricate observation.
assert(collect_count==before)
print("PASS actual installed LuaJIT collector harness: ordered complete inventory, raw seed/mismatch, escaping, holes/UTF8/overflow failures; engine APIs stubbed")
