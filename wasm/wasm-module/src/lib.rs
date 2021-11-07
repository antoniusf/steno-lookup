// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![no_std]

use core::mem::size_of;
//use core::fmt::Write;
//use core::convert::TryInto;
//use core::borrow::Borrow;
use query_engine::{self, error, InternalError, DataStructuresContainer};

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
    core::arch::wasm32::unreachable();
}

fn handle_loader_error(error: InternalError) -> ! {
    log_err_internal(error);
    panic!();
}

const FORMAT_VERSION: u32 = 0x00_02_00_00;

#[repr(packed(4))]
struct Header {
    version: u32,
    usize_buffer_length: usize,
    u8_buffer_length: usize,
}

// Set up the allocation structure for the query engine
// 
struct Container {
    // this memory will never be deallocated as long as this program runs
    header: &'static Header,
    usize_buffer: &'static mut [usize],
    u8_buffer: &'static mut [u8],
}

impl DataStructuresContainer for Container {

    fn allocate (usize_buffer_length: usize, u8_buffer_length: usize) -> Container {

        let memory_needed =
            size_of::<Header>()
            + usize_buffer_length * size_of::<usize>() // requires align 4, maintains align 4
            + u8_buffer_length; // requires align 1, maintains align 1

        let wasm_page_size = 65536;
        // this is just a rounding-up division for ints.
        let number_of_new_pages = (memory_needed - 1) / wasm_page_size + 1;

        // allocate new memory
        let previous_mem_size_pages = core::arch::wasm32::memory_grow(0, number_of_new_pages);
        let new_memory_start = previous_mem_size_pages * wasm_page_size;

        unsafe {
            // previously, i made sure that the beginning of the page was
            // aligned. this is a fair bit of work and it should not be
            // necessary, tbh, so im leaving it out.

            let mut offset = 0;

            let header = &mut *((new_memory_start + offset) as *mut Header);

            offset += size_of::<Header>();

            let usize_buffer = core::slice::from_raw_parts_mut(
                (new_memory_start + offset) as *mut usize,
                usize_buffer_length
            );

            offset += usize_buffer_length * size_of::<usize>();

            let u8_buffer = core::slice::from_raw_parts_mut(
                (new_memory_start + offset) as *mut u8,
                u8_buffer_length
            );

            offset += u8_buffer_length;

            // whew
            assert_eq!(offset, memory_needed);

            header.version = FORMAT_VERSION;
            header.usize_buffer_length = usize_buffer_length;
            header.u8_buffer_length = u8_buffer_length;

            Container {
                header: header,
                usize_buffer: usize_buffer,
                u8_buffer: u8_buffer
            }
        }
    }

    fn get_usize_buffer(&self) -> &[usize] {
        &self.usize_buffer
    }

    fn get_usize_buffer_mut(&mut self) -> &mut [usize] {
        &mut self.usize_buffer
    }

    fn get_u8_buffer(&self) -> &[u8] {
        &self.u8_buffer
    }

    fn get_u8_buffer_mut(&mut self) -> &mut [u8] {
        &mut self.u8_buffer
    }

    fn get_both_buffers_mut(&mut self) -> (&mut [usize], &mut [u8]) {
        (&mut self.usize_buffer[..], &mut self.u8_buffer[..])
    }
}

#[no_mangle]
pub unsafe extern fn load_json(offset: u32, length: u32) -> u32 {
    let buffer = core::slice::from_raw_parts_mut(
        offset as *mut u8,
        length as usize
    );
    let container = query_engine::load_json_internal::<Container>(buffer).map_err(handle_loader_error).unwrap();

    return (container.header as *const Header) as u32;
}

#[link(wasm_import_module = "env")]
extern { fn yield_result(string_offset: u32, string_length: u32, stroke_offset: u32, stroke_length: u32); }

fn yield_result_internal(strokes: &[u8], translation: &[u8]) {
    unsafe {
        yield_result(
            translation.as_ptr() as u32, translation.len() as u32,
            strokes.as_ptr() as u32, strokes.len() as u32);
    }
}

// if find_stroke == 0, performs a normal lookup using the query term starting at the given offset
//                      with the given length
// if find_stroke == 1, performs a stroke lookup by interpreting the offset field as a stroke. length is unused.
#[no_mangle]
pub unsafe extern fn query(offset: u32, length: u32, data_offset: usize, find_stroke: u8) {

    let offset_info = &*(data_offset as *const Header);

    if offset_info.version != FORMAT_VERSION {
        log_err_internal(error!(b"Dictionary format mismatch! Please remove the current dictionary and load it back in to store it in the current format.", b""));
        panic!();
    }

    let usize_buffer_start = data_offset + size_of::<Header>();

    let u8_buffer_start = usize_buffer_start + offset_info.usize_buffer_length * size_of::<usize>();

    let usize_buffer = core::slice::from_raw_parts_mut(
        usize_buffer_start as *mut usize,
        offset_info.usize_buffer_length
    );

    let u8_buffer = core::slice::from_raw_parts_mut(
        u8_buffer_start as *mut u8,
        offset_info.u8_buffer_length
    );

    let mut container = Container {
        header: offset_info,
        usize_buffer: usize_buffer,
        u8_buffer: u8_buffer
    };

    let query = core::slice::from_raw_parts(
        offset as *const u8,
        length as usize
    );

    if find_stroke == 0 {
        query_engine::query_internal(query, &mut container, yield_result_internal).unwrap_or_else(log_err_internal);
    }
    else {
        query_engine::find_strokes_internal(query, &mut container, yield_result_internal).unwrap_or_else(log_err_internal);
    }
}
