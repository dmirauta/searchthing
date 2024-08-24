use core::cell::RefCell;
use std::ffi::CString;

use searchthing_interface::SearchItemHandle;

thread_local! {
    static ENTRIES: RefCell<Vec<CString>> = Default::default();
    static MATCH_RES: RefCell<Vec<SearchItemHandle>> = Default::default();
    static PLUG_INFO: CString = CString::new("Rust plugin (external),text-x-rust").unwrap();
}

#[no_mangle]
pub extern "C" fn init() -> *const i8 {
    ENTRIES.with_borrow_mut(|e| {
        *e = vec![
            CString::new("apple").unwrap(),
            CString::new("banana").unwrap(),
            CString::new("coconut").unwrap(),
        ];
    });
    PLUG_INFO.with(|p| p.as_ptr())
}

#[no_mangle]
pub extern "C" fn queery(_mh: SearchItemHandle) -> *const SearchItemHandle {
    MATCH_RES.with_borrow_mut(|r| {
        *r = vec![
            SearchItemHandle(0),
            SearchItemHandle(1),
            SearchItemHandle(2),
            SearchItemHandle::TERMINATOR,
        ];
        r.as_ptr()
    })
}

#[no_mangle]
pub extern "C" fn name(mh: SearchItemHandle) -> *const i8 {
    ENTRIES.with_borrow_mut(|e| e[mh.0 as usize].as_ptr())
}

#[no_mangle]
pub extern "C" fn desc(mh: SearchItemHandle) -> *const i8 {
    ENTRIES.with_borrow_mut(|e| e[mh.0 as usize].as_ptr())
}

#[no_mangle]
pub extern "C" fn icon_name(mh: SearchItemHandle) -> *const i8 {
    ENTRIES.with_borrow_mut(|e| e[mh.0 as usize].as_ptr())
}

#[no_mangle]
pub extern "C" fn handle_selection(mh: SearchItemHandle) {
    ENTRIES.with_borrow_mut(|e| {
        println!("External rust handling: {:?}", e[mh.0 as usize]);
    });
}
