// TODO temporary!!
#![allow(dead_code)]
use core::mem::size_of;
use core::convert::TryInto;
use core::iter::Iterator;
use core::hash::Hasher;
use core::borrow::Borrow;
use wyhash::{wyhash, WyHash};

pub struct Entry<'a> {
    offset: usize,
    length: usize,
    pub key: &'a [u8],
    pub value: u32
}

impl<'a> Entry<'a> {
    fn new_with_length(data: &'a [u8], offset: usize, length: usize) -> Entry<'a> {
        let key_start = offset + 2;
        let key_end = offset + length - 4;
        let value_start = key_end;

        let key = &data[key_start .. key_end];
        let value_bytes = &data[value_start .. value_start + 4];
        let value = u32::from_ne_bytes(value_bytes.try_into().unwrap());

        Entry {
            length,
            offset,
            key,
            value
        }
    }

    fn new(data: &'a [u8], offset: usize) -> Entry<'a> {
        let length = u16::from_ne_bytes(
            data[offset .. offset + 2].try_into().unwrap())
            as usize;

        Entry::new_with_length(data, offset, length)
    }

    pub fn get_offset(&self) -> usize {
        self.offset
    }

    pub fn to_handle(self) -> EntryHandle {
        EntryHandle {
            offset: self.offset,
            length: self.length
        }
    }
}

// the purpose of EntryHandle is to not contain any borrows from
// the hashtable, while keeping position and length such that
// entry values can be conveniently set on the hashtable. (this
// wouldn't be possible otherwise, since entry contains an immutable
// borrow into the hashtable, but we need a mutable borrow to modify.
pub struct EntryHandle {
    offset: usize,
    length: usize
}

pub struct BucketEntryIterator<'a> {
    starting_offset: usize,
    offset: usize,
    end: usize,
    data: &'a [u8]
}

impl<'a> BucketEntryIterator<'a> {
    fn new<'b>(bucket_index: usize, buckets: &'b [usize], data: &'a [u8]) -> BucketEntryIterator<'a> {
        let offset = buckets[bucket_index];
        let data_length = data.as_ref().len();
        let end = *buckets.get(bucket_index + 1).unwrap_or(&data_length);

        BucketEntryIterator {
            starting_offset: offset,
            offset: offset,
            end,
            data
        }
    }
}

impl<'a> Iterator for BucketEntryIterator<'a> {
    type Item = Entry<'a>;

    fn next(&mut self) -> Option<Entry<'a>> {
        if self.offset >= self.end {
            return None;
        }

        let data = self.data.as_ref();

        let length = data[self.offset] as usize + ((data[self.offset] as usize) << 8);

        // this is only needed during initialization
        if length == 0 {
            return None;
        }

        let entry_offset = self.offset;
        self.offset += length;

        Some(Entry::new_with_length(data, entry_offset, length))
    }
}

pub struct HashTableMaker<I, J, K>
where
    I: Iterator<Item = J> + Clone,
    J: Iterator<Item = K> + Clone,
    K: Borrow<u8>
{
    num_entries: usize,
    num_keybytes_total: usize,
    load_factor: f64,
    keys: I
}

impl<I, J, K> HashTableMaker<I, J, K>
where
    I: Iterator<Item = J> + Clone,
    J: Iterator<Item = K> + Clone,
    K: Borrow<u8>
{
    pub fn initialize(keys: I) -> HashTableMaker<I, J, K> {
        let keys_firstpass = keys.clone();

        let mut num_entries = 0;
        let mut num_keybytes_total = 0;
        for key in keys_firstpass {
            num_entries += 1;
            num_keybytes_total += key.count();
        }

        HashTableMaker {
            num_entries,
            num_keybytes_total,
            load_factor: 10.0,
            keys
        }
    }

    pub fn set_load_factor(&mut self, load_factor: f64) {
        self.load_factor = load_factor;
    }

    pub fn get_buckets_length(&self) -> usize {
        let num_buckets = (self.num_entries as f64 / self.load_factor) as usize;

        num_buckets
    }

    pub fn get_data_length(&self) -> usize {
        let num_headerbytes_total = self.num_entries * 2;
        let data_length = num_headerbytes_total + self.num_keybytes_total;

        data_length
    }

    // each bucket will be initialized to u32::MAX
    pub fn make_hash_table<'b>(self, buckets_memory: &'b mut [usize], data_memory: &'b mut [u8]) -> HashTable<'b> {
        assert_eq!(buckets_memory.len(), self.get_buckets_length());
        assert_eq!(data_memory.len(), self.get_data_length());

        let buckets = buckets_memory;
        let data = data_memory;

        // structure the data array
        //
        // for this, we have to know how much space each bucket takes up
        // in the data array. for this, we're going to do a first pass
        // where we use the conveniently sized space in the buckets array
        // to store the amount of space that the corresponding bucket will
        // need in the data array.

        for bucket in buckets.iter_mut() {
            *bucket = 0;
        }

        let header_size = 2;
        let payload_size = size_of::<usize>();

        for key in self.keys.clone() {
            let total_size = header_size + key.clone().count() + payload_size;

            let index = get_bucket_index_from_iterator(key, buckets);
            buckets[index] += total_size;
        }

        // now that we know how much space each bucket needs, we can
        // compute the necessary offset into the data array by doing
        // a simple prefix sum.
        //
        // we will also initialize each bucket array with an empty marker.
        // (normally, the first two bytes indicate the length of the first
        // entry in the bucket, so that we can quickly jump to the next one.
        // since this length includes these first two bytes, it is never 0
        // once the first entry is populated, and can thus serve as an
        // emptyness marker.)

        let mut offset = 0;
        for bucket in buckets.iter_mut() {
            let bucket_size = *bucket;

            if bucket_size > 0 {
                *bucket = offset;
                offset += bucket_size;

                // initialize the corresponding data section
                data[*bucket .. *bucket + 2].copy_from_slice(
                    &0_u16.to_ne_bytes());
            }
            else {
                // mark this as an empty bucket
                *bucket = usize::max_value();
            }
        }

        for key in self.keys.clone() {
            let index = get_bucket_index_from_iterator(key.clone(), buckets);
            let mut offset = buckets[index];

            // find the next empty spot
            loop {
                let length = u16::from_ne_bytes(
                    (&data[offset .. offset + 2]).try_into().unwrap());

                if length == 0 {
                    break;
                }

                offset += length as usize;
            }

            let key_length = key.clone().count();
            let length = header_size + key_length + payload_size;
            assert!(length < 0xFFFF);

            // set length
            data[offset .. offset + 2].copy_from_slice( 
                &(length as u16).to_ne_bytes());
            offset += 2;

            // copy key
            let mut write_pos = offset;
            for byte in key {
                data[write_pos] = *byte.borrow();
                write_pos += 1;
            }
            offset += key_length;

            // set payload to 0xffffffff (to indicate that it has not
            // been set yet)
            data[offset .. offset + 4].copy_from_slice(
                &u32::MAX.to_ne_bytes());
            offset += 4;

            assert_eq!(offset - buckets[index], length);

            let bucket_end = *buckets.get(index + 1).unwrap_or(&data.len());
            let this_bucket_is_full = offset == bucket_end;
            if !this_bucket_is_full {
                // set empty marker for the next entry
                data[offset .. offset + 2].copy_from_slice(
                    &0_u16.to_ne_bytes());
            }
        }

        HashTable {
            buckets,
            data
        }
    }
}

fn get_bucket_index(string: &[u8], buckets: &[usize]) -> usize {

    let hash = wyhash(string, 1);
    let index = (hash as usize) % buckets.len();

    return index;
}

fn get_bucket_index_from_iterator(iterator: impl Iterator<Item = impl Borrow<u8>>, buckets: &[usize]) -> usize {
    let mut hasher = WyHash::with_seed(1);
    // wyhash iterates over groups of 32 bytes internally
    // we do the same so that the final results will match
    let mut buffer = [0u8; 32];
    let mut buffer_pos = 0;
    for byte in iterator {
        if buffer_pos >= buffer.len() {
            hasher.write(&buffer);
            buffer_pos = 0;
        }

        buffer[buffer_pos] = *byte.borrow();
    }

    hasher.write(&buffer[..buffer_pos]);
    let hash = hasher.finish();
    let index = (hash as usize) % buckets.len();

    return index;
}

// keys are always (&)[u8]
// values are always u32
pub struct HashTable<'a> {
    buckets: &'a [usize],
    data: &'a mut [u8]
}

impl<'a> HashTable<'a> {
    pub fn get_bucket_iterator<'b, 'c>(&'c self, key: &'b [u8]) -> BucketEntryIterator<'c> {
        let index = get_bucket_index(key, self.buckets);
        BucketEntryIterator::new(index, self.buckets, self.data)
    }

    pub fn get_bucket_iterator_from_key_iterator<'c, I, K>(&'c self, key: I) -> BucketEntryIterator<'c>
    where
        I: Iterator<Item = K>,
        K: Borrow<u8>
    {
        let index = get_bucket_index_from_iterator(key, self.buckets);
        BucketEntryIterator::new(index, self.buckets, self.data)
    }

    pub fn get_values<'c>(&'c self, key: &'c [u8]) -> impl Iterator<Item = u32> + 'c {
        let iterator = self.get_bucket_iterator(key);

        iterator.filter(move |entry| entry.key == key)
            .map(|entry| entry.value)
    }

    // returns the first value for this key, or None. There is
    // no indication whether there could have been additional values.
    pub fn get_value(&self, key: &[u8]) -> Option<u32> {
        self.get_values(key).next()
    }

    // finds the first unset value for this key and sets
    // it to the given value. returns Some(()) if it was
    // successful, None otherwise (no unset value was found).
    pub fn set_unset_value(&mut self, key: &[u8], value: u32) -> Option<()> {
        let mut iterator = self.get_bucket_iterator(key);

        while let Some(entry) = iterator.next() {
            if entry.key == key && entry.value == u32::MAX {
                // value is the last 4 bytes of the previous entry
                let value_offset = iterator.offset - 4;
                self.data[value_offset .. value_offset + 4].copy_from_slice(
                    &value.to_ne_bytes());
                return Some(());
            }
        }

        None
    }

    pub fn set_value(&mut self, entry_handle: EntryHandle, value: u32) {
        let entry_end = entry_handle.offset + entry_handle.length;
        let value_bytes =
            &mut self.data[entry_end - 4 .. entry_end];

        value_bytes.copy_from_slice(&value.to_ne_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestIterator {
        data: &'static [u8],
        offset: usize
    }

    impl Iterator for TestIterator {
        type Item = core::slice::Iter<'static, u8>;

        fn next(&mut self) -> Option<Self::Item> {
            if self.offset + 4 < self.data.len() {
                Some(self.data[self.offset..self.offset+4].iter())
            }
            else {
                None
            }
        }
    }
    
    #[test]
    fn test_hashtable_initialization () {
        let mut test_iterator = TestIterator {
            data: b"asdf bla hello world",
            offset: 0
        };
        let mut hash_table_maker = HashTableMaker::initialize(test_iterator);
    }
}
