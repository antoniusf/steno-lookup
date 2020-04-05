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
    let mut counter = 0;
    for byte in string_data {
        counter += *byte as u32;
    }
    return counter;
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
