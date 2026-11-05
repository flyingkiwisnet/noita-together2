-- Production entry/native-owner diagnostic opt-out over the existing focused
-- current-player fixture. Required protection/GUI behaviour is still exercised.
ModSettingGet=function(k)if k=='noita_together2.NT_ENABLE_LOGGER'then return false end end
dofile(assert(os.getenv('NT2_A3_REPO'))..'/apps/desktop/tools/a2-polymorph-entry-test.lua')
