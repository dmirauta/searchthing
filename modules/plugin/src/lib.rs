//! Allows loading of external plugins from shared objects.

use libloading::{Library, Symbol};
use searchthing_interface::{SearchItemHandle, SearchModule};
use std::{
    ffi::{c_char, CStr},
    path::PathBuf,
};

type InitFn = unsafe extern "C" fn() -> *const c_char;
// will have to be a zero terminated array
type QueeryFn = unsafe extern "C" fn(*const u8, u32) -> *const SearchItemHandle;
// name, desc, icon_name
type HandleToStrFn = unsafe extern "C" fn(SearchItemHandle) -> *const c_char;
type HandleSelectionFn = unsafe extern "C" fn(SearchItemHandle);

struct PlugMethods<'a> {
    queery: Symbol<'a, QueeryFn>,
    name: Symbol<'a, HandleToStrFn>,
    desc: Symbol<'a, HandleToStrFn>,
    icon_name: Symbol<'a, HandleToStrFn>,
    handle_selection: Symbol<'a, HandleSelectionFn>,
    plug_name: &'a str,
    plug_icon: &'a str,
}

impl<'a> PlugMethods<'a> {
    unsafe fn get(lib: &'a Library) -> Result<Self, libloading::Error> {
        let init: Symbol<InitFn> = lib.get(b"init")?;
        let queery = lib.get(b"queery")?;
        let name = lib.get(b"name")?;
        let desc = lib.get(b"desc")?;
        let icon_name = lib.get(b"icon_name")?;
        let handle_selection = lib.get(b"handle_selection")?;

        let info = CStr::from_ptr(init()).to_str().unwrap_or("<get info err>,");
        let mut sp = info.split(",");
        let plug_name = sp.next().unwrap();
        let plug_icon = sp.next().unwrap();

        Ok(Self {
            plug_name,
            plug_icon,
            queery,
            name,
            desc,
            icon_name,
            handle_selection,
        })
    }
}

/// A wrapper module for external plugins.
pub struct PluginModule {
    plug_name: String,
    plug_icon: String,
    lib: Library,
}

impl PluginModule {
    pub unsafe fn new(lib_path: &PathBuf) -> Result<Self, libloading::Error> {
        let lib = Library::new(lib_path)?;
        // NOTE: Gets info, but also checks that all the expected symbols can be found
        let methods = PlugMethods::get(&lib)?;
        Ok(Self {
            plug_name: methods.plug_name.into(),
            plug_icon: methods.plug_icon.into(),
            lib,
        })
    }
}

impl SearchModule for PluginModule {
    fn mod_info(&self) -> searchthing_interface::SearcherInfo {
        searchthing_interface::SearcherInfo {
            name: &self.plug_name,
            icon: &self.plug_icon,
        }
    }

    fn queery(
        &self,
        input: &str,
        max_returned: u32,
    ) -> Vec<searchthing_interface::SearchItemHandle> {
        let mut res = vec![];
        unsafe {
            let methods = PlugMethods::get(&self.lib).unwrap();
            // TODO: Should convert to CStr?
            let mut res_ptr = (methods.queery)(input.as_ptr(), max_returned);
            while *res_ptr != SearchItemHandle::TERMINATOR {
                res.push(*res_ptr);
                res_ptr = res_ptr.wrapping_add(1);
            }
        }
        res
    }

    fn match_info(
        &self,
        item: searchthing_interface::SearchItemHandle,
    ) -> searchthing_interface::MatchInfo {
        unsafe {
            let methods = PlugMethods::get(&self.lib).unwrap();
            let name = CStr::from_ptr((methods.name)(item)).to_str().unwrap();
            let desc = CStr::from_ptr((methods.desc)(item)).to_str().unwrap();
            let icon = CStr::from_ptr((methods.icon_name)(item)).to_str().unwrap();
            searchthing_interface::MatchInfo { name, desc, icon }
        }
    }

    fn handle_selection(&self, selection: searchthing_interface::SearchItemHandle) {
        unsafe {
            let methods = PlugMethods::get(&self.lib).unwrap();
            (methods.handle_selection)(selection);
        }
    }
}
