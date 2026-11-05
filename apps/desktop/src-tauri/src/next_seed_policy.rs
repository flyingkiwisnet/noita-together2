//! Consumer of Server's g3-seed-loot-1 canonical configuration. No RNG resolution here.
use super::*;
pub(super) const POLICY_CAP:&str="core.seed-policy.v1";
pub(super) const LOOT_CAP:&str="core.randomize-loot.v1";
#[derive(Debug,Clone,PartialEq,Eq)]
pub(super) struct Policy {pub seed:u32,pub bits:u32,pub name:String,pub loot:bool,pub explicit:bool,pub testing_mode:bool}
pub(super) fn testing_mode(config:&wire::Config)->Result<bool,String>{
 let mut result=None;for setting in config.settings.as_deref().unwrap_or(&[]){if setting.key.as_deref()==Some("NT_testing_mode"){if result.is_some(){return Err("Duplicate Testing mode".into())}result=Some(match setting.value.as_deref(){Some("true")=>true,Some("false")=>false,_=>return Err("Invalid Testing mode".into())});}}Ok(result.unwrap_or(false))
}
pub(super) fn resolved(config:&wire::Config)->Result<Policy,String>{
    let testing_mode=testing_mode(config)?;
    let seed=config.seed.unwrap_or(0); // canonical protobuf uint32 default
    let Some(name)=config.seed_policy.as_deref() else {
        // Preserve old tested configs including0. Never widen omitted semantics.
        return Ok(Policy{seed,bits:1,name:"shared-fixed".into(),loot:false,explicit:false,testing_mode});
    };
    let loot=config.randomize_loot.unwrap_or(false); // canonical protobuf omitted false
    let bits=match name {
        "shared-fixed"|"shared-random" if seed>0=>if loot{5}else{1},
        "individual-random" if seed==0&&!loot=>2,
        _=>return Err("Unresolved or invalid canonical seed/loot policy; review lobby again".into()),
    };
    let required=config.required_capabilities.as_deref().unwrap_or(&[]);
    if !required.iter().any(|c|c==POLICY_CAP)||(loot&&!required.iter().any(|c|c==LOOT_CAP)){
        return Err("Seed/loot policy lacks its canonical required capability".into());
    }
    Ok(Policy{seed,bits,name:name.into(),loot,explicit:true,testing_mode})
}
impl<S:CredentialStore> Core<S> {
    pub(super) fn local_game_capabilities(&self)->Vec<String>{
        let mut caps=if self.social_requested&&self.social_enabled{vec!["core.seed".into(),"core.ghost".into(),"core.hud".into(),"core.poi".into()]}
            else if self.g3.is_some(){vec!["core.seed".into(),"core.ghost".into()]}else{vec!["core.seed".into()]};
        // A service advertisement cannot approve another installed entry/native closure.
        // Seed/loot and A3 require the compiled matching profile plus its verified current root.
        if self.social_enabled{
            if let Some(metadata)=&self.social_profile {
                let approved=nt2_desktop_g0::g3_social_installation::development_profile().ok();
                if self.game_path.as_ref().is_some_and(|root|self.social_verified.as_ref().is_some_and(|v|v.matches(root,metadata)))
                    &&approved.as_ref().is_some_and(|a|a["profile"]==*metadata) {
                    for cap in [POLICY_CAP,LOOT_CAP]{if metadata["capabilities"].as_array().is_some_and(|a|a.iter().any(|c|c==cap)){caps.push(cap.into());}}
                    if self.social_requested&&metadata["capabilities"].as_array().is_some_and(|a|["core.emotes","core.cosmetics"].iter().all(|cap|a.iter().any(|c|c==cap))){
                        caps.extend(["core.emotes".into(),"core.cosmetics".into()]);
                    }
                    if self.social_requested&&metadata["capabilities"].as_array().is_some_and(|a|a.iter().any(|c|c=="core.gold")){caps.push("core.gold".into());}
                    if self.social_requested&&metadata["capabilities"].as_array().is_some_and(|a|a.iter().any(|c|c=="core.items")){caps.push("core.items".into());}
                    if self.social_requested&&metadata["capabilities"].as_array().is_some_and(|a|["core.world-rules","core.hourglass"].iter().all(|cap|a.iter().any(|c|c==cap))){caps.extend(["core.world-rules".into(),"core.hourglass".into()]);}
                    if self.social_requested&&metadata["capabilities"].as_array().is_some_and(|a|a.iter().any(|c|c=="race.goals.v1")){caps.push("race.goals.v1".into());}
                }
            }
        }
        caps
    }
    pub(super) fn supported_policy(&self,config:&wire::Config)->Result<Policy,String>{
        let policy=resolved(config)?;
        if policy.testing_mode {
            let approved=nt2_desktop_g0::g3_social_installation::development_profile().ok();
            let installed=self.game_path.as_ref().zip(self.social_profile.as_ref()).is_some_and(|(root,metadata)|self.social_verified.as_ref().is_some_and(|v|v.matches(root,metadata))&&approved.as_ref().is_some_and(|a|a["profile"]==*metadata&&["mods/noita_together2/files/startup/testing_loadout.lua","mods/noita_together2/files/startup/testing-starting-loadout.json"].iter().all(|path|a["requiredLocalFiles"].as_array().is_some_and(|files|files.iter().any(|f|f["path"]==*path)))));
            if !installed{return Err("Testing mode requires its reviewed matching installed runtime".into())}
        }
        if policy.explicit {
            let local=self.local_game_capabilities();let service=self.info.as_ref().and_then(|i|i.capabilities.as_ref());
            for cap in [POLICY_CAP].into_iter().chain(policy.loot.then_some(LOOT_CAP)) {
                if !local.iter().any(|c|c==cap)||service.is_none_or(|caps|!caps.iter().any(|c|c==cap)){
                    return Err("Selected seed/loot policy requires the reviewed matching runtime; U2 composition is pending".into());
                }
            }
        }
        Ok(policy)
    }
}
#[cfg(test)]mod tests{
    use super::*;
    #[cfg(feature="g3-rejoin")]
    #[test]fn a3_handshake_capabilities_require_exact_compiled_and_verified_current_runtime(){
        let approval=nt2_desktop_g0::g3_social_installation::development_profile().unwrap();let profile=approval["profile"].clone();if !profile["capabilities"].as_array().unwrap().iter().any(|c|c=="core.emotes"){return}
        let repo=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).unwrap().to_path_buf();let owned=repo.join("spikes/noita-entry/.build").join(format!("a3-capabilities-{}",uuid::Uuid::new_v4()));let game=owned.join("game");std::fs::create_dir_all(&game).unwrap();
        for mapping in approval["runtimeMappings"].as_array().unwrap(){let target=game.join(mapping["path"].as_str().unwrap());std::fs::create_dir_all(target.parent().unwrap()).unwrap();std::fs::copy(repo.join(mapping["sourcePath"].as_str().unwrap()),target).unwrap();}
        for name in ["noita.exe","lua51.dll"]{std::fs::copy(std::path::Path::new("C:/Noita").join(name),game.join(name)).unwrap();}
        let mut c=Core::new(super::super::tests::MemoryStore::default(),uuid::Uuid::new_v4(),Arc::new(Mutex::new(None)),owned.join("claims"));c.enable_g3(&owned.join("journal.jsonl"),Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()))).unwrap();let stream=Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default()));c.request_social(stream.clone()).unwrap();c.game_path=Some(game.clone());c.social_profile=Some(profile.clone());
        c.info=Some(wire::Info{capabilities:Some(vec!["core.emotes".into(),"core.cosmetics".into(),"service-only".into()]),..Default::default()});
        let absent=|c:&Core<super::super::tests::MemoryStore>|assert!(!c.local_game_capabilities().iter().any(|v|v=="core.emotes"||v=="core.cosmetics"||v=="core.gold"));absent(&c);
        c.enable_social(&game,&profile,stream.clone()).unwrap();let expected:Vec<String>=profile["capabilities"].as_array().unwrap().iter().map(|c|c.as_str().unwrap().into()).collect();assert_eq!(c.local_game_capabilities(),expected);assert!(!c.local_game_capabilities().iter().any(|v|v=="service-only"));
        c.social_enabled=false;absent(&c);c.social_enabled=true;c.social_requested=false;absent(&c);c.social_requested=true;
        c.game_path=Some(owned.join("different-game"));absent(&c);c.game_path=Some(game.clone());
        let mut changed=profile.clone();changed["id"]=json!("service-substitution");c.social_profile=Some(changed);absent(&c);c.social_profile=Some(profile.clone());
        let mut uncompiled=profile.clone();uncompiled["capabilities"].as_array_mut().unwrap().push(json!("service-only"));c.social_profile=Some(uncompiled.clone());absent(&c);assert!(c.enable_social(&game,&uncompiled,stream).is_err());
        c.social_profile=Some(profile);c.social_verified=None;absent(&c);drop(c);assert!(owned.canonicalize().unwrap().starts_with(repo.join("spikes/noita-entry/.build").canonicalize().unwrap()));std::fs::remove_dir_all(owned).unwrap();
    }
    #[test]fn testing_loadout_frozen_config_reaches_actual_startup_snapshot(){
        use nt2_desktop_g0::startup_local::{Prepared,Scope};
        let mut config=wire::Config{seed:Some(424242),..Default::default()};assert!(!resolved(&config).unwrap().testing_mode);
        for enabled in [false,true]{
            config.settings=Some(vec![wire::Setting{key:Some("NT_testing_mode".into()),value:Some(enabled.to_string())}]);assert_eq!(resolved(&config).unwrap().testing_mode,enabled);
            let value=serde_json::to_value(&config).unwrap();let config_digest=nt2_desktop_g0::protocol_connection::canonical_fingerprint(&serde_json::to_vec(&value).unwrap()).unwrap();
            let scope=Scope{origin:"https://127.0.0.1:44399".into(),service:"test_service".into(),service_epoch:"epoch".into(),account:"account".into(),device:uuid::Uuid::new_v4(),desktop:"desktop".into(),desktop_epoch:"desktop_epoch".into(),room:"room".into(),run:"run".into(),run_epoch:"run_epoch".into(),config:1,config_digest,preparation:"preparation".into()};
            let bundle=Prepared::lifecycle(scope,uuid::Uuid::new_v4(),424242,1,1,std::time::Instant::now()+Duration::from_secs(10),value.clone()).unwrap();
            let snapshot=bundle.snapshot().unwrap();let decoded:Value=serde_json::from_slice(&snapshot).unwrap();assert_eq!(decoded["config"],value);assert_eq!(decoded["config"]["settings"][0]["value"],enabled.to_string());
            if let Ok(output)=std::env::var("NT2_TESTING_LOADOUT_CONFIG_OUTPUT"){
                let repo=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).unwrap();let output=std::path::PathBuf::from(output);assert!(output.is_absolute()&&output.canonicalize().unwrap().starts_with(repo.join("spikes/noita-entry/.build").canonicalize().unwrap()));
                let path=output.join(if enabled{"enabled-snapshot.json"}else{"disabled-snapshot.json"});assert!(!path.exists());std::fs::write(path,snapshot).unwrap();
            }
        }
        let (c,_,_)=super::super::tests::release_case();let unsupported=wire::Config{settings:Some(vec![wire::Setting{key:Some("NT_testing_mode".into()),value:Some("true".into())}]),..Default::default()};assert!(c.supported_policy(&unsupported).unwrap_err().contains("Testing mode requires"));
        for setting in [vec![wire::Setting{key:Some("NT_testing_mode".into()),value:Some("yes".into())}],vec![wire::Setting{key:Some("NT_testing_mode".into()),value:None}],vec![wire::Setting{key:Some("NT_testing_mode".into()),value:Some("true".into())};2]]{config.settings=Some(setting);assert!(resolved(&config).is_err());}
    }
    #[test]fn seedloot01_release_binds_individual_actual_seed_policy_and_loot(){
        let (mut c,mut p,mut r)=super::super::tests::release_case();
        let config=c.room.as_mut().unwrap().config.as_mut().unwrap();config.seed=Some(0);config.seed_policy=Some("individual-random".into());config.randomize_loot=Some(false);config.required_capabilities=Some(vec![POLICY_CAP.into()]);
        p.seed=0;p.world_policy=2;p.actual_seed=Some(424242);
        r.actual_seed=Some("424242".into());r.seed_policy=Some("individual-random".into());r.randomize_loot=Some(false);
        assert!(c.validate_release(&r,&p).is_ok());
        r.actual_seed=Some("888".into());assert!(c.validate_release(&r,&p).is_err());r.actual_seed=Some("424242".into());
        r.randomize_loot=Some(true);assert!(c.validate_release(&r,&p).is_err());r.randomize_loot=Some(false);
        r.seed_policy=Some("shared-fixed".into());assert!(c.validate_release(&r,&p).is_err());
        let config=c.room.as_ref().unwrap().config.as_ref().unwrap();assert!(c.supported_policy(config).unwrap_err().contains("matching runtime"));
    }
    #[test]fn seedloot01_resolved_policy_preserves_legacy_and_rejects_unresolved_or_missing_capability(){
        for seed in [0,424242]{let c=wire::Config{seed:Some(seed),randomize_loot:Some(true),..Default::default()};assert_eq!(resolved(&c).unwrap().bits,1);assert!(!resolved(&c).unwrap().loot);}
        for (name,seed,loot,bits) in [("shared-fixed",424242,false,1),("shared-random",123456,true,5),("individual-random",0,false,2)]{
            let mut c=wire::Config{seed:Some(seed),seed_policy:Some(name.into()),randomize_loot:Some(loot),required_capabilities:Some(vec![POLICY_CAP.into(),LOOT_CAP.into()]),..Default::default()};assert_eq!(resolved(&c).unwrap().bits,bits);
            c.required_capabilities=Some(vec![]);assert!(resolved(&c).is_err());
        }
        let sparse=wire::Config{seed_policy:Some("individual-random".into()),required_capabilities:Some(vec![POLICY_CAP.into()]),..Default::default()};assert_eq!(resolved(&sparse).unwrap().bits,2);
        for (name,seed,loot) in [("shared-random",0,true),("shared-fixed",0,false),("individual-random",1,false),("individual-random",0,true),("foreign",0,false)]{
            let c=wire::Config{seed:Some(seed),seed_policy:Some(name.into()),randomize_loot:Some(loot),..Default::default()};assert!(resolved(&c).is_err());
        }
    }
}
