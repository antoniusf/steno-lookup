import json
import math
import numpy as np

# terms:
# 
#     m: size of probability distribution table
#     b: base of how many bits are written out at a time (i think i want 256, so one byte)
#     l: beginning of normalization range, we want this to be as large as possible
#        (for precision) (more specifically, m/l should be small)
# 
# constraints:
# 
#     m should divide l (b-uniqueness)
#     b should also divide l (this makes things simpler, since we always transfer the same number of bits)
# 
# i think i'll use 64 bit ints as state, because why not
# 
# make l as large as possible:
# 
#     b*l - 1 is upper end of range, ie INT_MAX
#     b*l - 1 = 2**64 - 1
# 
#     <=> b*l = 2**64
#     <=> l = 2**64 / b = 2**64 / 2**8 = 2**48
# 
# how big should m be?
# 
# -> big, so we can map probabilities accurately
# -> not too big, so m/l stays small
# 
# there are 256 symbols, so if m is 256, everyone has the same probability. not good.
# let's try m = 2**16, the smallest symbol can now take only 1/256 of the average
# -> m/l = 2**32, this feels like quite a lot
# 
# let's try m = 2**24, the smallest symbol can now take only 1/65536 of the average, this definitely feels like overkill
# -> m/l = 2**24, is this still good? (probably) (i just checked, least used symbol (words, stanmain dictionary) has p about 1e-7. using
#     eq. 17 from the duda paper i feel like this should dominate the order of magnitude, so a rough estimation would
#     give delta H = m * 1e-9 bits / symbol = 0.09 bits / symbol. this feels pretty okay, but i don't think we should
#     stretch it further.

l = 2**48
m = 2**24
k = l / m
b = 256

def discretize_frequencies(frequencies):

    first_guess = [max(1, round(frequency*m)) for frequency in frequencies]
    error = sum(first_guess) - m

    print(error)
    print(first_guess)

    if error > 0:
        # we have used too many slots, so those items where the rounding_error is highest (i.e. where the count
        # is higher than what it should be) will get downgraded. Exception: symbols where the count is already
        # only one, we still need to be able to encode these.
        rounding_errors = [(count - frequency*m, index)
                for index, (count, frequency)
                in enumerate(zip(first_guess, frequencies))
                if count > 1]

        # sort the highest first
        rounding_errors.sort(reverse=True)

        for _ in range(error):
            # take the highest item
            error, index = rounding_errors.pop(0)
            first_guess[index] -= 1

            if first_guess[index] > 1:
                # put it back
                # since we have rounding differences, which are between [-0.5, 0.5),
                # this is now automatically the smallest item
                rounding_errors.append((error - 1, index))

    elif error < 0:
        # we have used too few slots, so we can distribute some extra.
        rounding_errors = [(count-frequency*m, index)
                for index, (count, frequency)
                in enumerate(zip(first_guess, frequencies))]

        # sort the smallest first (this is going to be a negative number probably)
        rounding_errors.sort()

        for _ in range(error):
            error, index = rounding_errors.pop(0)
            first_guess[index] += 1

            # this is probably unnecessary? if everyone got rounded down,
            # the most we could have left over should be 256, i.e. everyone
            # gets an extra slot. so putting them back at the end shouldn't
            # be necessary.
            rounding_errors.append((error + 1, index))

    # there should be no zero counts, we need to be able to encode everything
    # we see
    assert first_guess.count(0) == 0

    cumulative = []
    accumulator = 0
    for count in first_guess:
        cumulative.append(accumulator)
        accumulator += count

    # cumulative now contains the starting position for each symbol.
    return cumulative

def rans_encode(state, symbol, cumulative, output):

    # check if we need to push bits
    # upper end of I_s range is b*l_s*k - 1, where k = l/m,
    # and l_s is the symbol count

    symbol_count = cumulative[symbol + 1] - cumulative[symbol]
    highest_acceptable_value_for_state = b * k * symbol_count - 1

    while state > highest_acceptable_value_for_state:
        output.append(state & 0xFF)
        state >>= 8

    # encode
    # find which m-block the current state falls into (for this symbol)
    # int rounds down
    block_index = int(state / symbol_count)

    # offset into the section of that symbol on that block
    offset_from_start_of_symbol_section = state % symbol_count

    offset_from_start_of_block = cumulative[symbol]

    block_offset = block_index * m

    state = block_offset + offset_from_start_of_block + offset_from_start_of_symbol_section

    return state

def rans_decode(state, cumulative, data):

    block_index = int(state / m)
    offset_from_start_of_block = state % m

    # (for the last symbol, the if will never run,
    #  and it will just stay assigned to the last symbol)
    for (symbol, start) in enumerate(cumulative):
        if start > offset_from_start_of_block:
            # this is the beginning of the next symbol,
            # so subtract one
            symbol = symbol - 1
            break

    offset_from_start_of_symbol_section = offset_from_start_of_block - cumulative[symbol]

    symbol_count = cumulative[symbol + 1] - cumulative[symbol]
    occurences_of_symbol_before_this_block = symbol_count * block_index

    state = occurences_of_symbol_before_this_block + offset_from_start_of_symbol_section

    # pull new bytes if necessary
    while state < l and len(data) > 0:
        state = (state << 8) | data.pop(0)

    return state, symbol


#print("loading words")
#with open("../../../stanmain.json") as f:
#    words = list(json.load(f).values())
#
#print("computing frequencies")
#chars = [char for word in words for char in word.encode("utf-8")]
#counts = [chars.count(char) for char in range(256)]
#total = sum(counts)
#probabilities = [count/total for count in counts]
#
#print("discretizing frequencies")
#cumulative = np.array(discretize_frequencies(probabilities))
#print(cumulative)
#np.save("fs.npy", cumulative)

cumulative = np.load("fs.npy")

print("encoding")
output = bytearray()
state = 0

for char in reversed("hello world!".encode("utf-8")):
    state = rans_encode(state, char, cumulative, output)
    print(state, output)

print(len(output))
print(output)

result = bytearray()
while state != 0:
    print(state, output)
    state, symbol = rans_decode(state, cumulative, output)
    result.append(symbol)

print(result)
print(result.decode("utf-8"))
