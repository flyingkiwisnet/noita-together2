-- Fixture-only engine view for optional Sampo capture. Global gameplay and
-- protection inventory APIs stay intact while the optional observation is absent.
local M={}
function M.capture_engine(engine,unavailable)
 return setmetatable({EntityGetAllChildren=function(...)
  if unavailable()then return nil end
  return engine.EntityGetAllChildren(...)
 end},{__index=engine})
end
return M
