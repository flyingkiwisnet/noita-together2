local repo=assert(os.getenv('NT2_C3_REPO'))
local helper=dofile(repo..'/apps/desktop/tools/lifecycle-lc1-loot-harness.lua')
-- Use repository production files directly; no native or service success is injected.
helper.regressions(repo)
