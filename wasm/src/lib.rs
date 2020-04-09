#![no_std]

use core::convert::TryFrom;

#[link(wasm_import_module = "env")]
extern { fn logErr(offset: u32, length: u32); }

fn log_err_internal(string: &[u8]) {
    unsafe {
        logErr(string.as_ptr() as u32, string.len() as u32);
    }
}

#[cfg(all(target_arch = "wasm32", not(test)))]
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

fn handle_loader_error(message: &[u8]) -> usize {
    log_err_internal(message);
    panic!();
}

#[no_mangle]
pub unsafe extern fn load_json(offset: u32, length: u32) -> u32 {
    let buffer = core::slice::from_raw_parts_mut(
        offset as *mut u8,
        length as usize
    );
    return load_json_internal(buffer).unwrap_or_else(handle_loader_error) as u32;
}

// loads a json dictionary into two parallel packed arrays,
// one for the strings (translations) and one for the strokes
// in 4-byte format. the string array will replace the initial
// json buffer, while the stroke array will go into newly allocated
// memory. the function returns a pointer to the start of this new
// memory, which has two u32s that give the length of the string and
// stroke array (in bytes), respectively, followed by the packed
// stroke array. the lengths are not strictly necessary, but they
// will save the js code some work when it picks the results out of
// our memory.
pub fn load_json_internal(mut buffer: &mut [u8]) -> Result<usize, &[u8]> {
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
    // and second, determining the sizes of the required packed array,
    // so we know how much to allocate.
    //
    // note: this is not a full-fledged json parser. it is specifically
    // designed for reading plover dicionaries, and it will fail when
    // passed otherwise valid json that does not fit this schema.

    let mut read_pos = 0;
    let mut write_pos = 0;

    let mut string_array_length = 0;
    let mut stroke_array_length = 0;

    // unfortunately, we can't use an iterator for all this since we
    // need to be able to write the slice while doing this.

    // INVARIANT: read_pos >= write_pos

    skip_whitespace(buffer, &mut read_pos).or(Err(b"Parser error: no data found".as_ref()))?;
    expect_char(buffer, &mut read_pos, b'{')?;
    
    // INVARIANT: read_pos >= write_pos + 1

    loop {

        // read key
        skip_whitespace(buffer, &mut read_pos)?;
        stroke_array_length += rewrite_string(&mut buffer, &mut read_pos, &mut write_pos, true)?;
        // INVARIANT: read_pos >= write_pos + 1

        skip_whitespace(buffer, &mut read_pos)?;
        expect_char(buffer, &mut read_pos, b':')?;
        skip_whitespace(buffer, &mut read_pos)?;

        // read value
        string_array_length += rewrite_string(&mut buffer, &mut read_pos, &mut write_pos, false)?;
        // INVARIANT: read_pos >= write_pos + 1
        // (note: we could get better bounds in practice, since each
        //  string read actually increases the space by one. but we
        //  don't need these, so this is how it's going to stay.)

        skip_whitespace(buffer, &mut read_pos)?;

        let byte = buffer.get(read_pos).ok_or(b"Parser error: data incomplete".as_ref())?;
        if *byte == b'}' {
            // reached file end
            read_pos += 1;
            break;
        }
        expect_char(buffer, &mut read_pos, b',')?;
    }

    // first pass is done.
    // second pass:
    //   copy the strings and strokes into their respective arrays. we will reuse
    //   the json allocation for the strings (since they're guaranteed to shrink,
    //   unlike the strokes), and allocate a new one for the strokes.

    // I'm including a little bit extra, so we can store the lengths of the
    // stroke and string array, so the js can extract them easily.
    // also, leave a bit more space in case we have to align.
    // (we shouldn't, but see below.)
    let u32_align = core::mem::align_of::<u32>();
    
    let memory_needed = stroke_array_length + 8 + (u32_align - 1);

    let wasm_page_size = 65536;
    // this is just a rounding-up division for ints.
    let number_of_new_pages = (memory_needed - 1) / wasm_page_size + 1;

    // allocate new memory
    let previous_mem_size_pages = core::arch::wasm32::memory_grow(0, number_of_new_pages);
    let new_memory_start = previous_mem_size_pages * wasm_page_size;

    let stroke_array;
    let new_data_start;

    unsafe {
        // turn everything into pointers / slices;
        // first, I am going to align new_memory_start. I am going to do this
        // while new_memory_start is a u8 pointer, since align_offset only
        // works in multiples of the type size (which I don't understand, tbh.)
        //
        // note that I *think* it should already be aligned, since we're at
        // the start of a fresh page, but I don't really want to mess around
        // with this and I haven't seen explicit guarantees anywhere.
        //
        // I am doing all of this in a single block, so that all of the
        // intermediate values will go out of scope and get dropped
        // cleanly when we are done here.

        let start_ptr_u8 = new_memory_start as *mut u8;
        let offset = start_ptr_u8.align_offset(u32_align);
        new_data_start = new_memory_start + offset;

        if offset >= u32_align {
            // this should not be happening
            panic!("align_offset has returned a value that is too big.");
        }

        // write the two lengths
        let string_array_length_ptr = start_ptr_u8.add(offset) as *mut u32;
        let stroke_array_length_ptr = string_array_length_ptr.add(1);

        *string_array_length_ptr = string_array_length as u32;
        *stroke_array_length_ptr = stroke_array_length as u32;

        stroke_array = core::slice::from_raw_parts_mut(
            string_array_length_ptr.add(2),
            stroke_array_length / 4
        );

        // whew
    }

    // second pass!
    // this is the end of the buffer, i.e. the index of the byte
    // one after the last one we wrote.
    let buffer_end = write_pos;
    read_pos = 0;
    let mut string_write_pos = 0;
    let mut stroke_write_pos = 0;

    while read_pos < buffer_end {
        
        // step 1:
        // read the strokes
        let mut is_first_stroke = 1 << 31;
        loop {

            let stroke = parse_stroke_fast(buffer, &mut read_pos) | is_first_stroke;
            is_first_stroke = 0;

            stroke_array[stroke_write_pos] = stroke;
            stroke_write_pos += 1;

            if buffer[read_pos] == b'"' {
                // stop reading strokes
                read_pos += 1;
                break;
            }

            // otherwise, we'll get another stroke
            read_pos += 1;
        }

        // step 2:
        // copy the string
        // we're going to leave two bytes of space at the beginning, so we can
        // go back and write the length of the string once we know it.
        let length_field = string_write_pos;
        let mut length: usize = 0;
        string_write_pos += 2;
        
        loop {
            let byte = buffer[read_pos];
            read_pos += 1;

            if byte == 0 {
                break;
            }

            buffer[string_write_pos] = byte;
            string_write_pos += 1;
            length += 1;
        }

        let length_u16 = u16::try_from(length).or(Err("Parser error: a string in your dictionary is longer than 64kB, which is very big and kind of surprising. I am sorry, but we cannot handle this right now, and probably never will.")).unwrap();

        // write the length in little endian
        buffer[length_field] = (length_u16 & 0xFF) as u8;
        buffer[length_field+1] = (length_u16 >> 8) as u8;
    }

    // sanity check
    assert_eq!(string_write_pos, string_array_length);
    log_err_internal(b"debug: in sanity check");

    {
        let mut a = *b"string array length:          / string write pos:         ";
        let mut num = string_array_length;
        let mut num2 = string_write_pos;
        let mut idx = a.len();
        while idx > a.len() - 8 {
            idx -= 1;
            a[idx] = (num2 % 10) as u8 + b'0';
            a[idx-28] = (num % 10) as u8 + b'0';
            num /= 10;
            num2 /= 10;
        }

        log_err_internal(&a);
    }
    assert_eq!(stroke_write_pos*4, stroke_array_length);

    log_err_internal(b"debug: done");

    return Ok(new_data_start);
}

// returns the length of this value in the final array, in bytes.
// if is_strokes is true, this is the number of forward slashes plus one, times four. (four bytes per stroke)
// otherwise, it is the number of bytes in the string plus two (for the length).
// if is_strokes is true, it will also ensure that the string is free of escape sequences (and thus quotes)
// and terminate it with a double quote instead of NULL, since that is what parse_strokes needs.
fn rewrite_string<'a>(buffer: &mut[u8], read_pos: &mut usize, write_pos: &mut usize, is_strokes: bool) -> Result<usize, &'a [u8]> {

    // EXPECTATION: read_pos >= write_pos

    // we're just going to turn the json strings
    // into simple null-terminated strings. normally,
    // I'd prefer length-prefixed strings, but I think
    // this is just the simplest option here.

    // I think it's simpler to just compute both and then
    // only return the one we need. It saves some ifs.
    let mut final_size_guess_string = 2;
    let mut final_size_guess_strokes = 4;

    expect_char(&buffer, read_pos, b'"')?;
    // INVARIANT: read_pos >= write_pos + 1

    let mut escape_next = false;

    loop {
        let byte = *buffer.get(*read_pos).ok_or(b"Parser error: data ended in the middle of string".as_ref())?;
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
                    return Err(b"Parser error: escape sequence found in stroke definition");
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
            final_size_guess_strokes += 4;
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

// TODO: figure out the lifetime for the error string here
fn skip_whitespace<'a>(buffer: &[u8], pos: &mut usize) -> Result<(), &'a [u8]> {
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
    return Err(b"Parser error: data incomplete");
}

fn expect_char<'a>(buffer: &[u8], pos: &mut usize, expected: u8) -> Result<(), &'a [u8]> {
    if let Some(byte) = buffer.get(*pos) {
        if *byte == expected {
            *pos += 1;
            return Ok(());
        }
        else {
            // todo: maybe find a more elegant way to statically determine
            // the interpolation positions?
            let message = b"Parser error: expected '$', but got '$' (at char     )";
            // copy this onto the stack (i think) so we can format it.
            let mut formatted_message = *message;
            formatted_message[24] = expected;
            formatted_message[37] = *byte;
            formatted_message[49] = (*pos/1000) as u8 + b'0';
            formatted_message[50] = ((*pos/100) % 10) as u8 + b'0';
            formatted_message[51] = ((*pos/10) % 10) as u8 + b'0';
            formatted_message[52] = ((*pos) % 10) as u8 + b'0';
            log_err_internal(&formatted_message);
            return Err(message);
        }
    }
    else {
        let message = b"Parser error: expected '$', but hit the end of data";
        unsafe {
            *(message[24] as *mut u8) = expected;
        }
        return Err(message);
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
    NUMBER | (1 << 9), // 0
    NUMBER | (1 << 1), // 1
    NUMBER | (1 << 2), // 2
    NUMBER | (1 << 4), // 3
    NUMBER | (1 << 6), // 4
    NUMBER | (1 << 8), // 5
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
fn parse_stroke_fast(buffer: &[u8], pos: &mut usize) -> u32 {

    let mut state = 0;
    let mut stroke = 0;
    while (state & (1 << 7)) == 0 {
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

    // move back before the stop symbol, so the calling code
    // can handle that.
    *pos -= 1;
    return stroke;
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
