// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![no_std]

use core::mem::size_of;
use core::fmt::Write;
use core::convert::TryInto;
use core::borrow::Borrow;
use query_engine::{self, InternalError};

#[link(wasm_import_module = "env")]
extern { fn logErr(message_offset: u32, message_length: u32, details_offset: u32, details_length: u32, line: u32); }

fn log_err_internal(error: InternalError) {
    unsafe {
        logErr(error.message.as_ptr() as u32, error.message.len() as u32,
               error.details.as_ptr() as u32, error.details.len() as u32,
               error.line);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {

    // this is for debugging only
    // (the formatting code adds about 10kB to the wasm executable,
    // and i don't think it's useful in production.)
    //
    // let mut message = [0u8; 200];
    // let mut buffer = WriteBuffer { buffer: &mut message[..], position: 0 };
    // write!(buffer, "{}", info);
    // let end = message.iter().position(|&byte| byte == 0).unwrap_or(message.len());
    // 
    // log_err_internal(InternalError {
    //     message: &message[..end],
    //     details: b"".as_ref(),
    //     line: info.location().map_or(0, |loc| loc.line())
    // });
    unsafe {
        core::arch::wasm32::unreachable();
    }
}

fn handle_loader_error(error: InternalError) -> usize {
    log_err_internal(error);
    panic!();
}

#[no_mangle]
pub unsafe extern fn load_json(offset: u32, length: u32) -> u32 {
    let buffer = core::slice::from_raw_parts_mut(
        offset as *mut u8,
        length as usize
    );
    return query_engine::load_json_internal(buffer).unwrap_or_else(handle_loader_error) as u32;
}

//#[link(wasm_import_module = "env")]
//extern { fn yield_result(string_offset: u32, string_length: u32, stroke_offset: u32, stroke_length: u32); }
//
//// if find_stroke == 0, performs a normal lookup using the query term starting at the given offset
////                      with the given length
//// if find_stroke == 1, performs a stroke lookup by interpreting the offset field as a stroke. length is unused.
//#[no_mangle]
//pub unsafe extern fn query(offset: u32, length: u32, data_offset: usize, find_stroke: u8) {
//
//    let offset_info = &*(data_offset as *const Header);
//    if offset_info.version != FORMAT_VERSION {
//        log_err_internal(error!(b"Dictionary format mismatch! Please remove the current dictionary and load it back in to store it in the current format.", b""));
//        panic!();
//    }
//    let hashmap_size = offset_info.stroke_index - offset_info.hash_table;
//    let hashmap = core::slice::from_raw_parts(
//        (offset_info.hash_table + data_offset) as *const usize,
//        hashmap_size / size_of::<usize>()
//    );
//    let stroke_prefix_lookup = core::slice::from_raw_parts(
//        (offset_info.stroke_index + data_offset) as *const usize,
//        257
//    );
//    let stroke_prefix_lookup_size = 257 * size_of::<usize>();
//
//    let stroke_subindices_offset = offset_info.stroke_index + stroke_prefix_lookup_size;
//    let stroke_subindices_size = offset_info.definitions - stroke_subindices_offset;
//    
//    let stroke_subindices = core::slice::from_raw_parts(
//        (stroke_subindices_offset + data_offset) as *const StrokeIndexEntry,
//        stroke_subindices_size / size_of::<StrokeIndexEntry>()
//    );
//
//    let definitions_size = offset_info.end - offset_info.definitions;
//    let definitions = core::slice::from_raw_parts(
//        (offset_info.definitions + data_offset) as *const u8,
//        definitions_size
//    );
//
//    if find_stroke == 0 {
//        let query = core::slice::from_raw_parts(
//            offset as *const u8,
//            length as usize
//        );
//        query_internal(query, hashmap, definitions).unwrap_or_else(log_err_internal);
//    }
//    else {
//        let query_stroke = offset;
//        find_stroke_internal(query_stroke, stroke_prefix_lookup, stroke_subindices, definitions).unwrap_or_else(log_err_internal);
//    }
//}
