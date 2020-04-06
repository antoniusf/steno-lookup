#![no_std]

#[link(wasm_import_module = "env")]
extern { fn logErr(offset: u32, length: u32); }

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

#[no_mangle]
pub unsafe extern fn load_json(offset: u32, length: u32) -> u32{
    let string_data = core::slice::from_raw_parts(
        offset as *const u8,
        length as usize
    );

    // here's the plan: we are going to read the file in, piece-by-piece,
    // convert it to our binary format, and then write that back as we go,
    // so we never have to allocate more memory than the initial dictionarys size.
    // this is kind of important, since wasm can't give memory back.
    //
    // on a normal (non-pathologic) dictionary, the binaray format should save
    // a lot of space, so we'll have some left over at the end.
    //
    // this is not a general json parser. it is very specifically made for reading
    // plovers json dictionaries, which consist of a single object with string keys
    // and string values. furthermore, the keys should be pure ascii, so we don't have
    // to decode them. values should be utf-8, so we can just copy them over. (we don't
    // have to parse those, unlike the keys)

    let mut read_pos = 0;
    let mut write_pos = 0;

    // INVARIANT: read_pos >= write_pos

    // data format:
    // values: length-prefixed strings (u16), so we can iterate efficiently.
    //         u16 should be sufficient, no one is going to have a translation longer than 65.000 characters.
    //         (the longest one in stan's dictionary is a reporter's certificate, which is really long
    //          and has 478 chars. i think its fair to say that a u16 is enough.
    // keys: (strokes) packed u32s, one for each stroke. first stroke of each def is marked on the msb,
    //        no explicit length info necessary.

    // so, here's the problem.
    // since we work in-place, the new format must be guaranteed to be smaller
    // than the old one. on first glance, this is easy: the json format includes
    // a lot of extra characters, and most strokes get a lot shorter when we store
    // them in the binary format. however, the binary stroke format needs a constant
    // 4 bytes per stroke, but in json, a single-key stroke only needs 2 (the key,
    // and the following other character ('"' or '/'). in most cases, this is not a
    // problem because (a) we do have some extra characters in each line and (b)
    // there are likely other strokes that will become shorter and thus make space
    // for these. however, at the very beginning of a file, other strokes may not
    // have had the chance to create this extra space and we may not be able to
    // expand the stroke.
    //
    // to handle this, we use two passes: the first pass will only parse strokes that fit,
    // and mark unexpanded strokes with a special marker, while also keeping track of how
    // many extra bytes are needed. then, at the end, we do a second pass, this time going
    // from back to front, moving everything back by this number of bytes. when we encounter
    // an unexpanded stroke, we expand it into the extra space and decrease the number of
    // bytes that are still free.
    //
    // to make this second back-to-front pass possible, we have to store data the wrong way
    // around: strings have to be length-suffixed, not prefixed, and the special marker has
    // to come *after* the unparsed data, not before. strokes don't have to be stored differently,
    // since the first-stroke-marker works in reverse as well.
    let mut extra_bytes_needed = 0;

    // TODO: make this a struct with impls, so we can stop passing all these values around?
    skip_whitespace(buffer, &mut read_pos);
    expect_char(buffer, &mut read_pos, b'{');
    // INVARIANT: read_pos >= write_pos + 1
    while true {
        // INVARIANT: read_pos >= write_pos + 1 (dominated by loop precondition)
        skip_whitespace(buffer, &mut read_pos);
        expect_char(buffer, &mut read_pos, b'"');
        // INVARIANT: read_pos >= write_pos + 2
        
        // read the strokes
        let mut is_first_stroke = (1 << 31);
        while true {
            // INVARIANT: read_pos >= write_pos (dominated by loop end condition)

            let stroke = parse_stroke_fast(buffer, &mut read_pos) | is_first_stroke;
            is_first_stroke = 0;

            // read this now so we have a bit more space for writing
            // (this will come in handy later)
            let end_char = buffer[read_pos];
            read_pos += 1;

            // INVARIANT: read_pos >= write_pos + 1

            // we have to write 4 bytes, check if there's enough space
            if (write_pos + 4 <= read_pos) {
                // we're just doing this manually, alignment isn't guaranteed anyways
                // this'll be fixed when we reformat everything later
                // little endian, write the highest byte last
                buffer[write_pos] = stroke & 0xFF;
                buffer[write_pos+1] = (stroke >> 8) & 0xFF;
                buffer[write_pos+2] = (stroke >> 16) & 0xFF;
                buffer[write_pos+3] = (stroke >> 24) & 0xFF;
                write_pos += 4;
                // INVARIANT: read_pos >= write_pos
            }
            else {
                // uh-oh, there's not enough space
                // INVARIANT: read_pos >= write_pos + 1
                extra_bytes_needed += 4 - (read_pos - write_pos);

                // this is safe as u8, (a) because strokes shouldn't
                // be longer than 256 bytes anyways and (b) if the stroke
                // was this long, we'd have no problem fitting in the parsed
                // version. (whatever the parser did with all those extra bytes
                // we don't care)
                let unparsed_length: u8 = 0;

                // copy the next stroke unparsed
                while true {
                    // INVARIANT: read_pos >= write_pos + 1
                    let byte = buffer[read_pos];
                    read_pos += 1;
                    // INVARIANT: read_pos >= write_pos + 2
                    if (byte == b'"' || byte == b'/') {
                        break;
                    }
                    buffer[write_pos] = byte;
                    write_pos += 1;
                    // INVARIANT: read_pos >= write_pos + 1
                    unparsed_length += 1;
                }
                // INVARIANT: read_pos >= write_pos + 2

                // we also have to store the length of the unparsed data,
                // so that the 2nd pass knows how much to read
                // also, I'm going to abuse the msb of this to store
                // if this was the first stroke
                buffer[write_pos] = unparsed_length | ((stroke >> 31) << 7) as u8;
                write_pos += 1;
                // INVARIANT: read_pos >= write_pos + 1

                // mark the unparsed stroke
                // if a normal stroke ended here, this would be the info byte
                // which would either be 0 or (1 << 7). So this way, we can distinguish.
                buffer[write_pos] = 0xFF;
                write_pos += 1;
                // INVARIANT: read_pos >= write_pos
            }

            // INVARIANT: read_pos >= write_pos

            if end_char == b'"' {
                // stop reading strokes
                // INVARIANT: read_pos >= write_pos
                break;
            }

            // otherwise, we'll get another stroke
            // TODO: check?: end_char should be '/'
            assert!(byte == b'/');
            // INVARIANT: read_pos >= write_pos
        }
        // INVARIANT: read_pos >= write_pos
        skip_whitespace(buffer, &mut read_pos);
        expect_char(buffer, &mut read_pos, b':');
        // INVARIANT: read_pos >= write_pos + 1
        skip_whitespace(buffer, &mut read_pos);
        expect_char(buffer, &mut read_pos, b'"');
        // INVARIANT: read_pos >= write_pos + 2

        // copy string
        let mut escape_next = false;
        let mut length: u16 = 0;
        while true {
            // INVARIANT: read_pos >= write_pos + 2
            let byte = buffer[read_pos];
            read_pos += 1;
            // INVARIANT: read_pos >= write_pos + 3

            if (!escape_next) {
                if (byte == b'"') {
                    break;
                }
                else if (byte == b'\\') {
                    escape_next = true;
                }
                // no, we're not reading other escapes
            }
            else {
                escape_next = false;
            }

            buffer[write_pos] = byte;
            write_pos += 1;
            // INVARIANT: read_pos >= write_pos + 2
            length += 1;
        }
        // INVARIANT: read_pos >= write_pos + 3
        buffer[write_pos] = length & 0xFF;
        buffer[write_pos + 1] = (length >> 8) & 0xFF;
        write_pos += 2;
        // INVARIANT: read_pos >= write_pos + 1

        // string copied!
        skip_whitespace(buffer, &mut read_pos);
        let byte = buffer[read_pos];
        read_pos += 1;
        // INVARIANT: read_pos >= write_pos + 2

        if byte == b'}' {
            // we're done!! \o/
            break;
        }
        // otherwise, byte should be == b','
        assert!(byte == b',');
        // INVARIANT: read_pos >= write_pos + 2
    }

    // first pass done, yay!
    // second pass: read from back to front, making space for unparsed strokes,
    // rewriting strings to length-prefix form since that's what we need in the end

    if (read_pos - write_pos) > extra_bytes {
        // we'd have to allocate more, I don't want to do that yet
        // TODO: log error (doesn't work via panic since we can't format args)
        panic!("the dictionary is very weird and would be longer in binary than in json. i cant handle this, sorry");
    }

    // write_pos is on the byte one after the last one we wrote.
    read_pos = write_pos - 1;
    write_pos = read_pos + extra_bytes_needed;
    while true {
        // read string length
        let length = buffer[read_pos - 1] | (buffer[read_pos] << 8);
        read_pos -= 2;

        // copy string
        for i in 0..length {
            buffer[write_pos] = buffer[read_pos];
            write_pos -= 1;
            read_pos -= 1;
        }

        // write string length
        buffer[write_pos] = length >> 8;
        buffer[write_pos-1] = length & 0xFF;
        write_pos -= 2;

        // copy strokes
        while true {
            // do we have to parse this stroke?
            if buffer[read_pos] == 0xFF {
                read_pos -= 1;
                let length = buffer[read_pos] & 127;
                let is_first_stroke = buffer[read_pos] >> 7;
                read_pos -= 1;
                // mark the end of the stroke in a way that parse_stroke can understand
                read_pos += 1;
                // read_pos is now on the first byte after the stroke
                buffer[read_pos] = b'"';
                read_pos -= length;

                let stroke = parse_stroke_fast(buffer, &mut read_pos) | (is_first_stroke << 31);
                // read_pos is now on the first byte after the stroke, *again*.
                read_pos -= length;
                // read_pos is now on the first byte *of* the stroke
                read_pos -= 1;
                // read_pos is now on the byte *before* the stroke, so we can continue
                // reading normally.

                buffer[write_pos] = stroke >> 24;
                buffer[write_pos-1] = (stroke >> 16) & 0xFF;
                buffer[write_pos-2] = (stroke >> 8) & 0xFF;
                buffer[write_pos-3] = stroke & 0xFF;
                write_pos -= 4;

                // note that we're writing more bytes than we're reading here.
                // if extra_bytes_needed was computed correctly, we should
                // eventually be in a situation where the write pointer
                // has caught up to the read pointer.

                if (is_first_stroke != 0) {
                    // we're done with the strokes!
                    break;
                }
            }
            else {
                // we can just copy this stroke normally
                buffer[write_pos] = buffer[read_pos];
                buffer[write_pos-1] = buffer[read_pos-1];
                buffer[write_pos-2] = buffer[read_pos-2];
                buffer[write_pos-3] = buffer[read_pos-3];

                // since we stored this little-endian, the msb of the stroke
                // is the msb of the last byte.
                let is_first_stroke = buffer[read_pos] >> 7;
                read_pos -= 4;
                write_pos -= 4;

                if (is_first_stroke != 0) {
                    break;
                }
            }
        }
    }
}

fn skip_whitespace(buffer: &[u8], pos: &mut usize) {
    while true {
        let byte = buffer[*pos];
        if (byte != b' '
            & byte != b'\t'
            & byte != b'\r'
            & byte != b'\n'
        ) {
            // character is non-whitespace. don't advance position,
            // return and let the caller continue.
            return;
        }
        *pos += 1;
    }
}

fn expect_char(buffer: &[u8], pos: &mut usize, expected: u8) {
    let byte = buffer[*pos];
    if (byte == expected) {
        *pos += 1
    }
    // TODO: error handling – what happens when this is not the right byte?
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
