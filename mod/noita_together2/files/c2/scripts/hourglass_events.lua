local store=dofile('mods/noita_together2/files/c2/storage.lua')
local api=dofile('mods/noita_together2/files/c2/hook_api.lua')
local registry=NT2_C2_HOURGLASS_REGISTRY or dofile('mods/noita_together2/files/c2/hourglass.lua').new(_G)
NT2_C2_HOURGLASS_REGISTRY=registry
hourglassEventHandlers=registry.entries
function RegisterHourglassEvent(effect,material,handler,force)return registry:register(effect,material,handler,force)end
function CreateHourglassEntity(x,y,effect,material,anchor)
 local entry=registry.entries[effect];if not entry or type(entry.handler)~='function'or entry.material~=material then return nil end
 local id=EntityLoad('mods/noita_together2/files/c2/entities/hourglass_effect_trigger.xml',x,y)
 local checker=EntityGetFirstComponentIncludingDisabled(id,'MaterialAreaCheckerComponent')
 if not checker then return nil end
 local mat=CellFactory_GetType(material);ComponentSetValue2(checker,'material',mat);ComponentSetValue2(checker,'material2',mat)
 store.mark(_G,id,'nt2_c2_hourglass',{anchor=anchor,effectId=effect,materialId=material})
 EntitySetComponentIsEnabled(id,checker,false);return id
end
function material_area_checker_success()
 local id=GetUpdatedEntityID();local group=store.component(_G,id,'nt2_c2_hourglass');if not group then return end
 local anchor=group.anchor;if not anchor or not EntityGetIsAlive(anchor)then return end
 local bound=store.component(_G,anchor,'nt2_c2_hourglass_group');local a=api.current()
 if not a or not bound or bound.world~=a.world or bound.context~=a.context or bound.progression~=a.progression or not bound.activationId or bound.claimed then return end
 local entry=registry.entries[group.effectId];if not entry or type(entry.handler)~='function'or entry.material~=group.materialId or not a.hourglass or a.hourglass[group.effectId]~=group.materialId then GamePrint('NT2: unsupported hourglass registration.');return end
 -- A pre-bound ID persists one immutable choice before feedback. Capacity
 -- failure leaves the checker enabled to retry without losing the activation.
 local original=api.capture({hourglass={effectId=group.effectId,materialId=group.materialId,catalogueRevision='nt2-hourglass-builtins-1'}},bound.activationId,id)
 if not original then return end
 bound.claimed=true;store.mark(_G,anchor,'nt2_c2_hourglass_group',bound)
 EntitySetComponentsWithTagEnabled(id,'enabled_by_liquid',true);EntitySetComponentsWithTagEnabled(id,'disabled_by_liquid',false)
 local x,y=EntityGetTransform(id)
 for _,other in ipairs(EntityGetInRadiusWithTag(x,y,150,'hourglass_entity')or{})do local row=store.component(_G,other,'nt2_c2_hourglass');if other~=id and row and row.anchor==anchor then EntityKill(other)end end
 GamePrintImportant(dofile('mods/noita_together2/files/c2/hourglass.lua').text(_G,'$noitatogether_hourglass_gave_boon'),'');api.submit(original)
end
