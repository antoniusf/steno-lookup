import json
import os.path

def get_number_of_matching_chars(string1, string2):

    for index in range(min(len(string1), len(string2))):
        if string1[index] != string2[index]:
            return index

    return min(len(string1), len(string2))

def find_optimal_end_point_in_interval(words, start_word, end_interval, end_interval_offset):

    matching_chars_at_beginning_of_interval = get_number_of_matching_chars(start_word, end_interval[0])
    matching_chars_at_end_of_interval = get_number_of_matching_chars(start_word, end_interval[-1])

    #print(f"Chunk starts at {start_word}.")
    #print(f"Chunk end interval starts at {end_interval[0]}, and ends at {end_interval[-1]}")
    #print(f"This means that {matching_chars_at_beginning_of_interval} match at the beginning, \
    #and {matching_chars_at_end_of_interval} match at the end.")
    
    assert matching_chars_at_beginning_of_interval >= matching_chars_at_end_of_interval

    # we want to match the maximum number of characters, hence, the beginning of the interval
    match_this_many_chars = matching_chars_at_beginning_of_interval
    match_prefix = start_word[:match_this_many_chars]
    assert start_word[:match_this_many_chars] == end_interval[0][:match_this_many_chars]
    #print(f"Overall, we want this chunk to have a prefix of {match_prefix}")

    if matching_chars_at_beginning_of_interval != matching_chars_at_end_of_interval:
        # then, find where these chars no longer match
        *_, end_index = (index for (index, word) in enumerate(end_interval) if word.startswith(match_prefix))

        #print(f"This means that the chunk ends with the word {end_interval[end_index]}. The next chunk \
        #starts with {end_interval[end_index+1]}")
        print(f"{match_prefix}: {start_word} – {end_interval[end_index]}")

        return end_index + end_interval_offset

    else:
        # there is no optimal end index, the caller should handle this
        #print(f"This means that there is no optimal end point, the caller will handle this.")
        print(f"{match_prefix}")
        return None



with open(os.path.expandvars("$HOME/stanmain.json")) as f:
    dictionary = json.load(f)

words = list(dictionary.values())
words.sort()

min_chunksize = 40
target_chunksize = 80
max_chunksize = 100

chunk_start_index = 0

while True:
    start_word = words[chunk_start_index]
    end_interval_start = min(len(words), chunk_start_index + min_chunksize)
    end_interval_end = min(len(words), chunk_start_index + max_chunksize)

    if end_interval_start != end_interval_end:
        end_point = find_optimal_end_point_in_interval(words, start_word, words[end_interval_start:end_interval_end], end_interval_start)
        if not end_point:
            # there is no optimal end point, so it will just be at:
            end_point = min(len(words), chunk_start_index + target_chunksize)
    else:
        end_point = end_interval_start
        break

    chunk_start_index = end_point + 1

    #print()
    #print()

