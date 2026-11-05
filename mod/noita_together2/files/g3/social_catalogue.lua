-- Fixed legacy catalogue. Wire IDs select only these local assets.
local M={ids={'wave','heart','thumbs_up','disapprove','come_on','t_pose','mario','luigi','clap','macarena','the_wave','aggressive_point','default_dance','praise_the_sun','dab','worm','meditate','repose','egyptian','salute'},robes={'red','orange','yellow','green','blue','purple'},entries={}}
local names={'Wave','Heart','Thumbs Up','Disapprove','Beckon','T-Pose','Mario Dance','Luigi Dance','Clap','Macarena','The Wave','Aggressive Point','Default Dance','Praise The Sun','Dab','Worm','Meditate','Repose','Egyptian','Salute'}
local frames={heart=101,thumbs_up=240,disapprove=140,praise_the_sun=375,worm=216}
local icons={come_on='beckon',the_wave='thewave'}
local effects={heart='heart_entity',thumbs_up='thumbs_up_entity',aggressive_point='aggressive_point_entity',praise_the_sun='praise_the_sun_entity'}
M.root='mods/noita_together2/files/g3/emotes/'
for i,id in ipairs(M.ids)do M.entries[id]={id=id,name=names[i],frames=frames[id]or 0,icon=M.root..'icons/'..(icons[id]or id)..'.png',cape=id~='default_dance',effect=effects[id]and M.root..'effects/'..effects[id]..'.xml'}end
function M.robe(id)for _,v in ipairs(M.robes)do if v==id then return true end end;return false end
return M
