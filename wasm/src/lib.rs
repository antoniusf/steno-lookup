// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![no_std]

use core::mem::size_of;
use core::fmt::Write;
use core::convert::TryInto;
use core::borrow::Borrow;

mod hashtable;
use hashtable::HashTableMaker;

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

struct InternalError<'a> {
    // message is the part of the error that's meant to be
    // simple, so everyone can understand it.
    message: &'a [u8],
    // details is the part of the error that's meant to give
    // more precise information on what went wrong.
    details: &'a [u8],
    line: u32
}

type InternalResult<T> = Result<T, InternalError<'static>>;

macro_rules! error {
    ($message:expr, $details:expr) => {
        InternalError {
            message: $message.as_ref(),
            details: $details.as_ref(),
            line: line!()
        }
    };
}

static PARSER_ERROR: &'static [u8; 83] = b"Sorry, we couldn't load your dictionary because we don't understand its formatting.";

#[no_mangle]
pub unsafe extern fn load_json(offset: u32, length: u32) -> u32 {
    let buffer = core::slice::from_raw_parts_mut(
        offset as *mut u8,
        length as usize
    );
    return load_json_internal(buffer).unwrap_or_else(handle_loader_error) as u32;
}

#[repr(packed(4))]
struct Header {
    version: u32,
    strokes_buckets: usize,
    strings_buckets: usize,
    strokes_data: usize,
    strings_data: usize,
    end: usize
}

// the sole purpose of this is so I can use write! for debugging
// (no, simple &mut [u8]s won't work, since those can be written to
//  but only using std::io::Write, which we can't use in no_std
struct WriteBuffer<'a> {
    buffer: &'a mut [u8],
    position: usize
}

impl<'a> Write for WriteBuffer<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let space_remaining = self.buffer.len() - self.position;
        
        if space_remaining >= bytes.len() {
            self.buffer[self.position .. self.position + bytes.len()]
                .copy_from_slice(bytes);

            self.position += bytes.len();
            Ok(())
        }
        else {
            Err(core::fmt::Error)
        }
    }
}

fn compare_with_iterator(data: &[u8], iterator: impl Iterator<Item = impl Borrow<u8>>) -> bool {
    let data_iterator = data.iter().map(|val| *val);

    iterator.map(|val| *val.borrow()).eq(data_iterator)
}

#[derive(Clone)]
struct BufferIterator<'a> {
    offset: usize,
    buffer: &'a [u8]
}

impl<'a> BufferIterator<'a> {
    fn new(buffer: &'a [u8]) -> BufferIterator<'a> {
        BufferIterator {
            offset: 0,
            buffer
        }
    }
}

impl<'a> Iterator for BufferIterator<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {

        if self.offset >= self.buffer.len() {
            return None;
        }

        let length = u16::from_ne_bytes(
            self.buffer[self.offset .. self.offset + 2]
            .try_into().unwrap()) as usize;

        let data = &self.buffer[self.offset + 2 .. self.offset + length];
        self.offset += length;

        Some(data)
    }
}

#[derive(Clone)]
struct AllTranslationsIterator<'a> {
    buffer_iterator: BufferIterator<'a>
}

impl<'a> AllTranslationsIterator<'a> {
    fn new(buffer: &'a [u8]) -> AllTranslationsIterator<'a> {
        AllTranslationsIterator {
            buffer_iterator: BufferIterator::new(buffer)
        }
    }
}

impl<'a> Iterator for AllTranslationsIterator<'a> {
    type Item = core::slice::Iter<'a, u8>;

    fn next(&mut self) -> Option<core::slice::Iter<'a, u8>> {
        let _strokes = self.buffer_iterator.next()?;
        let translation = self.buffer_iterator.next()
            .map(|slice| slice.iter());

        translation
    }
}

#[derive(Clone)]
struct AllStrokesIterator<'a> {
    buffer_iterator: BufferIterator<'a>
}

impl<'a> AllStrokesIterator<'a> {
    fn new(buffer: &'a [u8]) -> AllStrokesIterator<'a> {
        AllStrokesIterator {
            buffer_iterator: BufferIterator::new(buffer)
        }
    }
}

impl<'a> Iterator for AllStrokesIterator<'a> {
    type Item = ParseStrokesIterator<'a>;

    fn next(&mut self) -> Option<ParseStrokesIterator<'a>> {
        let strokes = self.buffer_iterator.next();
        let _translation = self.buffer_iterator.next()?;

        if let Some(raw_stroke_data) = strokes {
            Some(ParseStrokesIterator::new(raw_stroke_data))
        }
        else {
            None
        }
    }
}

#[derive(Clone)]
struct ParseStrokesIterator<'a> {
    raw_strokes_data: &'a [u8],
    read_index: usize,
    current_stroke: u32
}

impl<'a> ParseStrokesIterator<'a> {
    fn new(raw_strokes_data: &'a [u8]) -> ParseStrokesIterator<'a> {
        ParseStrokesIterator {
            raw_strokes_data,
            read_index: 0,
            current_stroke: 0
        }
    }
}

impl<'a> Iterator for ParseStrokesIterator<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        if self.current_stroke == 0 {
            if self.read_index < self.raw_strokes_data.len() {
                let stroke = parse_stroke_fast(self.raw_strokes_data, &mut self.read_index);
                self.current_stroke = stroke >> 8;

                Some(stroke as u8)
            }
            else {
                None
            }
        }
        else { // self.current_stroke != 0
            let stroke = self.current_stroke;
            self.current_stroke >>= 8;

            Some(stroke as u8)
        }
    }
}


const FORMAT_VERSION: u32 = 0x00_01_00_03;

// loads a json array into our custom memory format.
fn load_json_internal(mut buffer: &mut [u8]) -> InternalResult<usize> {
    // in-place parsing turned out to not be possible in the end.
    // so, we're not going to do it.
    //
    // to get rid of the extra memory, the js will have to copy out
    // the important data, dump the wasm instance, and hope that the
    // memory will get gc'd.

    // step 1:
    // pre-parse the json.
    // this serves two purposes: first, validating the json input,
    // (and converting it into a binary format that's easier to read)
    // and second, determining the sizes of the required packed arrays,
    // so we know how much to allocate.
    //
    // note: this is not a full-fledged json parser. it is specifically
    // designed for reading plover dicionaries, and it will fail when
    // passed otherwise valid json that does not fit this schema.
    //
    // the binary format is basically just a '"'-terminated strokes
    // list, followed by a 0-terminated string, followed by the next
    // entry. the conversion to the binary format will happen
    // in-place, which is not a problem, since the binary format
    // is shorter than the original data format. we'll read the
    // original data from the read pointer, convert it, then write
    // it out to the write pointer.
    //
    // unfortunately, we can't use an iterator for all this since we
    // need to be able to write the slice while doing this.

    let mut read_pos = 0;
    let mut write_pos = 0;

    // INVARIANT: read_pos >= write_pos

    skip_whitespace(buffer, &mut read_pos).or(Err(error!(PARSER_ERROR, b"Parser error: no data found")))?;
    expect_char(buffer, &mut read_pos, b'{')?;
    
    // INVARIANT: read_pos >= write_pos + 1

    loop {

        // read key
        skip_whitespace(buffer, &mut read_pos)?;

        // validate/rewrite the strokes string
        rewrite_string(&mut buffer, &mut read_pos, &mut write_pos, true)?;
        // INVARIANT: read_pos >= write_pos + 1

        skip_whitespace(buffer, &mut read_pos)?;
        expect_char(buffer, &mut read_pos, b':')?;
        skip_whitespace(buffer, &mut read_pos)?;

        // read value
        rewrite_string(&mut buffer, &mut read_pos, &mut write_pos, false)?;
        // INVARIANT: read_pos >= write_pos + 1
        // (note: we could get better bounds in practice, since each
        //  string read actually increases the space by one. but we
        //  don't need these, so this is how it's going to stay.)

        skip_whitespace(buffer, &mut read_pos)?;

        let byte = buffer.get(read_pos).ok_or(error!(PARSER_ERROR, b"Parser error: data incomplete"))?;
        if *byte == b'}' {
            // reached file end
            //#[allow(unused_assignments)]
            read_pos += 1;
            break;
        }
        expect_char(buffer, &mut read_pos, b',')?;
    }

    // first pass is done, the data is parsed.
    // we can now use this to start initializing the hash tables.
    let hash_table_load_factor = 10.0;

    let strokes_iterator = AllStrokesIterator::new(buffer);
    let mut strokes_table_maker = HashTableMaker::initialize(strokes_iterator.clone());
    strokes_table_maker.set_load_factor(hash_table_load_factor);

    let strings_iterator = AllTranslationsIterator::new(buffer);
    let mut strings_table_maker = HashTableMaker::initialize(strings_iterator.clone());
    strings_table_maker.set_load_factor(hash_table_load_factor);

    let memory_needed = size_of::<Header>()
        + strokes_table_maker.get_buckets_length() * size_of::<usize>() // requires align 4, maintains align 4
        + strings_table_maker.get_buckets_length() * size_of::<usize>() // requires align 4, maintains align 4
        + strokes_table_maker.get_data_length() // requires align 1, maintains align 1
        + strings_table_maker.get_data_length(); // requires align 1, maintains align 1

    let wasm_page_size = 65536;
    // this is just a rounding-up division for ints.
    let number_of_new_pages = (memory_needed - 1) / wasm_page_size + 1;

    // allocate new memory
    let previous_mem_size_pages = core::arch::wasm32::memory_grow(0, number_of_new_pages);
    let new_memory_start = previous_mem_size_pages * wasm_page_size;

    let offset_info;
    let strokes_buckets;
    let strings_buckets;
    let strokes_data;
    let strings_data;

    unsafe {
        // previously, i made sure that the beginning of the page was
        // aligned. this is a fair bit of work and it should not be
        // necessary, tbh, so im leaving it out.
        //
        // im doing all of this in a single block, so that all of the
        // intermediate values will go out of scope and get dropped
        // cleanly when we are done here.

        let mut offset = 0;
        offset_info = &mut *((new_memory_start + offset) as *mut Header);
        offset += size_of::<Header>();

        strokes_buckets = core::slice::from_raw_parts_mut(
            (new_memory_start + offset) as *mut usize,
            strokes_table_maker.get_buckets_length()
        );

        offset += strokes_table_maker.get_buckets_length() * size_of::<usize>();

        strings_buckets = core::slice::from_raw_parts_mut(
            (new_memory_start + offset) as *mut usize,
            strings_table_maker.get_buckets_length()
        );

        offset += strings_table_maker.get_buckets_length() * size_of::<usize>();

        strokes_data = core::slice::from_raw_parts_mut(
            (new_memory_start + offset) as *mut u8,
            strokes_table_maker.get_data_length()
        );

        offset += strokes_table_maker.get_data_length();

        strings_data = core::slice::from_raw_parts_mut(
            (new_memory_start + offset) as *mut u8,
            strings_table_maker.get_data_length()
        );

        offset += strings_table_maker.get_data_length();

        assert!(offset == memory_needed);

        // whew
    }

    // store our offset_info
    offset_info.version = FORMAT_VERSION;
    offset_info.strokes_buckets = strokes_buckets.as_ptr() as usize - new_memory_start;
    offset_info.strings_buckets = strings_buckets.as_ptr() as usize - new_memory_start;
    offset_info.strokes_data = strokes_data.as_ptr() as usize - new_memory_start;
    offset_info.strings_data = strings_data.as_ptr() as usize - new_memory_start;
    offset_info.end = (*offset_info).strings_data + strings_data.len();

    // make the hash tables!
    let mut strokes_table = strokes_table_maker.make_hash_table(strokes_buckets, strokes_data);
    let mut strings_table = strings_table_maker.make_hash_table(strings_buckets, strings_data);

    // write our values
    for (strokes, translation) in strokes_iterator.zip(strings_iterator) {
        let mut strokes_bucket_iterator = strokes_table.get_bucket_iterator_from_key_iterator(strokes.clone());
        let strokes_entry = strokes_bucket_iterator
            .find(|entry| compare_with_iterator(entry.key, strokes.clone()) && entry.value == u32::MAX)
            .expect("Populating hash table: no fitting entry found!");

        let mut strings_bucket_iterator = strings_table.get_bucket_iterator_from_key_iterator(translation.clone());
        let translation_entry = strings_bucket_iterator
            .find(|entry| compare_with_iterator(entry.key, translation.clone()) && entry.value == u32::MAX)
            .expect("Populating hash table: no fitting entry found!");

        let translation_offset = translation_entry.get_offset();
        let strokes_offset = translation_entry.get_offset();

        // we have to do this before the set_value call because otherwise rustc
        // can't figure out the borrows
        let strokes_entry_handle = strokes_entry.to_handle();
        let translation_entry_handle = translation_entry.to_handle();

        strokes_table.set_value(strokes_entry_handle, translation_offset.try_into().unwrap());
        strings_table.set_value(translation_entry_handle, strokes_offset.try_into().unwrap());
    }

    return Ok(new_memory_start);
}

// rewrites a json string into a length-prefixed version, un-escaping simple
// escapes, and checking validity for the stroke strings.
fn rewrite_string<'a>(buffer: &mut[u8], read_pos: &mut usize, write_pos: &mut usize, is_strokes: bool) -> InternalResult<()> {


    // EXPECTATION: read_pos >= write_pos + 1

    // we're just going to turn the json strings
    // into length-prefixed strings (2 bytes)
    // 
    // yes, there are going to be lots of unaligned reads,
    // but worst-case, these should translate to two byte-sized
    // reads plus a shift and an or. i'd considered making some
    // sort of variable-length encoding for this, but i don't see
    // a way to get significant improvements in performance like that.
    //
    // I'm just going to put this here in case i need it later
    // return Err(error!(b"I'm sorry, but we can't handle your dictionary.", b"There is nothing wrong with it, except that it has at least one really reaaallly long entry, and this is not something that our internal format can deal with."));

    expect_char(&buffer, read_pos, b'"')?;
    // INVARIANT: read_pos >= write_pos + 2

    let length_header_offset = *write_pos;
    *write_pos += 2;
    // INVARIANT: read_pos >= write_pos

    let mut escape_next = false;
    let mut num_strokes = 1;

    loop {
        // INVARIANT: read_pos >= write_pos and (escape_next == true => read_pos >= write_pos + 1)
        
        let byte = *buffer.get(*read_pos).ok_or(error!(PARSER_ERROR, b"Parser error: data ended in the middle of string"))?;
        *read_pos += 1;
        // INVARIANT: read_pos >= write_pos + 1 and (escape_next == true => read_pos >= write_pos + 2)

        if escape_next {
            // INVARIANT: read_pos >= write_pos + 2
            if byte == b'"' || byte == b'\\' {
                buffer[*write_pos] = byte;
                *write_pos += 1;
                // INVARIANT: read_pos >= write_pos + 1
            }
            else {
                // copy the escape sequence unchanged
                buffer[*write_pos] = b'\\';
                buffer[*write_pos+1] = byte;
                *write_pos += 2;
                // INVARIANT: read_pos >= write_pos
            }
            escape_next = false;
            // INVARIANT: escape_next == false and read_pos >= write_pos
        }

        else {
            // INVARIANT: read_pos >= write_pos + 1
            if byte == b'"' {
                break;
                // INVARIANT: read_pos >= write_pos + 1
            }
            else if byte == b'\\' {
                if is_strokes {
                    // the stroke parser can't handle those, so we have to make sure
                    // they won't be in there.
                    return Err(error!(PARSER_ERROR, b"Parser error: escape sequence found in stroke definition"));
                }
                escape_next = true;
                // INVARIANT: escape_next == true and read_pos >= write_pos + 1
            }
            else {
                buffer[*write_pos] = byte;
                *write_pos += 1;
                // INVARIANT: read_pos >= write_pos + 1
            }

            // BRANCH INVARIANTS:
            //   IF: (omitted, control flow doesn't land here)
            //   ELSE IF: escape_next == true and read_pos >= write_pos + 1
            //   ELSE: read_pos >= write_pos
            //
            // INVARIANT: read_pos >= write_pos and (escape_next == true => read_pos >= write_pos + 1)

            if is_strokes && byte == b'/' {
                num_strokes += 1;
            }
        }

        // BRANCH INVARIANTS:
        //   IF: escape_next == false and read_pos >= write_pos
        //   ELSE: read_pos >= write_pos and (escape_next == true => read_pos >= write_pos + 1)
        //
        // INVARIANT: read_pos >= write_pos and (escape_next == true => read_pos >= write_pos + 1)
    }
    // INVARIANT: read_pos >= write_pos + 1 (there is only one break from the above loop)

    // validation: the max number of strokes is 1000, this restriction comes from BufferIterator
    if num_strokes > 1000 {
        return Err(error!(b"I'm sorry, but we can't handle your dictionary.", b"There is nothing wrong with it, except that one definition uses more than a thousand strokes, which is not something we can currently deal with."));
    }

    let length: u16 = (*write_pos - length_header_offset).try_into()
        .map_err(|_e| error!(b"I'm sorry, but we can't handle your dictionary.", b"There is nothing wrong with it, except that it has at least one really reaaallly long entry, and this is not something that our internal format can deal with."))?;

    buffer[length_header_offset .. length_header_offset + 2]
        .copy_from_slice(&(length as u16).to_ne_bytes());

    // INVARIANT: read_pos >= write_pos + 1

    Ok(())
}

fn skip_whitespace(buffer: &[u8], pos: &mut usize) -> InternalResult<()> {
    while let Some(&byte) = buffer.get(*pos) {
        if byte != b' '
            && byte != b'\t'
            && byte != b'\r'
            && byte != b'\n'
         {
            // character is non-whitespace. don't advance position,
            // return and let the caller continue.
            return Ok(());
        }

        *pos += 1;
    }
    return Err(error!(PARSER_ERROR, b"Parser error: data incomplete"));
}

fn expect_char(buffer: &[u8], pos: &mut usize, expected: u8) -> InternalResult<()> {
    if let Some(byte) = buffer.get(*pos) {
        if *byte == expected {
            *pos += 1;
            return Ok(());
        }
        else {
            // todo: maybe find a more elegant way to statically determine
            // the interpolation positions?
            let message = b"Parser error: expected '$', but got '$'";
            // copy this onto the stack (i think) so we can format it.
            let mut formatted_message = *message;
            formatted_message[24] = expected;
            formatted_message[37] = *byte;
            log_err_internal(error!(PARSER_ERROR, &formatted_message));
            return Err(error!(PARSER_ERROR, message));
        }
    }
    else {
        let message = b"Parser error: expected '$', but hit the end of data";
        let mut formatted_message = *message;
        formatted_message[24] = expected;
        log_err_internal(error!(PARSER_ERROR, &formatted_message));
        return Err(error!(PARSER_ERROR, message));
    }
}

// structure of this table:
//  - starts at ' ', 64 entries long
//  - it then repeats (so the same but bit 6 is set) for the right bank
//  - the three low bytes for each entry are the stroke bits
//  - bit 30 (ie bit 6 of the top byte) is a special bit that distinguishes between left and right bank
//  - the msb signals a stop condition
static STOP: u32 = 1 << 31;
static IGNORE: u32 = 0;
static NUMBER: u32 = 1 << 0;
static RIGHT_BANK: u32 = 1 << 30;

static PARSE_STROKE_TABLE: [u32; 128] = [
    IGNORE, // ' '
    IGNORE, // '!'
    STOP, // "
    NUMBER, // #
    IGNORE, // $
    IGNORE, // %
    IGNORE, // &
    IGNORE, // ' (this is ignore since json only does ""-delimited strings
    IGNORE, // (
    IGNORE, // )
    (1 << 10) | RIGHT_BANK, // *
    IGNORE, // +
    IGNORE, // ,
    RIGHT_BANK, // -
    IGNORE, // .
    STOP, // /
    NUMBER | (1 << 9) | RIGHT_BANK, // 0
    NUMBER | (1 << 1), // 1
    NUMBER | (1 << 2), // 2
    NUMBER | (1 << 4), // 3
    NUMBER | (1 << 6), // 4
    NUMBER | (1 << 8) | RIGHT_BANK, // 5
    NUMBER | (1 << 13), // 6
    NUMBER | (1 << 15), // 7
    NUMBER | (1 << 17), // 8
    NUMBER | (1 << 19), // 9
    IGNORE, // :
    IGNORE, // ;
    IGNORE, // <
    IGNORE, // =
    IGNORE, // >
    IGNORE, // ?
    IGNORE, // @
    (1 << 8) | RIGHT_BANK, // A
    IGNORE, // B
    IGNORE, // C
    IGNORE, // D
    (1 << 11) | RIGHT_BANK, // E
    IGNORE, // F
    IGNORE, // G
    (1 << 6), // H
    IGNORE, // I
    IGNORE, // J
    (1 << 3), // K
    IGNORE, // L
    IGNORE, // M
    IGNORE, // N
    (1 << 9) | RIGHT_BANK, // O
    (1 << 4), // P
    IGNORE, // Q
    (1 << 7), // R
    (1 << 1), // S
    (1 << 2), // T
    (1 << 12) | RIGHT_BANK, // U
    IGNORE, // V
    (1 << 5), // W
    IGNORE, // X
    IGNORE, // Y
    IGNORE, // Z

    // this is just filler to pad to 64 entries
    IGNORE, // [
    IGNORE, // \
    IGNORE, // ]
    IGNORE, // ^
    IGNORE, // _

    // right-bank table starts here
    // everything here needs to have the RIGHT_BANK bit set,
    // otherwise we'll fall back to the left bank, which shouldn't happen.
    RIGHT_BANK | IGNORE, // ' '
    RIGHT_BANK | IGNORE, // '!'
    RIGHT_BANK | STOP, // "
    RIGHT_BANK | NUMBER, // # <- this shouldn't actually appear right bank, but I'm not sure I care
    RIGHT_BANK | IGNORE, // $
    RIGHT_BANK | IGNORE, // %
    RIGHT_BANK | IGNORE, // &
    RIGHT_BANK | IGNORE, // ' (this is ignore since json only does ""-delimited strings
    RIGHT_BANK | IGNORE, // (
    RIGHT_BANK | IGNORE, // )
    RIGHT_BANK | (1 << 10), // *
    RIGHT_BANK | IGNORE, // +
    RIGHT_BANK | IGNORE, // ,
    RIGHT_BANK, // -
    RIGHT_BANK | IGNORE, // .
    RIGHT_BANK | STOP, // /
    RIGHT_BANK | NUMBER | (1 << 9), // 0
    RIGHT_BANK | NUMBER | (1 << 1), // 1
    RIGHT_BANK | NUMBER | (1 << 2), // 2
    RIGHT_BANK | NUMBER | (1 << 4), // 3
    RIGHT_BANK | NUMBER | (1 << 6), // 4
    RIGHT_BANK | NUMBER | (1 << 8), // 5
    RIGHT_BANK | NUMBER | (1 << 13), // 6
    RIGHT_BANK | NUMBER | (1 << 15), // 7
    RIGHT_BANK | NUMBER | (1 << 17), // 8
    RIGHT_BANK | NUMBER | (1 << 19), // 9
    RIGHT_BANK | IGNORE, // :
    RIGHT_BANK | IGNORE, // ;
    RIGHT_BANK | IGNORE, // <
    RIGHT_BANK | IGNORE, // =
    RIGHT_BANK | IGNORE, // >
    RIGHT_BANK | IGNORE, // ?
    RIGHT_BANK | IGNORE, // @
    RIGHT_BANK | (1 << 8), // A
    RIGHT_BANK | (1 << 16), // B
    RIGHT_BANK | IGNORE, // C
    RIGHT_BANK | (1 << 21), // D
    RIGHT_BANK | (1 << 11), // E
    RIGHT_BANK | (1 << 13), // F
    RIGHT_BANK | (1 << 18), // G
    RIGHT_BANK | IGNORE, // H
    RIGHT_BANK | IGNORE, // I
    RIGHT_BANK | IGNORE, // J
    RIGHT_BANK | IGNORE, // K
    RIGHT_BANK | (1 << 17), // L
    RIGHT_BANK | IGNORE, // M
    RIGHT_BANK | IGNORE, // N
    RIGHT_BANK | (1 << 9), // O
    RIGHT_BANK | (1 << 15), // P
    RIGHT_BANK | IGNORE, // Q
    RIGHT_BANK | (1 << 14), // R
    RIGHT_BANK | (1 << 20), // S
    RIGHT_BANK | (1 << 19), // T
    RIGHT_BANK | (1 << 12), // U
    RIGHT_BANK | IGNORE, // V
    RIGHT_BANK | IGNORE, // W
    RIGHT_BANK | IGNORE, // X
    RIGHT_BANK | IGNORE, // Y
    RIGHT_BANK | (1 << 22), // Z

    RIGHT_BANK | // this is just filler to pad to 64 entries
    RIGHT_BANK | IGNORE, // [
    RIGHT_BANK | IGNORE, // \
    RIGHT_BANK | IGNORE, // ]
    RIGHT_BANK | IGNORE, // ^
    RIGHT_BANK | IGNORE, // _
    
];

// pos is a pointer, so the calling code can pick up
// where we left off
// TODO: what about zero-length strokes or other malformed input? (error handling??)
fn parse_stroke_fast(buffer: &[u8], pos: &mut usize) -> u32 {

    let mut state = 0;
    let mut stroke = 0;
    while (state & (1 << 7)) == 0 && *pos < buffer.len() {
        let byte = buffer[*pos];
        *pos += 1;

        // first, cast up to u32, since we'll have to go up anyways.
        //  (the other option would be downcasting state, but that's
        //   just unnecessary effort and it doesn't matter anyways)
        // then, subtract 32, since the table starts there.
        // then, clear the top 2 bits, in case we get nasty input.
        // then add in the right-bank bit, which is in state bit 6.
        // note that this is also the position we need for the lookup!
        let index = ((byte as u32 - 32) & 63) | (state & (1 << 6));
        let result = PARSE_STROKE_TABLE[index as usize];
        stroke |= result & 0x00FFFFFF;
        state = result >> 24 as u8;
    }

    return stroke;
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
//
//fn query_internal(query: &[u8], hashmap: &[usize], definitions: &[u8]) -> InternalResult<()> {
//
//    let hash = wyhash(query, 1);
//    let index = (hash as usize) % hashmap.len();
//
//    let bucket_offset = hashmap[index];
//    if bucket_offset == usize::max_value() {
//        // no results
//        return Ok(());
//    }
//
//    for definition in BucketEntryIterator::new(&definitions[bucket_offset..]) {
//        
//        let is_match = (definition.len() > query.len())
//            && (definition[query.len()] == 0)
//            && (&definition[0 .. query.len()] == query);
//
//        if is_match {
//
//            let string = &definition[0 .. query.len()];
//
//            let strokes_start = query.len() + 1;
//            let mut stroke_pos = strokes_start;
//
//            // read strokes
//            loop {
//
//                let stroke1 = definition[stroke_pos] as u32;
//                let stroke2 = definition[stroke_pos+1] as u32;
//                let stroke3 = definition[stroke_pos+2] as u32;
//                stroke_pos += 3;
//
//                let stroke = stroke1 | (stroke2 << 8) | (stroke3 << 16);
//                if (stroke >> 23) == 1 {
//                    break;
//                }
//            }
//            let strokes_end = stroke_pos;
//            let strokes = &definition[strokes_start..strokes_end];
//
//            unsafe {
//                yield_result(string.as_ptr() as u32, string.len() as u32, strokes.as_ptr() as u32, (strokes_end - strokes_start) as u32);
//            }
//        }
//    }
//
//    return Ok(());
//}
//
//fn find_stroke_internal(mut query_stroke: u32, stroke_prefix_lookup: &[usize], stroke_subindices: &[StrokeIndexEntry], definitions: &[u8]) -> InternalResult<()> {
//
//    // currently, we're storing the strokes with the last stroke marker,
//    // so we'll have to add that in
//    query_stroke |= 1 << 23;
//
//    let first_byte = query_stroke & 0xFF;
//    let last_two_bytes = query_stroke >> 8;
//
//    let subindex_start = stroke_prefix_lookup[first_byte as usize];
//    let subindex_end = stroke_prefix_lookup[first_byte as usize + 1];
//    let subindex = &stroke_subindices[subindex_start..subindex_end];
//
//    let result = subindex.binary_search_by_key(&(last_two_bytes as u16), stroke_index_sortkey);
//
//    if let Ok(index) = result {
//        let entry = &subindex[index];
//        // skip the header
//        let definition_offset = entry.definition_offset + 2;
//
//        let string_start = definition_offset;
//        let string_end = *(&definitions[string_start..].iter().position(|&byte| byte == 0).unwrap()) + string_start;
//        let string = &definitions[string_start..string_end];
//        let strokes_start = string_end + 1;
//        let strokes_end = *(&definitions[strokes_start..].chunks_exact(3).position(|stroke| (stroke[2] >> 7) == 1).unwrap()) * 3 + 3 + strokes_start;
//        let strokes = &definitions[strokes_start..strokes_end];
//
//        unsafe {
//            yield_result(string.as_ptr() as u32, string.len() as u32, strokes.as_ptr() as u32, (strokes_end - strokes_start) as u32);
//        }
//    }
//
//    return Ok(());
//}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     
//     #[test]
//     fn test_parse_stroke() {
//         // TODO: I've just precomputed these values and checked that they are correct.
//         // but there should be a better way.
//         let mut pos = 0;
//         assert_eq!(parse_stroke_fast(b"KPWHREPLGS/", &mut pos), 1476856);
//         pos = 0;
//         assert_eq!(parse_stroke_fast(b"K-FRBL/", &mut pos), 221192);
//         pos = 0;
//         assert_eq!(parse_stroke_fast(b"#AO/", &mut pos), 769);
//         pos = 0;
//         assert_eq!(parse_stroke_fast(b"50/", &mut pos), 769);
//         pos = 0;
//     }
// }
