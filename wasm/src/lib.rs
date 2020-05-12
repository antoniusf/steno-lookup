// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![no_std]

use core::mem::size_of;
use core::fmt::Write;

extern crate wyhash;
use wyhash::wyhash;

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
    hash_table: usize,
    stroke_index: usize,
    definitions: usize,
    end: usize
}

#[repr(packed(2))]
struct StrokeIndexEntry {
    last_two_bytes: u16,
    definition_offset: usize
}

struct Definition<'a> { 
    string: &'a [u8],
    strokes: &'a [u8]
}

struct BucketEntryIterator<'a> {
    buffer: &'a [u8],
    position: usize,
    last_definition_read: bool
}

impl<'a> BucketEntryIterator<'a> {
    /// Makes a new iterator
    ///
    /// # Arguments
    ///
    /// * `bucket` – slice of the underlying definitions buffer. the start of the
    /// slice must correspond to the start of the bucket, but the end of the slice
    /// can be beyond the end of the bucket, since this can detect the end itself.
    fn new(bucket: &'a [u8]) -> BucketEntryIterator<'a> {
        BucketEntryIterator { buffer: bucket, position: 0, last_definition_read: false }
    }

    fn get_next_free_slot(&self) -> Option<usize> {
        if self.last_definition_read {
            return Some(self.position);
        }
        else {
            return None;
        }
    }
}

impl<'a> Iterator for BucketEntryIterator<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {

        if self.last_definition_read {
            return None;
        }

        let header = (self.buffer[self.position] as u16) | ((self.buffer[self.position + 1] as u16) << 8);
        let is_last = header >> 15 == 1;
        let length = (header & ((1 << 15) - 1)) as usize;
        let definition = &self.buffer[self.position + 2 .. self.position + length];

        self.position += length;

        if is_last {
            self.last_definition_read = true;
        }

        return Some(definition);
    }
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
            &self.buffer[self.position .. self.position + bytes.len()]
                .copy_from_slice(bytes);

            self.position += bytes.len();
            Ok(())
        }
        else {
            Err(core::fmt::Error)
        }
    }
}

const FORMAT_VERSION: u32 = 0x00_01_00_01;

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

    // size of the definitions array in bytes
    let mut definitions_size = 0;
    let mut num_definitions = 0;

    // strokes will be stored in what i think is like a two-layer
    // prefix-tree? or something? the first layer is indexed by the
    // first byte and is sparse, the second layer is compact and
    // accessed using bsearch. we need to know the lengths of all
    // the layer2 subarrays so that we can pre-allocate the space
    // and so we know where each layer2 subarray starts and ends,
    // and we're doing this in the first pass so that during the
    // second pass we can place the strokes in the right place
    // directly.
    let mut stroke_subindex_lengths = [0; 256];

    // INVARIANT: read_pos >= write_pos

    skip_whitespace(buffer, &mut read_pos).or(Err(error!(PARSER_ERROR, b"Parser error: no data found")))?;
    expect_char(buffer, &mut read_pos, b'{')?;
    
    // INVARIANT: read_pos >= write_pos + 1

    loop {

        // read key
        skip_whitespace(buffer, &mut read_pos)?;

        // read/validate the strokes string first, then parse out the stroke
        let mut strokes_reader = write_pos;
        definitions_size += rewrite_string(&mut buffer, &mut read_pos, &mut write_pos, true)?;
        // INVARIANT: read_pos >= write_pos + 1

        // parse the strokes for the first time, to fill in stroke_index_layer2_lengths
        {
            let stroke = parse_stroke_fast(&buffer, &mut strokes_reader)?;

            // currently, we're only indexing single-stroke defs
            let is_last_stroke = buffer[strokes_reader] == b'"';
            if is_last_stroke {
                let first_byte = (stroke & 0xFF) as u8;
                stroke_subindex_lengths[first_byte as usize] += 1;
            }
        }

        skip_whitespace(buffer, &mut read_pos)?;
        expect_char(buffer, &mut read_pos, b':')?;
        skip_whitespace(buffer, &mut read_pos)?;

        // read value
        definitions_size += rewrite_string(&mut buffer, &mut read_pos, &mut write_pos, false)?;
        // INVARIANT: read_pos >= write_pos + 1
        // (note: we could get better bounds in practice, since each
        //  string read actually increases the space by one. but we
        //  don't need these, so this is how it's going to stay.)

        num_definitions += 1;
        // each definition gets an extra 2 bytes for its header
        definitions_size += 2;

        skip_whitespace(buffer, &mut read_pos)?;

        let byte = buffer.get(read_pos).ok_or(error!(PARSER_ERROR, b"Parser error: data incomplete"))?;
        if *byte == b'}' {
            // reached file end
            read_pos += 1;
            break;
        }
        expect_char(buffer, &mut read_pos, b',')?;
    }

    // first pass is done.
    // we can now compute the sizes for all of the necessary data structures
    // and allocate them.

    let hash_table_load_factor = 0.85;
    let hash_table_length = (num_definitions as f64 / hash_table_load_factor) as usize;
    let hash_table_size = hash_table_length * size_of::<usize>();

    // determine the length of the stroke index
    // stroke_index_subindex_lengths contains the length of each subindex,
    // so if we add them all up we get the total amount of space needed
    let stroke_subindices_length = stroke_subindex_lengths.iter().sum();
    let stroke_subindices_size = stroke_subindices_length * size_of::<StrokeIndexEntry>();
    let stroke_prefix_lookup_length = 257;
    let stroke_prefix_lookup_size = stroke_prefix_lookup_length * size_of::<usize>();

    let memory_needed = size_of::<Header>()
        + hash_table_size // requires align 4, maintains align 4
        + stroke_prefix_lookup_size // requires align 4, maintains align 4
        + stroke_subindices_size // requires align 2, maintains align 2
        + definitions_size; // requires align 1, maintains align 1

    let wasm_page_size = 65536;
    // this is just a rounding-up division for ints.
    let number_of_new_pages = (memory_needed - 1) / wasm_page_size + 1;

    // allocate new memory
    let previous_mem_size_pages = core::arch::wasm32::memory_grow(0, number_of_new_pages);
    let new_memory_start = previous_mem_size_pages * wasm_page_size;

    let hash_table;
    let stroke_prefix_lookup;
    let stroke_subindices;
    let definitions;
    let mut offset_info;

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

        hash_table = core::slice::from_raw_parts_mut(
            (new_memory_start + offset) as *mut usize,
            hash_table_length
        );

        offset += hash_table_size;

        stroke_prefix_lookup = core::slice::from_raw_parts_mut(
            (new_memory_start + offset) as *mut usize,
            stroke_prefix_lookup_length
        );

        offset += stroke_prefix_lookup_size;

        stroke_subindices = core::slice::from_raw_parts_mut(
            (new_memory_start + offset) as *mut StrokeIndexEntry,
            stroke_subindices_length
        );

        offset += stroke_subindices_size;

        definitions = core::slice::from_raw_parts_mut(
            (new_memory_start + offset) as *mut u8,
            definitions_size
        );

        offset += definitions_size;

        // whew
    }

    // initialization
    // hash_table is a sparse structure, so
    // we'll need to initialize it.
    //
    // this pass: the hash table will store the size of each of the
    // bucket entry arrays
    for elem in hash_table.iter_mut() {
        *elem = 0;
    }

    // also store our offset_info
    // length of the hash table is given by the start of stroke_index
    offset_info.version = FORMAT_VERSION;
    offset_info.hash_table = hash_table.as_ptr() as usize - new_memory_start;
    offset_info.stroke_index = stroke_prefix_lookup.as_ptr() as usize - new_memory_start;
    offset_info.definitions = definitions.as_ptr() as usize - new_memory_start;
    offset_info.end = (*offset_info).definitions + definitions.len();

    // for reference: the definitions format
    // definitions contains data for each of the buckets of the
    // hash table. each bucket can contain one or more items, and the
    // hash table contains an offset into definitions to where the array
    // for the given bucket begins. this array contains a number of variable
    // length entries, each of which has the following format: a 2-byte header
    // (in little endian), of which the msb indicates whether this is the last
    // entry (set to 1 if it is). the remaining 15 bits give the (unsigned) offset
    // to the next entry, in bytes. this is followed by the translation (string),
    // which is 0-terminated, and the strokes, in 3-byte compact format.
    // during initialization only, the first two bytes of the array can be set to 0x0000
    // (which would be a zero-length entry) to mark that no entries have been added yet.
    // this is safe to do, since zero-length entries are impossible. (the header alone
    // takes two bytes)

    // second pass:
    //   fill up the hash table. then, we'll copy the strings
    //   out in the correct order. (as they are in the hash table)

    // this is the end of the buffer, i.e. the index of the byte
    // one after the last one we wrote.
    let buffer_end = write_pos;
    read_pos = 0;

    while read_pos < buffer_end {
        
        // we have to re-compute num_strokes now,
        // just so we can size all of the arrays properly
        let mut num_strokes = 1;
        loop {
            let byte = buffer[read_pos];
            read_pos += 1;

            if byte == b'"' {
                break;
            }
            else if byte == b'/' {
                num_strokes += 1;
            }
        }

        // hash the string
        let string_start = read_pos;
        
        loop {
            let byte = buffer[read_pos];
            read_pos += 1;

            if byte == 0 {

                let string_length_including_null = read_pos - string_start;
                let strokes_length = num_strokes * 3;
                let offset_header_length = 2;
                let entry_size = offset_header_length + string_length_including_null + strokes_length;

                // (read_pos - 1) since we're ignoring the null byte
                let string = &buffer[string_start .. (read_pos - 1)];
                let index = get_hash_table_index(string, hash_table);
                hash_table[index] += entry_size;
                break;
            }
        }

    }

    // third pass
    // now that we know the length of each of the per-bucket entry
    // arrays, we can compute its offset by doing a prefix sum.

    let mut bucket_array_offset = 0;
    for entry in hash_table.iter_mut() {
        let bucket_array_length = *entry;

        if bucket_array_length > 0 {
            *entry = bucket_array_offset;

            // initialize bucket array with a temporary empty marker
            // (there won't be any empty bucket arrays in the end, hence
            //  why it is temporary)
            definitions[bucket_array_offset] = 0x00;
            definitions[bucket_array_offset + 1] = 0x00;

            bucket_array_offset += bucket_array_length;
        }
        else {
            *entry = usize::max_value();
        }
    }

    // we know now where each entry has to be stored, so we're going
    // to store them.  once we have packed an entry, we can also link
    // to it from the stroke index, so we'll write that in the same
    // step

    // determine layout of the stroke index and write it into the stroke prefix lookup table
    // stroke_subindex_lengths contains the length of each subindex. since the subindices
    // are packed in order, all we have to do is a prefix sum to get the offset for each array.
    // NOTE that doing it this way means that unused subindices simply get a length of 0, but
    // still have a valid offset, which is kind of handy.
    let mut accumulator = 0;
    for (index, length) in stroke_subindex_lengths.iter().enumerate() {
        stroke_prefix_lookup[index] = accumulator;
        accumulator += length;
    }
    stroke_prefix_lookup[256] = accumulator;

    // also, store the current write position for each subarray, so we can push values
    let mut stroke_subindex_write_positions = [0; 256];
    
    let last_stroke_marker = 1 << 23;
    
    read_pos = 0;
    while read_pos < buffer_end {

        // we have to write the string first, so skip the strokes for now
        let strokes_start = read_pos;
        loop {
            let byte = buffer[read_pos];
            read_pos += 1;
            if byte == b'"' {
                break;
            }
        }
        let strokes_end = read_pos;

        // read the string
        let string_start = read_pos;
        loop {
            let byte = buffer[read_pos];
            read_pos += 1;

            if byte == 0 {
                break;
            }
        }
        let string_end_including_terminator = read_pos;

        let string_without_terminator = &buffer[string_start..(string_end_including_terminator - 1)];
        let index = get_hash_table_index(string_without_terminator, hash_table);

        let bucket_offset = hash_table[index];

        let bucket_is_empty = definitions[bucket_offset] == 0 && definitions[bucket_offset + 1] == 0;

        let definition_offset = 
            if bucket_is_empty {
                bucket_offset
            }
            else {
                // iterate and find the next empty slot
                let mut pos = bucket_offset;
                
                loop {
                    let mut header = (definitions[pos] as u16) | ((definitions[pos + 1] as u16) << 8);
                    let is_last = header >> 15 == 1;
                    let offset = (header & ((1 << 15) - 1)) as usize;

                    if is_last {
                        // not anymore, since we're appending an entry!
                        header &= !(1 << 15);
                        definitions[pos] = (header & 0xFF) as u8;
                        definitions[pos + 1] = (header >> 8) as u8;

                        // make pos point to the start of the next entry
                        pos += offset;
                        break;
                    }
                    pos += offset;
                }

                pos
            };

        let mut definition_writer = definition_offset;
        // skip the header for now, we'll come back to that later
        // (once we know how long this entry is)
        definition_writer += 2;

        // copy string
        let string_length_including_terminator = string_end_including_terminator - string_start;
        definitions[definition_writer .. definition_writer + string_length_including_terminator]
            .copy_from_slice(&buffer[string_start .. string_end_including_terminator]);

        definition_writer += string_length_including_terminator;
        
        // read strokes
        let mut stroke_reader = strokes_start;
        let mut is_first_stroke = true;
        while stroke_reader < strokes_end {
            let mut stroke = parse_stroke_fast(&buffer, &mut stroke_reader)?;
            let end_byte = buffer[stroke_reader];
            stroke_reader += 1;

            if end_byte == b'"' {
                stroke |= last_stroke_marker;

                if is_first_stroke {
                    // this is a single-stroke def, add to stroke index
                    let first_byte = (stroke & 0xFF) as u8;
                    // tis works since stroke only uses the lower three bytes
                    let last_bytes = (stroke >> 8) as u16;
                    let subindex_offset = stroke_prefix_lookup[first_byte as usize];
                    let subindex_write_pos = &mut stroke_subindex_write_positions[first_byte as usize];
                    let index_entry_pos = subindex_offset + *subindex_write_pos;
                    stroke_subindices[index_entry_pos] = StrokeIndexEntry {
                        last_two_bytes: last_bytes,
                        // this is the offset into the main entry array
                        definition_offset: definition_offset as usize
                    };
                    *subindex_write_pos += 1;
                }
            }

            // write stroke
            definitions[definition_writer] = (stroke & 0xFF) as u8;
            definitions[definition_writer+1] = ((stroke >> 8) & 0xFF) as u8;
            definitions[definition_writer+2] = (stroke >> 16) as u8;
            definition_writer += 3;

            is_first_stroke = false;
        }

        let definition_length = definition_writer - definition_offset;
        let is_last_entry = 1 << 15;
        let length_mask = (1 << 15) - 1;

        if (definition_length & length_mask) != definition_length {
            return Err(error!(b"I'm sorry, but we can't handle your dictionary.", b"There is nothing wrong with it, except that it has at least one really reaaallly long entry, and this is not something that our internal format can deal with."));
        }

        let header = (definition_length as u16) | is_last_entry;
        definitions[definition_offset] = (header & 0xFF) as u8;
        definitions[definition_offset + 1] = (header >> 8) as u8;
    }

    // fourth pass: sort the subindices, so we can binary search
    // through them. TODO: store them as parallel arrays for stroke
    // bytes + definition pointers, so all reads are aligned and fast?
    // (note though that we are already at two-byte alignment and
    //  last_two_strokes, which is what we sort by, is a two-byte value)
    for i in 0..256 {
        let offset = stroke_prefix_lookup[i];
        let end = stroke_prefix_lookup[i+1];
        // subindex.len() may be 0 if there is no subindex for this start byte
        let subindex = &mut stroke_subindices[offset .. end];
        subindex.sort_unstable_by_key(stroke_index_sortkey);
    }

    return Ok(new_memory_start);
}

fn stroke_index_sortkey(entry: &StrokeIndexEntry) -> u16 {
    entry.last_two_bytes
}

fn get_hash_table_index(string: &[u8], hash_table: &mut [usize]) -> usize {

    let hash = wyhash(string, 1);
    let index = (hash as usize) % hash_table.len();

    return index;
}

// returns the length of this value in the final entry array, in bytes. (NOT in the rewritten string!)
// if is_strokes is true, this is the number of forward slashes plus one, times three. (three bytes per stroke)
// otherwise, it is the number of bytes in the string plus one (for the null terminator).
// if is_strokes is true, it will also ensure that the string is free of escape sequences (and thus quotes)
// and terminate it with a double quote instead of NULL, since that is what parse_strokes needs.
fn rewrite_string<'a>(buffer: &mut[u8], read_pos: &mut usize, write_pos: &mut usize, is_strokes: bool) -> InternalResult<usize> {

    // EXPECTATION: read_pos >= write_pos

    // we're just going to turn the json strings
    // into simple null-terminated strings. normally,
    // I'd prefer length-prefixed strings, but I think
    // this is just the simplest option here.

    // I think it's simpler to just compute both and then
    // only return the one we need. It saves some ifs.
    let mut final_size_guess_string = 1;
    let mut final_size_guess_strokes = 3;

    expect_char(&buffer, read_pos, b'"')?;
    // INVARIANT: read_pos >= write_pos + 1

    let mut escape_next = false;

    loop {
        let byte = *buffer.get(*read_pos).ok_or(error!(PARSER_ERROR, b"Parser error: data ended in the middle of string"))?;
        *read_pos += 1;

        if escape_next {
            if byte == b'"' || byte == b'\\' {
                buffer[*write_pos] = byte;
                *write_pos += 1;
                final_size_guess_string += 1;
            }
            else {
                // copy the escape sequence unchanged
                buffer[*write_pos] = b'\\';
                buffer[*write_pos+1] = byte;
                *write_pos += 2;
                final_size_guess_string += 2;
            }
            escape_next = false;
        }

        else {
            if byte == b'"' {
                break;
            }
            else if byte == b'\\' {
                if is_strokes {
                    // the stroke parser can't handle those, so we have to make sure
                    // they won't be in there.
                    return Err(error!(PARSER_ERROR, b"Parser error: escape sequence found in stroke definition"));
                }
                escape_next = true;
            }
            else {
                buffer[*write_pos] = byte;
                *write_pos += 1;
                final_size_guess_string += 1;
            }
        }

        if byte == b'/' {
            final_size_guess_strokes += 3;
        }
    }
    // INVARIANT: read_pos >= write_pos + 2

    if is_strokes {
        buffer[*write_pos] = b'"';
    }
    else {
        buffer[*write_pos] = 0;
    }
    *write_pos += 1;
    // INVARIANT: read_pos >= write_pos + 1

    if is_strokes {
        return Ok(final_size_guess_strokes);
    }
    else {
        return Ok(final_size_guess_string);
    }
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
// TODO: what about zero-length strokes or other malformed input?
fn parse_stroke_fast(buffer: &[u8], pos: &mut usize) -> InternalResult<u32> {

    let mut state = 0;
    let mut stroke = 0;
    while (state & (1 << 7)) == 0 {
        let byte = *buffer.get(*pos).ok_or(
            error!(PARSER_ERROR, b"Parser error: Reached end of data while reading stroke (in parse_stroke_fast)"))?;
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

    // move back to the stop symbol, so the calling code
    // can handle that.
    *pos -= 1;
    return Ok(stroke);
}

#[link(wasm_import_module = "env")]
extern { fn yield_result(string_offset: u32, string_length: u32, stroke_offset: u32, stroke_length: u32); }

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
    let hashmap_size = offset_info.stroke_index - offset_info.hash_table;
    let hashmap = core::slice::from_raw_parts(
        (offset_info.hash_table + data_offset) as *const usize,
        hashmap_size / size_of::<usize>()
    );
    let stroke_prefix_lookup = core::slice::from_raw_parts(
        (offset_info.stroke_index + data_offset) as *const usize,
        257
    );
    let stroke_prefix_lookup_size = 257 * size_of::<usize>();

    let stroke_subindices_offset = offset_info.stroke_index + stroke_prefix_lookup_size;
    let stroke_subindices_size = offset_info.definitions - stroke_subindices_offset;
    
    let stroke_subindices = core::slice::from_raw_parts(
        (stroke_subindices_offset + data_offset) as *const StrokeIndexEntry,
        stroke_subindices_size / size_of::<StrokeIndexEntry>()
    );

    let definitions_size = offset_info.end - offset_info.definitions;
    let definitions = core::slice::from_raw_parts(
        (offset_info.definitions + data_offset) as *const u8,
        definitions_size
    );

    if find_stroke == 0 {
        let query = core::slice::from_raw_parts(
            offset as *const u8,
            length as usize
        );
        query_internal(query, hashmap, definitions).unwrap_or_else(log_err_internal);
    }
    else {
        let query_stroke = offset;
        find_stroke_internal(query_stroke, stroke_prefix_lookup, stroke_subindices, definitions).unwrap_or_else(log_err_internal);
    }
}

fn query_internal(query: &[u8], hashmap: &[usize], definitions: &[u8]) -> InternalResult<()> {

    let hash = wyhash(query, 1);
    let index = (hash as usize) % hashmap.len();

    let bucket_offset = hashmap[index];
    if bucket_offset == usize::max_value() {
        // no results
        return Ok(());
    }

    for definition in BucketEntryIterator::new(&definitions[bucket_offset..]) {
        
        let is_match = (definition.len() > query.len())
            && (definition[query.len()] == 0)
            && (&definition[0 .. query.len()] == query);

        if is_match {

            let string = &definition[0 .. query.len()];

            let strokes_start = query.len() + 1;
            let mut stroke_pos = strokes_start;

            // read strokes
            loop {

                let stroke1 = definition[stroke_pos] as u32;
                let stroke2 = definition[stroke_pos+1] as u32;
                let stroke3 = definition[stroke_pos+2] as u32;
                stroke_pos += 3;

                let stroke = stroke1 | (stroke2 << 8) | (stroke3 << 16);
                if (stroke >> 23) == 1 {
                    break;
                }
            }
            let strokes_end = stroke_pos;
            let strokes = &definition[strokes_start..strokes_end];

            unsafe {
                yield_result(string.as_ptr() as u32, string.len() as u32, strokes.as_ptr() as u32, (strokes_end - strokes_start) as u32);
            }
        }
    }

    return Ok(());
}

fn find_stroke_internal(mut query_stroke: u32, stroke_prefix_lookup: &[usize], stroke_subindices: &[StrokeIndexEntry], definitions: &[u8]) -> InternalResult<()> {

    // currently, we're storing the strokes with the last stroke marker,
    // so we'll have to add that in
    query_stroke |= 1 << 23;

    let first_byte = query_stroke & 0xFF;
    let last_two_bytes = query_stroke >> 8;

    let subindex_start = stroke_prefix_lookup[first_byte as usize];
    let subindex_end = stroke_prefix_lookup[first_byte as usize + 1];
    let subindex = &stroke_subindices[subindex_start..subindex_end];

    let result = subindex.binary_search_by_key(&(last_two_bytes as u16), stroke_index_sortkey);

    if let Ok(index) = result {
        let entry = &subindex[index];
        // skip the header
        let definition_offset = entry.definition_offset + 2;

        let string_start = definition_offset;
        let string_end = *(&definitions[string_start..].iter().position(|&byte| byte == 0).unwrap()) + string_start;
        let string = &definitions[string_start..string_end];
        let strokes_start = string_end + 1;
        let strokes_end = *(&definitions[strokes_start..].chunks_exact(3).position(|stroke| (stroke[2] >> 7) == 1).unwrap()) * 3 + 3 + strokes_start;
        let strokes = &definitions[strokes_start..strokes_end];

        unsafe {
            yield_result(string.as_ptr() as u32, string.len() as u32, strokes.as_ptr() as u32, (strokes_end - strokes_start) as u32);
        }
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
        pos = 0;
    }
}
