-- Repair only the owned emitter pair, including partially loaded entities.
local id=GetUpdatedEntityID()
local emitters=EntityGetComponentIncludingDisabled(id,'ParticleEmitterComponent')or{}
if not emitters[1]or not emitters[2]then return end
if ComponentGetValue2(emitters[1],'m_particle_attractor_id')<0 then
 local other=ComponentGetValue2(emitters[2],'m_particle_attractor_id')
 if other and other>=0 then ComponentSetValue2(emitters[1],'m_particle_attractor_id',other)end
end
