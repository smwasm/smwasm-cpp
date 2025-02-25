use json::JsonValue;
use lazy_static::lazy_static;
use std::sync::RwLock;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::collections::HashMap;

use smcore::{smh, smu};
use smdton::{SmDton, SmDtonBuffer, SmDtonBuilder, SmDtonMap};


type SmCallFunction = extern "C" fn(*const c_char) -> *mut c_char;

lazy_static! {
    static ref LIB_DATA: RwLock<LibData> = RwLock::new(LibData { _sn: 0 });
    static ref CALLBACK: RwLock<HashMap<String, SmCallFunction>> = RwLock::new(HashMap::new());
}

struct LibData {
    _sn: u64,
}

fn inc_sn() -> u64 {
    let mut need_init = false;
    let sn: u64;
    {
        let mut hb = LIB_DATA.write().unwrap();
        if hb._sn == 0 {
            need_init = true;
        }
        hb._sn += 1;
        sn = hb._sn;
    }

    if need_init {
        smloadwasm::init();
        smsys::init();
    }

    return sn;
}

fn get_item(jsn: &JsonValue, item: &str) -> Option<String> {
    let value = jsn[item].as_str()?;
    Some(value.to_string())
}

#[no_mangle]
pub extern "C" fn sm_register(_usage: *const c_char, _callback: SmCallFunction) {
    inc_sn();

    let c_str = unsafe { CStr::from_ptr(_usage) };
    match c_str.to_str() {
        Ok(txt) => {
            let jsn = json::parse(txt);
            match jsn {
                Ok(parsed_json) => {
                    let op_name = get_item(&parsed_json, "$usage");

                    if let Some(name) = op_name {
                        {
                            let mut cbmap = CALLBACK.write().unwrap();
                            cbmap.insert(name.clone(), _callback);
                        }
        
                        smh.register_by_json(&parsed_json, _call_sm);
                    } else {
                    }
                },
                Err(_e) => {
                }
            }
        },
        Err(_) => {
        }
    }
}

#[no_mangle]
pub extern "C" fn sm_load(_wasm: *const c_char, _space: c_int) {
    inc_sn();

    let c_str = unsafe { CStr::from_ptr(_wasm) };
    match c_str.to_str() {
        Ok(itxt) => {
            smloadwasm::load_wasm(itxt, _space);
        },
        Err(_) => {
        }
    }
}

#[no_mangle]
pub extern "C" fn sm_sn() -> c_int {
    let ret = inc_sn() as i32;
    return ret;
}

#[no_mangle]
pub extern "C" fn sm_call(_intxt: *const c_char) -> *mut c_char {
    let mut otxt = "{}".to_string();

    let c_str = unsafe { CStr::from_ptr(_intxt) };
    match c_str.to_str() {
        Ok(itxt) => {
            let ret = call_native(itxt);
            if ret.len() > 0 {
                otxt = ret;
            } else {
                let jsn = json::parse(&itxt).unwrap();
                let smb = smu.build_buffer(&jsn);
                let ret = smh.call(smb);
            
                let op_ret = ret.stringify();
                match op_ret {
                    Some(txt) => {
                        otxt = txt;
                    },
                    None => {
                    },
                }
            }
        },
        Err(_) => {
        }
    }

    let result = CString::new(otxt).unwrap();
    return result.into_raw();
}

fn call_native(itxt: &str) -> String {
    let mut otxt = "".to_string();

    let jsn = json::parse(itxt);
    match jsn {
        Ok(parsed_json) => {
            let name = get_item(&parsed_json, "$usage");
            if name.is_some() {
                let mut fnn : Option<SmCallFunction> = None;
                {
                    let cb2 = CALLBACK.read().unwrap();
                    if let Some(callback) = cb2.get(&name.unwrap()) {
                        fnn = Some(*callback);
                    } else {
                    }
                }
                
                if fnn != None {
                    let _fn = fnn.unwrap();
                    otxt = do_call_native(itxt, _fn);
                } else {
                }
            }
        },
        Err(_e) => {
        }
    }

    return otxt;
}

fn do_call_native(_intxt: &str, _fn: SmCallFunction) -> String {
    let c_string = CString::new(_intxt).unwrap();
    let _inptr = c_string.as_ptr();
    let c_output = _fn(_inptr);

    let outtxt = unsafe {
        CStr::from_ptr(c_output)
            .to_str()
            .expect("Failed to convert CStr to &str")
            .to_string()
    };

    unsafe {
        libc::free(c_output as *mut libc::c_void);
    }

    return outtxt;
}

fn _call_sm(_input: &SmDtonBuffer) -> SmDtonBuffer {
    let sd = SmDton::new_from_buffer(_input);
    let intxt = sd.stringify().unwrap();

    let mut result_str = "{}".to_string();

    let ret = call_native(&intxt);
    if ret.len() > 0 {
        result_str = ret;
    }

    let parsed: Result<JsonValue, json::Error> = json::parse(&result_str);
    match parsed {
        Ok(jsn) => {
            let mut sdb = SmDtonBuilder::new_from_json(&jsn);
            return sdb.build();
        }
        Err(_e) => {
        }
    }

    let mut _map = SmDtonMap::new();
    return _map.build();
}
