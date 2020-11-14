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
# (actually, my old phone is armv7, which is 32 bits, so yeah)
# (i did some tests and 32 bits is enough anyway)
# 
# make l as large as possible:
# 
#     b*l - 1 is upper end of range, ie INT_MAX
#     b*l - 1 = 2**64 - 1
# 
#     <=> b*l = 2**64
#     <=> l = 2**64 / b = 2**64 / 2**8 = 2**56
# 
# how big should m be?
# 
# -> big, so we can map probabilities accurately
# -> not too big, so m/l stays small
# 
# there are 256 symbols, so if m is 256, everyone has the same probability. not good.
# let's try m = 2**16, the smallest symbol can now take only 1/256 of the average
# -> m/l = 2**40, this feels like quite a lot
#   I did some more tests, and on the sample data m = 2**16 leads to an increase in size
#   of about 0.07%. That's already overkill.
# 
# let's try m = 2**24, the smallest symbol can now take only 1/65536 of the average, this definitely feels like overkill
# -> m/l = 2**32, is this still good? (probably) (i just checked, least used symbol (words, stanmain dictionary) has p about 1e-7. using
#     eq. 17 from the duda paper i feel like this should dominate the order of magnitude, so a rough estimation would
#     give delta H = m * 1e-11 bits / symbol = 0.0003 bits / symbol. this feels pretty okay, but i don't think we should
#     stretch it further.

# this means that l*b will be at most 2**31 - 1.
# we need the state to be only 31 bits, not 32,
# so that the reciprocal multiplication will work.
# (needs 2*w + 1 bits, so with 32 we'd land at 65,
# which is not handy. plus, the reciprocal needs
# w + 1 bits, so with w = 31 it will fit into a
# 32-bit variable.

l_bits = 23
m_bits = 16
l = 1 << l_bits
m = 1 << m_bits
k = l / m
b = 256

def discretize_frequencies(frequencies):

    first_guess = []
    for frequency in frequencies:
        scaled = frequency*m
        down = int(scaled)
        up = down + 1

        if scaled**2 < down * up:
            first_guess.append(down)
        else:
            first_guess.append(up)

    print(first_guess)

    correction = m - sum(first_guess)
    correction_sign = -1 if correction < 0 else 1

    codelen_deltas = []
    for symbol, (scaled, frequency) in enumerate(zip(first_guess, frequencies)):
        if (scaled + correction_sign) == 0:
            codelen_delta = np.inf
        else:
            codelen_delta = frequency * np.log2(np.float64(scaled) / (scaled + correction_sign))
        codelen_deltas.append((symbol, codelen_delta))

    codelen_deltas.sort(key=lambda e: e[1])

    while correction != 0:
        symbol, delta = codelen_deltas.pop(0)
        first_guess[symbol] += correction_sign
        if (first_guess[symbol] + correction_sign) == 0:
            delta = np.inf
        else:
            delta = frequencies[symbol] * np.log2(np.float64(first_guess[symbol]) / (first_guess[symbol] + correction_sign))

        correction -= correction_sign
        
        # who cares about perf anyways??
        # computer go brrr
        codelen_deltas.append((symbol, delta))
        codelen_deltas.sort(key=lambda e: e[1])


    #if error > 0:
    #    # we have used too many slots, so those items where the rounding_error is highest (i.e. where the count
    #    # is higher than what it should be) will get downgraded. Exception: symbols where the count is already
    #    # only one, we still need to be able to encode these.
    #    rounding_errors = [(count - frequency*m, index)
    #            for index, (count, frequency)
    #            in enumerate(zip(first_guess, frequencies))
    #            if count > 1]

    #    # sort the highest first
    #    rounding_errors.sort(reverse=True)

    #    for _ in range(error):
    #        # take the highest item
    #        error, index = rounding_errors.pop(0)
    #        first_guess[index] -= 1

    #        if first_guess[index] > 1:
    #            # put it back
    #            # since we have rounding differences, which are between [-0.5, 0.5),
    #            # this is now automatically the smallest item
    #            rounding_errors.append((error - 1, index))

    #elif error < 0:
    #    # we have used too few slots, so we can distribute some extra.
    #    rounding_errors = [(count-frequency*m, index)
    #            for index, (count, frequency)
    #            in enumerate(zip(first_guess, frequencies))]

    #    # sort the smallest first (this is going to be a negative number probably)
    #    rounding_errors.sort()

    #    for _ in range(error):
    #        error, index = rounding_errors.pop(0)
    #        first_guess[index] += 1

    #        # this is probably unnecessary? if everyone got rounded down,
    #        # the most we could have left over should be 256, i.e. everyone
    #        # gets an extra slot. so putting them back at the end shouldn't
    #        # be necessary.
    #        rounding_errors.append((error + 1, index))

    # there should be no zero counts, we need to be able to encode everything
    # we see
    #assert first_guess.count(0) == 0

    print(first_guess)
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
        #print(f"outputting bits! before: {math.log2(state)}")
        state >>= 8
        #print(f"after: {math.log2(state)}")

    # encode
    # find which m-block the current state falls into (for this symbol)
    # int division rounds down

    # use alverson 91 integer reciprocals
    # yes i know it doesn't make sense to do it this way,
    # i just want to make sure i know how it works
    shift = 0
    while symbol_count > (1 << shift):
        shift += 1 # i just stole this from ryg's code

    shift += 31
    # this is the same as ceil(2**shift / symbol_count)
    # i also stole this from ryg, but it's a well-known
    # trick apparently
    a = ((1 << shift) + symbol_count - 1) // symbol_count
    block_index = (state * a) >> shift
    block_index_should_be = state // symbol_count
    if block_index != block_index_should_be:
        print(f"target result: {block_index_should_be}, reciprocals: {block_index}")
        print(f"divisor is {symbol_count}, shift is {shift - 31}")
    #block_index = state // symbol_count

    # offset into the section of that symbol on that block
    offset_from_start_of_symbol_section = state % int(symbol_count)

    offset_from_start_of_block = cumulative[symbol]

    block_offset = block_index * m

    assert offset_from_start_of_block + offset_from_start_of_symbol_section < m
    new_state = block_offset + offset_from_start_of_block + offset_from_start_of_symbol_section

    #print(f"encoding symbol {chr(symbol) if symbol < 127 else symbol}, added {math.log2(new_state) - math.log2(state):.2f} bits (optimal: {get_entropy_for_char(symbol)})")

    return new_state

def rans_decode(state, cumulative, data):

    # pull new bytes if necessary
    while state < l and len(data) > 0:
        state = (state << 8) | data.pop()
        #print("refilling:", state)

    if state < l:
        # we're done!
        raise StopIteration(state)

    block_index = state // m
    offset_from_start_of_block = state & ((1 << m_bits) - 1)

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

    return state, symbol

def get_entropy_for_char(char):
    p = probabilities[char]
    return math.log2(1/p)

print("loading words")
with open("../../../stanmain.json") as f:
    words = list(json.load(f).values())

#print("computing frequencies")
#chars = [char for word in words for char in word.encode("utf-8")]
#counts = [chars.count(char) for char in range(256)]
#total = sum(counts)
#probabilities = [count/total for count in counts]
#np.save("fs.npy", np.array(probabilities))
probabilities = np.load("fs.npy")

print("discretizing frequencies")
cumulative = np.array(discretize_frequencies(probabilities), dtype="uint64")
print(cumulative)

# numpy types somehow cast themselves to float64 at some point,
# and never go back
cumulative = [int(e) for e in cumulative]

print("encoding")
output = bytearray()

# start below the valid range for l, so that
# the decoder knows when to stop
# i.e.: state < l = 2**23
# we want to set the first bit, to make
# sure that the value is never too small.
# => bonus_value < l / 2 = 2**(23 - 1)
bonus_value = 0
assert bonus_value < (1 << (l_bits - 1))
state = bonus_value + (1 << (l_bits - 1))

# idea for pre-filling the state (note that we don't have to get
# it up to at least l, just anything that isn't super low will
# help with decoding efficiency):
# each string will be stored along with additional info, such
# as string length (at least for some), as well as a four-byte
# offset to the rest of the corresponding entry. we can initialize
# the state using this four-byte value (or other data), maybe set
# the next higher bit to ensure that this actually gets us to a high
# initial state value, and start encoding without the precision loss.
#
# second thought: we probably shouldn't store string length in there
# since we will only know the pre-fill data *after* decoding. that's
# not good enough for the string lengths since we need to know these
# quickly.

states = []

for word in words:
    for char in reversed(word.encode("utf-8")):
        states.append(state)
        state = rans_encode(state, char, cumulative, output)
        #print(state, output)

#for char in reversed("hello world".encode("utf-8")):
#    states.append(state)
#    state = rans_encode(state, char, cumulative, output)

print(words[-1].encode("utf-8"))

print(states[-2])
print(states[-1])
print(state)

coded_entropy = math.log2(state) + len(output)*8 - (l_bits - 1)
print(f"final entropy of encoded data: {coded_entropy:.3f}")

while state > 0:
    output.append(state & 0xFF)
    state >>= 8
    print(state)

#print(len(output) * 8 - 31)
#print(output)

result = bytearray()
while True:
    try:
        state, symbol = rans_decode(state, cumulative, output)
    except StopIteration as e:
        state = e.value
        print(f"The bonus value was: {state - (1 << (l_bits - 1))}")
        break
    #print("state:", state)
    #print("decoded symbol:", symbol)
    #assert state == states.pop()
    #should_be = states.pop()
    #if state != should_be:
    #    print(f"state should be {should_be}, but is {state} instead")
    #    assert False
    result.append(symbol)

compare = b"".join(w.encode("utf-8") for w in reversed(words))
assert compare == result

#print(result)
entropy = 0
for char in result:
    entropy += get_entropy_for_char(char)

#print(result.decode("utf-8"))

print(f"entropy in result: {entropy}")
print(f"loss: {coded_entropy - entropy:.3f} bits")
print(f"inefficiency: {(coded_entropy - entropy) / entropy * 100:.3f}%")
