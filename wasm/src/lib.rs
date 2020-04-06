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

    // TODO: make this a struct with impls, so we can stop passing all these values around?
    skip_whitespace(buffer, &mut read_pos);
    expect_char(buffer, &mut read_pos, b'{');
    // INVARIANT: read_pos >= write_pos + 1
    while true {
        // INVARIANT: read_pos >= write_pos + 1
        // NOTE: check that loop exit maintains this too!
        skip_whitespace(buffer, &mut read_pos);
        expect_char(buffer, &mut read_pos, b'"');
        // INVARIANT: read_pos >= write_pos + 2
        
        // read the strokes
        let mut is_first_stroke = (1 << 7);
        while true {
            // INVARIANT: read_pos >= write_pos + 1 (dominated by loop end condition)

            // stroke goes into the three higher bytes. we're writing this out in little
            // endian, and we need the info byte to go first.
            let stroke = (parse_stroke_fast(buffer, &mut read_pos) << 8) | is_first_stroke;
            is_first_stroke = 0;

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

                // mark the following as unparsed. 
                // if a normal stroke started here, this would be the info byte
                // which would either be 0 or (1 << 7). So this way, we can distinguish.
                // also, the 0xFF byte is neither part of ASCII, nor of utf-8, so this
                // is really the only place it should appear in the final document, except
                // inside another stroke.
                buffer[write_pos] = stroke & 0xFF;
                write_pos += 1;
                // INVARIANT: read_pos >= write_pos

                // copy the rest of the stroke verbatim
                while true {
                    byte = buffer[read_pos];
                }
            }

            // INVARIANT: read_pos >= write_pos

            if buffer[read_pos] == b'"' {
                // stop reading strokes
                read_pos += 1;
                // INVARIANT: read_pos >= write_pos + 1
                break;
            }

            // otherwise, we'll get another stroke
            expect_char(buffer, &mut read_pos, b'/');
            // INVARIANT: read_pos >= write_pos + 1
        }

        // INVARIANT: read_pos >= write_pos + 1
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
