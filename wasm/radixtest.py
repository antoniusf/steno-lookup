import json
import os.path

with open(os.path.expandvars("$HOME/stanmain.json")) as f:
    dictionary = json.load(f)

words = list(dictionary.values())
words.sort()

max_bucket = 100
min_bucket = 40

def make_level(prefixed_words, prefix):

    starting_chars = [word[len(prefix)] if len(word) > len(prefix) else "" for word in prefixed_words]
    counts = [(char, starting_chars.count(char)) for char in sorted(list(set(starting_chars)))]
    print(f"prefix '{prefix}':")
    
    # accumulate counts that are lower than 100
    total = 0
    grand_total = 0
    current_beginning_start_char = None
    previous_char = None
    sections = []

    for char, count in counts:
        if count >= max_bucket:
            new_prefix = prefix + char
            make_level([word for word in prefixed_words if word.startswith(new_prefix)], new_prefix)

        else:
            if current_beginning_start_char == None:
                current_beginning_start_char = char
                total = count
                previous_char = char
                continue

            if total + count < min_bucket:
                total += count

            elif total + count < max_bucket:
                sections.append((current_beginning_start_char, char, total + count))
                current_beginning_start_char = None
                grand_total += total + count
                total = 0

            else:
                # since min_bucket is less than half of max_bucket, and we always
                # commit entries larger than min_bucket, this means that the new
                # entry has to be larger than min_bucket and can be commited as well.
                assert previous_char != None
                sections.append((current_beginning_start_char, previous_char, total))
                sections.append((char, char, count))
                current_beginning_start_char = None
                grand_total += total + count
                total = 0

            previous_char = char

    if current_beginning_start_char:
        # commit the last section
        sections.append((current_beginning_start_char, previous_char, total))
        grand_total += total

    print(f"prefix '{prefix}' sections: {sections}")

make_level(words, "")

