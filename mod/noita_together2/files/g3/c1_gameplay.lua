-- Gameplay arithmetic adapted from Noita Together core/files/scripts/utils.lua,
-- append/heart{,_better}_append.lua and append/status_effects/hearty_*.lua.
-- Values use Noita HP units (one unit = 25 displayed HP). Keep legacy quirks.
local M={}
M.team_perks={EXTRA_MONEY=true,STRONG_KICK=true,GLOBAL_GORE=true,
 HEARTS_MORE_EXTRA_HP=true,ATTRACT_ITEMS=true,EXTRA_SHOP_ITEM=true,
 GENOME_MORE_HATRED=true,GENOME_MORE_LOVE=true,PEACE_WITH_GODS=true}
function M.heart(hp,max_hp,cap,count,multiplier,large,recipient)
 local minimum=(large and not recipient)and 0.32 or 0.16
 local amount=(large and not recipient)and 2 or 1
 local added=math.max(minimum,count>0 and amount/count or minimum)*multiplier
 local maximum=max_hp+added
 if cap>0 then maximum=math.min(maximum,cap)end
 -- Collector receives no healing. Remote large hearts are ordinary hearts;
 -- recipient HP intentionally is not clamped to max_hp_cap.
 return recipient and hp+added or hp,maximum,count==0
end
function M.share_perk(rules,id,restored)
 return not restored and not(rules.NT_ban_perks and id=='GLASS_CANNON')
  and(rules.NT_sync_perks or rules.NT_team_perks and M.team_perks[id]==true)or false
end
function M.orb_allowed(rules,count,owned)
 if not rules.NT_sync_orbs then return false,'sharing_disabled'end
 if rules.NT_sync_orb_count~=0 and count>=rules.NT_sync_orb_count then return false,'orb_cap'end
 if owned then return false,'orb_owned'end
 return true
end
function M.penalty(hp,max_hp,dying)
 local maximum=max_hp*0.8
 if maximum<1 then return hp,max_hp,true end
 return dying and maximum or math.min(hp,maximum),maximum,false
end
function M.heartache_start(hp,max_hp,alternate,boss,orb_count)
 if max_hp<=0.4 or boss and orb_count>=33 then return hp,max_hp,0 end
 if alternate then return math.max(hp*0.5,0.04),max_hp,hp end
 return math.max(max_hp*0.5*(hp/max_hp),0.04),max_hp*0.5,max_hp
end
function M.heartache_end(hp,max_hp,stored,alternate)
 if stored<=0 then return hp,max_hp end
 if alternate then return math.min(hp*2,max_hp),max_hp end
 return max_hp*2*(hp/max_hp),max_hp*2
end
function M.fungal_gate(frame,last_frame,gold,iteration)
 if frame<last_frame+60*60*5 then return false,'fungal_cooldown',iteration end
 if gold then return false,'gold_world',iteration end
 -- Legacy increments before checking the old value, accepting iteration 20.
 if iteration>20 then return false,'fungal_limit',iteration+1 end
 return true,nil,iteration+1
end
return M
