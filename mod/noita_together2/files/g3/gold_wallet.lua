-- Wallet-specific effect adapter. Bank identity/journal/UI/transport never
-- interpret engine components; other bank categories can provide their own.
local players=dofile('mods/noita_together2/files/g3/game_api.lua')
local M={}
function M.new(e,validate,subject,outlet)
 local api={}
 function api.snapshot()
  local player,playing=subject()
  if not playing or not player then return nil end
  local resolved,state=players.resolve_player(e,player)
  if resolved~=player or not state or not state.ready or state.transformed then return nil end
  for _,c in ipairs(e.EntityGetComponentIncludingDisabled(player,'VariableStorageComponent')or {})do if e.ComponentGetValue2(c,'name')=='nt2_g3_recovery'then return nil end end
  local wallet=e.EntityGetFirstComponentIncludingDisabled(player,'WalletComponent');if not wallet or wallet==0 then return nil end
  local balance=e.ComponentGetValue2(wallet,'money');if not validate.integer(balance,0,2147483647)then return nil end
  return {player=player,wallet=wallet,balance=balance}
 end
 function api.rebind_snapshot(before,now)local live=api.snapshot();return live and live.player==now.player and live.wallet==now.wallet or false end
 function api.commit(before,after)
  local now=api.snapshot();if not now or now.player~=before.player or now.wallet~=before.wallet or now.balance~=before.balance then return false end
  if not outlet()then return false end
  local value=e.ComponentSetValue2(now.wallet,'money',after);if value==false then return false end
  local check=api.snapshot();return check and check.player==now.player and check.wallet==now.wallet and check.balance==after or false
 end
 return api
end
return M
