/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

// vowels have unique associations to left/right bank,
// so we're omitting the dashes. (the actual reason is
// that it simplifies the conversion code :D)
// - it also uniquely identifies hyphen keys as those without a dash!
export const steno_keys = [
    "#",
    "S-",
    "T-", "K-",
    "P-", "W-",
    "H-", "R-",
    "A", "O",
    "*",
    "E", "U",
    "-F", "-R",
    "-P", "-B",
    "-L", "-G",
    "-T", "-S",
    "-D", "-Z"
];

const number_keys = {
    "0": "O-",
    "1": "S-",
    "2": "T-",
    "3": "P-",
    "4": "H-",
    "5": "A-",
    "6": "-F",
    "7": "-P",
    "8": "-L",
    "9": "-T"
}

// convert a list of steno keys to a binary stroke number (int32)
// this has a direct correspondence to keys, so the key with index 0
// gets represented by bit 0 (i.e. the lsb).
export function keylistToStroke(list) {
    let stroke = 0;
    let keybit = 1;
    for (const key of steno_keys) {
	if (list.includes(key)) {
	    stroke |= keybit;
	}
	keybit <<= 1;
    }
    return stroke;
}

export function strokeToText(stroke) {
    let text = "";
    let needs_separator = true;

    // since the first key is stored in the lsb, we can simply shift
    // the stroke right by one each time and check bit 0 each time,
    // to go through all keys in the right order.
    for (const key of steno_keys) {
	if (stroke & 1) {
	    // startswith is not needed here, since key is guaranteed
	    // to have at least one character, so this is safe to do
	    if ((key[0] == "-") && needs_separator) { 
		text += "-";
		needs_separator = false;
	    }

	    text += key.replace("-", "")

	    if (!key.includes("-") && key != "#") {
		// this is an implicit hyphen key
		needs_separator = false;
	    }
	}
	stroke >>= 1;
    }
    return text;
}

export function packedStrokesToText(strokes) {
    let texts = [];
    for (let i = 0; i < strokes.length; i += 3) {
	let stroke = strokes[i] | (strokes[i+1] << 8) | (strokes[i+2] << 16);
	texts.push(strokeToText(stroke));
    }
    return texts.join("/");
}

// TODO: standard replacements N- => TPH- etc.
export function textToStroke(text) {
    let next_consonant_is_right_bank = false;
    let stroke = 0;
    
    for (const character of text) {
	if (character == "-") {
	    next_consonant_is_right_bank = true;
	    continue;
	}
	// this will work directly for A, O, *, E, U – the implicit hyphen keys
	let key_index = steno_keys.indexOf(character);
	
	// number bar
	if (key_index == 0) {
	}
	else if (key_index > 0) {
	    // this was an implicit hyphen key
	    next_consonant_is_right_bank = true;
	}
	else {
	    if (next_consonant_is_right_bank) {
		key_index = steno_keys.indexOf("-" + character);
	    }
	    else {
		// we're still in the left bank
		key_index = steno_keys.indexOf(character + "-");
	    }
	    if (key_index < 0) {
		const translated_key = number_keys[character];
		if (translated_key) {
		    // this sets the bit at index 0, ie the number bar.
		    // a bit confusing, but it should work.
		    stroke |= 1;
		    key_index = steno_keys.indexOf(translated_key);
		}
		else {
		    console.log("invalid key!"); //TODO: throw something
		    continue;
		}
	    }
	}

	// add this key to the stroke by its key index
	stroke |= 1 << key_index;
    }

    return stroke;
}

export function strokeListToPackedStrokes(stroke_list) {
    let strokes = [];
    for (const stroke of stroke_list) {
        strokes.push(stroke & 0xFF);
        strokes.push((stroke >> 8) & 0xFF);
        strokes.push((stroke >> 16) & 0xFF);
    }

    return new Uint8Array(strokes);
}

export function strokeToKeylist (stroke) {
    let keylist = [];
    for (const key of steno_keys) {
	// same principle as in strokeToText
	if (stroke & 1) {
	    keylist.push(key);
	}
	stroke >>= 1;
    }
    return keylist;
}

export function strokeToKeydict (stroke) {
    let keydict = {};
    for (const key of steno_keys) {
	keydict[key] = stroke & 1;
	stroke >>= 1;
    }
    return keydict;
}

export function formatFilesize (size) {

    if (size < 0) {
        throw RangeError("File size must be larger than 0!");
    }

    // first, choose prefix
    // (get order of magnitude in base 1000)
    let order_of_magnitude = Math.log10(size) / 3;

    // i'm adding a small constant, so that values very
    // close to the next multiple of thousand get rounded up
    // (displaying 1MB instead of 995kB)
    // again, i am dividing the result from log10 by 3,
    // since we're working in base 1000
    const rounding_correction = Math.log10(1000/995) / 3;
    order_of_magnitude += rounding_correction;
    
    // i'm also adding another constant because i want
    // 100 kB (or 99.5kB, because of our correction) to get shown as 0.1 MB.
    // this means i am adding 1 / 3., which is basically
    // the same as if i'd multiplied the original value by 10
    // before doing anything else
    order_of_magnitude += 1/3;

    // lastly, i'm rounding down to get our final classification.
    // everything below 99.5 goes to 0, etc.
    // (yes i know that we can't have half bytes, so it should really
    //  say 99, but for the higher orders it's going to be
    //  99.5 * 1000^n gets rounded to n
    order_of_magnitude = Math.floor(order_of_magnitude);

    // now we can actually compute the rounded mantissa!
    // (that's the part before the prefix)
    // round to one decimal digit
    const mantissa = Math.round(size / (1000**order_of_magnitude) * 10) / 10;

    const prefixes = ["", "k", "M", "G", "T", "P", "E"]
    if (order_of_magnitude < prefixes.length) {
        // \u2009 is a narrow no-break space
        return mantissa + "\u202F" + prefixes[order_of_magnitude] + "B";
    } else {
        // just in case someone passes in an unrealistically large value
        return mantissa + "e" + (3 * order_of_magnitude) + "\u202FB";
    }
}
