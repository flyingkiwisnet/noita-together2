-- Engine-test harness: actual script text in a distinct Noita-like Lua state.
-- No native/module access or main-context NT2_C1 global is inherited.
return function(parent,read)
 local absent={require=true,ffi=true,package=true,io=true,os=true,debug=true,loadfile=true,NT2_C1=true,NT2_C2=true,NT2_C2_HOURGLASS_REGISTRY=true}
 local e=setmetatable({},{__index=function(_,key)if not absent[key]then return parent[key]end end});e._G=e
 function e.dofile(path)local f=assert(loadstring(read(path),'@'..path));setfenv(f,e);return f()end
 e.dofile_once=e.dofile
 return e
end
