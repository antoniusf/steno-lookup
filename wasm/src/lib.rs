extern crate wee_alloc;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[no_mangle]
pub unsafe extern fn load_json(offset: u32, length: u32) -> u32{
    let string_data = std::slice::from_raw_parts(
        offset as *const u8,
        length as usize
    );
    let mut counter = 0;
    for byte in string_data {
        counter += *byte as u32;
    }
    return counter;
}
