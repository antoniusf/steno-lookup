// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![cfg_attr(all(not(test), target_family = "wasm"), no_std)]

// TODO temporary!!
#![allow(dead_code)]

use core::fmt::Write;
use core::convert::TryInto;
use core::borrow::Borrow;

mod hashtable;

use hashtable::{HashTableMaker, HashTable};

#[cfg_attr(test, derive(Debug))]
pub struct InternalError<'a> {
    // message is the part of the error that's meant to be
    // simple, so everyone can understand it.
    pub message: &'a [u8],
    // details is the part of the error that's meant to give
    // more precise information on what went wrong.
    pub details: &'a [u8],
    pub line: u32
}

type InternalResult<T> = Result<T, InternalError<'static>>;

#[macro_export]
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
            //println!("parsing strokes: {}", std::str::from_utf8(raw_stroke_data).unwrap());
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
    current_stroke: u32,
    bytes_left_in_current_stroke: usize,
}

impl<'a> ParseStrokesIterator<'a> {
    fn new(raw_strokes_data: &'a [u8]) -> ParseStrokesIterator<'a> {
        ParseStrokesIterator {
            raw_strokes_data,
            read_index: 0,
            current_stroke: 0,
            bytes_left_in_current_stroke: 0,
        }
    }
}

impl<'a> Iterator for ParseStrokesIterator<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        if self.bytes_left_in_current_stroke == 0 {
            if self.read_index < self.raw_strokes_data.len() {
                let stroke = parse_stroke_fast(self.raw_strokes_data, &mut self.read_index);
                self.current_stroke = stroke >> 8;
                self.bytes_left_in_current_stroke = 2;

                Some(stroke as u8)
            }
            else {
                None
            }
        }
        else { // self.bytes_left_in_current_stroke != 0
            let stroke = self.current_stroke;
            self.current_stroke >>= 8;
            self.bytes_left_in_current_stroke -= 1;

            Some(stroke as u8)
        }
    }
}

pub trait DataStructuresContainer {
    fn allocate(len_usize_buffer: usize, len_u8_buffer: usize) -> Self;
    fn get_usize_buffer(&self) -> &[usize];
    fn get_usize_buffer_mut(&mut self) -> &mut [usize];
    fn get_u8_buffer(&self) -> &[u8];
    fn get_u8_buffer_mut(&mut self) -> &mut [u8];

    // borrowing both buffers mutably would not be possible otherwise
    fn get_both_buffers_mut(&mut self) -> (&mut [usize], &mut [u8]);
}

// loads a json array into our custom memory format.
pub fn load_json_internal<ContainerType>(mut buffer: &mut [u8]) -> InternalResult<ContainerType>
    where ContainerType: DataStructuresContainer
{
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
            //read_pos += 1;
            break;
        }
        expect_char(buffer, &mut read_pos, b',')?;
    }

    // first pass is done, the data is parsed.
    // we can now use this to start initializing the hash tables.
    let hash_table_load_factor = 10.0;

    let strokes_iterator = AllStrokesIterator::new(&buffer[..write_pos]);
    let mut strokes_table_maker = HashTableMaker::initialize(strokes_iterator.clone());
    strokes_table_maker.set_load_factor(hash_table_load_factor);

    let strings_iterator = AllTranslationsIterator::new(&buffer[..write_pos]);
    let mut strings_table_maker = HashTableMaker::initialize(strings_iterator.clone());
    strings_table_maker.set_load_factor(hash_table_load_factor);

    let usize_buffer_length = 2
        + strokes_table_maker.get_buckets_length()
        + strings_table_maker.get_buckets_length();

    let u8_buffer_length = 0
        + strokes_table_maker.get_data_length()
        + strings_table_maker.get_data_length();

    let mut container = ContainerType::allocate(usize_buffer_length, u8_buffer_length);
    let (usize_buffer, u8_buffer) = container.get_both_buffers_mut();

    // store the length of the strokes table arrays, so we'll remember where the
    // strings table arrays start
    usize_buffer[0] = strokes_table_maker.get_buckets_length();
    usize_buffer[1] = strokes_table_maker.get_data_length();

    let (strokes_buckets, strings_buckets) =
        usize_buffer[2..]
        .split_at_mut(strokes_table_maker.get_buckets_length());

    let (strokes_data, strings_data) = 
        u8_buffer
        .split_at_mut(strokes_table_maker.get_data_length());

    // make the hash tables!
    //println!("making strokes table");
    let mut strokes_table = strokes_table_maker.make_hash_table(strokes_buckets, strokes_data);
    //println!("making strings table");
    let mut strings_table = strings_table_maker.make_hash_table(strings_buckets, strings_data);

    //println!("writing values");

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
        let strokes_offset = strokes_entry.get_offset();

        // we have to do this before the set_value call because otherwise rustc
        // can't figure out the borrows
        let strokes_entry_handle = strokes_entry.to_handle();
        let translation_entry_handle = translation_entry.to_handle();

        strokes_table.set_value(strokes_entry_handle, translation_offset.try_into().unwrap());
        strings_table.set_value(translation_entry_handle, strokes_offset.try_into().unwrap());
    }

    return Ok(container);
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
            // TODO: if we add wee_alloc, we can output better error messages again
            //log_err_internal(error!(PARSER_ERROR, &formatted_message));
            return Err(error!(PARSER_ERROR, message));
        }
    }
    else {
        let message = b"Parser error: expected '$', but hit the end of data";
        let mut formatted_message = *message;
        formatted_message[24] = expected;
        //log_err_internal(error!(PARSER_ERROR, &formatted_message));
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

fn get_hashtables_from_container(container: &mut impl DataStructuresContainer) -> InternalResult<(HashTable, HashTable)> {

    let (usize_buffer, u8_buffer) = container.get_both_buffers_mut();

    let strokes_buckets_length = usize_buffer[0];
    let strokes_data_length = usize_buffer[1];

    let (strokes_buckets, strings_buckets) =
        usize_buffer[2..]
        .split_at(strokes_buckets_length);

    let (strokes_data, strings_data) = 
        u8_buffer
        .split_at_mut(strokes_data_length);

    Ok((
        HashTable {
            buckets: strokes_buckets,
            data: strokes_data
        },
        HashTable {
            buckets: strings_buckets,
            data: strings_data
        }
    ))
}

pub fn query_internal<F>(query: &[u8], container: &mut impl DataStructuresContainer, mut yield_result: F) -> InternalResult<()>
    where F: FnMut(&[u8], &[u8])
{
    let (strokes_table, strings_table) = get_hashtables_from_container(container)?;
    for strokes_offset in strings_table.get_values(query) {
        let strokes = hashtable::Entry::new(strokes_table.data, strokes_offset as usize).key;
        yield_result(strokes, query);
    }

    Ok(())
}

fn parse_strokes_query(query: &[u8], parsed_strokes_buffer: &mut [u8]) -> InternalResult<usize> {

    let mut pos = 0;

    for strokes_byte in ParseStrokesIterator::new(query) {
        if pos >= parsed_strokes_buffer.len() {
            return Err(error!(b"parse strokes error", b"strokes don't fit in provided buffer"));
        }

        parsed_strokes_buffer[pos] = strokes_byte;
        pos += 1;
    }

    return Ok(pos);
}

pub fn find_strokes_internal<F>(query: &[u8], container: &mut impl DataStructuresContainer, mut yield_result: F) -> InternalResult<()>
    where F: FnMut(&[u8], &[u8])
{
    let (strokes_table, strings_table) = get_hashtables_from_container(container)?;

    // TODO: figure out the allocation story for temporary buffers
    // limit query to 32 strokes
    let mut parsed_query_buffer = [0u8; 32*3];
    let parsed_len = parse_strokes_query(query, &mut parsed_query_buffer)?;
    let parsed_query = &parsed_query_buffer[..parsed_len];

    for strings_offset in strokes_table.get_values(parsed_query) {
        let translation = hashtable::Entry::new(strings_table.data, strings_offset as usize).key;
        yield_result(parsed_query, translation);
    }

    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_stroke() {
        // TODO: I've just precomputed these values and checked that they are correct.
        // but there should be a better way.
        let mut pos = 0;
        assert_eq!(parse_stroke_fast(b"KPWHREPLGS/", &mut pos), 1476856);
        pos = 0;
        assert_eq!(parse_stroke_fast(b"K-FRBL/", &mut pos), 221192);
        pos = 0;
        assert_eq!(parse_stroke_fast(b"#AO/", &mut pos), 769);
        pos = 0;
        assert_eq!(parse_stroke_fast(b"50/", &mut pos), 769);
    }

    struct Container {
        usize_buffer: Vec<usize>,
        u8_buffer: Vec<u8>
    }

    impl DataStructuresContainer for Container {

        fn allocate(usize_buffer_len: usize, u8_buffer_len: usize) -> Container {
            println!("usize buffer len: {}", usize_buffer_len);
            println!("u8 buffer len: {}", u8_buffer_len);
            println!("total memory usage: {} kiB", (usize_buffer_len * 4 + u8_buffer_len) / 1024);
            Container {
                usize_buffer: vec![0usize; usize_buffer_len],
                u8_buffer: vec![0u8; u8_buffer_len]
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

    const STENO_ORDER: &str = "#STKPWHRAO*EUFRPBLGTSDZ";

    fn format_stroke(stroke: u32) -> String {
        STENO_ORDER.chars().enumerate()
            .filter_map(|(i, val)| {
                if (stroke & (1 << i)) != 0 {
                    Some(val)
                }
                else {
                    None
                }
            })
            .collect()
    }

    fn format_strokes(strokes: &[u8]) -> String {
        strokes.chunks_exact(3).map(|stroke_bytes| {
            format_stroke(
                   (stroke_bytes[0] as u32)
                | ((stroke_bytes[1] as u32) << 8)
                | ((stroke_bytes[2] as u32) << 16)
            )
        }).fold(String::new(), |mut accumulator, stroke| {
            if accumulator.len() > 0 {
                accumulator.push_str("/");
            }
            accumulator.push_str(&stroke);
            accumulator
        })
    }

    #[test]
    fn test_loader() {
        let mut dictionary_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dictionary_path.push("resources/test/stanmain.json");
        let mut json_dict = std::fs::read(dictionary_path).unwrap();
        let mut container = load_json_internal::<Container>(&mut json_dict[..]).unwrap();

        println!("hashtable constructed!");

        query_internal(b"implicitly", &mut container, |strokes, translation| {
            println!("got result: {}, {}",
                     format_strokes(strokes),
                     std::str::from_utf8(translation).unwrap_or("<invalid utf-8>"));
        }).unwrap();

        find_strokes_internal(b"KPWHREUFLT", &mut container, |strokes, translation| {
            println!("got result: {}, {}",
                     format_strokes(strokes),
                     std::str::from_utf8(translation).unwrap_or("<invalid utf-8>"));
        }).unwrap();
    }
}
