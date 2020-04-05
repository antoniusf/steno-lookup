#![no_std]

#[no_mangle]
pub unsafe extern fn load_json(offset: u32, length: u32) -> u32{
    let string_data = core::slice::from_raw_parts(
        offset as *const u8,
        length as usize
    );
    let mut counter = 0;
    for byte in string_data {
        counter += *byte as u32;
    }
    panic!("asdf");
    return counter;
}

#[link(wasm_import_module = "env")]
extern { fn logErr(offset: u32, length: u32); }

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    if let Some(string) = info.payload().downcast_ref::<&str>() {
        unsafe {
            logErr(string.as_ptr() as u32, string.len() as u32);
        }
    }
    else {
        let string = "Panic occured, but we didn't get a usable payload.";
        unsafe {
            logErr(string.as_ptr() as u32, string.len() as u32);
        }
    }
    unsafe {
        core::arch::wasm32::unreachable();
    }
}
