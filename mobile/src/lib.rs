mod app;
mod data;
mod demo;
mod ui;

use app::App;
use serde_json::{json, Value};
use std::cell::RefCell;

thread_local! {
    static STATE: RefCell<Option<App>> = const { RefCell::new(None) };
    static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

// A single synchronous host dispatch owns its input until it is consumed here.
// Keeping the output buffer in Rust avoids handing a Rust String allocation to JS.
#[no_mangle]
pub extern "C" fn input_buffer(len: usize) -> *mut u8 {
    Box::into_raw(vec![0_u8; len].into_boxed_slice()) as *mut u8
}
/// # Safety
/// Pass the pointer and exact length returned by input_buffer once.
#[no_mangle]
pub unsafe extern "C" fn dispatch(ptr: *mut u8, len: usize) -> usize {
    let bytes = Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len));
    let event: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    let out=STATE.with_borrow_mut(|slot|{
        let app=slot.get_or_insert_with(||App::new(data::parse_date(event["today"].as_str().unwrap_or("")).expect("local ISO date required at startup")));
        if let Some(key)=event["result"].as_str(){
            app.result(key,event["data"].clone(),event["error"].as_str());
        }else if let Some(action)=event["action"].as_str(){app.action(action,&event["fields"])}
        json!({"html":app.html(),"effects":std::mem::take(&mut app.effects),"signed":app.signed(),"demo":app.demo})
    });
    OUTPUT.with_borrow_mut(|buf| {
        *buf = out.to_string().into_bytes();
        buf.len()
    })
}
#[no_mangle]
pub extern "C" fn output_buffer() -> *const u8 {
    OUTPUT.with_borrow(|b| b.as_ptr())
}
