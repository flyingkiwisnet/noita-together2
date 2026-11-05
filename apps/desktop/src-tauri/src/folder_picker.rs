//! User-initiated folder selection only; does not install or open a game.
pub fn choose()->Result<Option<String>,String>{choose_title("Choose the Noita folder containing noita.exe")}
#[cfg(windows)]pub fn choose_title(prompt:&str)->Result<Option<String>,String>{
 use std::ffi::c_void;
 #[repr(C)]struct Browse{owner:*mut c_void,root:*const c_void,display:*mut u16,title:*const u16,flags:u32,callback:*const c_void,param:isize,image:i32}
 #[link(name="shell32")]unsafe extern "system"{fn SHBrowseForFolderW(info:*const Browse)->*mut c_void;fn SHGetPathFromIDListW(id:*const c_void,path:*mut u16)->i32;}
 #[link(name="ole32")]unsafe extern "system"{fn CoInitializeEx(reserved:*mut c_void,flags:u32)->i32;fn CoUninitialize();fn CoTaskMemFree(value:*mut c_void);}
 unsafe{
  if CoInitializeEx(std::ptr::null_mut(),2)<0{return Err("Folder picker unavailable".into())}
  let mut display=[0u16;260];let title:Vec<u16>=prompt.encode_utf16().chain(Some(0)).collect();
  let info=Browse{owner:std::ptr::null_mut(),root:std::ptr::null(),display:display.as_mut_ptr(),title:title.as_ptr(),flags:0x51,callback:std::ptr::null(),param:0,image:0};
  let id=SHBrowseForFolderW(&info);let result=if id.is_null(){Ok(None)}else{let mut path=[0u16;260];let success=SHGetPathFromIDListW(id,path.as_mut_ptr())!=0;CoTaskMemFree(id);if success{let n=path.iter().position(|c|*c==0).unwrap_or(path.len());String::from_utf16(&path[..n]).map(Some).map_err(|_|"Folder path unavailable".into())}else{Err("Choose a local filesystem folder".into())}};CoUninitialize();result
 }
}
#[cfg(not(windows))]pub fn choose_title(_prompt:&str)->Result<Option<String>,String>{Err("Folder picker unavailable on this platform".into())}
