/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::str::CharRange;
use collections::deque::Deque;
use collections::dlist::DList;

struct Buffer {
    /// Byte position within the buffer.
    pos: uint,
    /// The buffer.
    buf: ~str,
}

impl Buffer {
    fn new(buf: ~str) -> Buffer {
        Buffer {
            pos: 0,
            buf: buf,
        }
    }
}


/// Either a single character or a run of "data" characters: those which
/// don't trigger input stream preprocessing, or special handling in any
/// of the Data / RawData / Plaintext tokenizer states.  We do not exclude
/// characters which trigger a parse error but are otherwise handled
/// normally.
#[deriving(Eq)]
pub enum DataRunOrChar {
    DataRun(~str),
    OneChar(char),
}

/// Count the number of data characters at the beginning of 's'.
fn data_span(s: &str) -> uint {
    let mut n = 0;
    for b in s.bytes() {
        match b {
        //  \0     \r     &      -      <
            0x00 | 0x0D | 0x26 | 0x2D | 0x3C => break,
            _ => n += 1,
        }
    }
    n
}

/// A queue of owned string buffers, which supports incrementally
/// consuming characters.
pub struct BufferQueue {
    /// Buffers to process.
    priv buffers: DList<Buffer>,

    /// Number of available characters.
    priv available: uint,
}

impl BufferQueue {
    /// Create an empty BufferQueue.
    pub fn new() -> BufferQueue {
        BufferQueue {
            buffers: DList::new(),
            available: 0,
        }
    }

    /// Add a buffer to the beginning of the queue.
    pub fn push_front(&mut self, buf: ~str) {
        self.account_new(buf.as_slice());
        self.buffers.push_front(Buffer::new(buf));
    }

    /// Add a buffer to the end of the queue.
    pub fn push_back(&mut self, buf: ~str) {
        self.account_new(buf.as_slice());
        self.buffers.push_back(Buffer::new(buf));
    }

    /// Do we have at least n characters available?
    pub fn has(&mut self, n: uint) -> bool {
        self.available >= n
    }

    /// Get multiple characters, if that many are available.
    pub fn pop_front(&mut self, n: uint) -> Option<~str> {
        if !self.has(n) {
            return None;
        }
        // FIXME: this is probably pretty inefficient
        Some(self.by_ref().take(n).collect())
    }

    /// Look at the next available character, if any.
    pub fn peek(&mut self) -> Option<char> {
        self.drop_empty_buffers();
        match self.buffers.front() {
            Some(&Buffer { pos, ref buf }) => Some(buf.char_at(pos)),
            None => None,
        }
    }

    /// Pop either a single character or a run of "data" characters.
    /// See `DataRunOrChar` for what this means.
    pub fn pop_data(&mut self) -> Option<DataRunOrChar> {
        self.drop_empty_buffers();
        match self.buffers.front_mut() {
            Some(&Buffer { ref mut pos, ref buf }) => {
                let n = data_span(buf.slice_from(*pos));

                // If we only have one character then it's cheaper not to allocate.
                if n > 1 {
                    let new_pos = *pos + n;
                    let out = buf.slice(*pos, new_pos).to_owned();
                    *pos = new_pos;
                    self.available -= n;
                    Some(DataRun(out))
                } else {
                    let CharRange { ch, next } = buf.char_range_at(*pos);
                    *pos = next;
                    self.available -= 1;
                    Some(OneChar(ch))
                }
            }
            _ => None,
        }
    }

    fn account_new(&mut self, buf: &str) {
        // FIXME: We could pass through length from the initial ~[u8] -> ~str
        // conversion, which already must re-encode or at least scan for UTF-8
        // validity.
        self.available += buf.char_len();
    }

    fn drop_empty_buffers(&mut self) {
        loop {
            match self.buffers.front_mut() {
                Some(&Buffer { pos, ref buf }) if pos >= buf.len() => (),
                _ => break,
            }
            self.buffers.pop_front();
        }
    }
}

impl Iterator<char> for BufferQueue {
    /// Get the next character, if one is available.
    ///
    /// Because more data can arrive at any time, this can return Some(c) after
    /// it returns None.  That is allowed by the Iterator protocol, but it's
    /// unusual!
    fn next(&mut self) -> Option<char> {
        self.drop_empty_buffers();
        match self.buffers.front_mut() {
            None => None,
            Some(&Buffer { ref mut pos, ref buf }) => {
                let CharRange { ch, next } = buf.char_range_at(*pos);
                *pos = next;
                self.available -= 1;
                Some(ch)
            }
        }
    }
}


#[test]
fn smoke_test() {
    let mut bq = BufferQueue::new();
    assert_eq!(bq.has(1), false);
    assert_eq!(bq.peek(), None);
    assert_eq!(bq.next(), None);

    bq.push_back(~"abc");
    assert_eq!(bq.has(1), true);
    assert_eq!(bq.has(3), true);
    assert_eq!(bq.has(4), false);

    assert_eq!(bq.peek(), Some('a'));
    assert_eq!(bq.next(), Some('a'));
    assert_eq!(bq.peek(), Some('b'));
    assert_eq!(bq.peek(), Some('b'));
    assert_eq!(bq.next(), Some('b'));
    assert_eq!(bq.peek(), Some('c'));
    assert_eq!(bq.next(), Some('c'));
    assert_eq!(bq.peek(), None);
    assert_eq!(bq.next(), None);
}

#[test]
fn can_pop_front() {
    let mut bq = BufferQueue::new();
    bq.push_back(~"abc");

    assert_eq!(bq.pop_front(2), Some(~"ab"));
    assert_eq!(bq.peek(), Some('c'));
    assert_eq!(bq.pop_front(2), None);
    assert_eq!(bq.next(), Some('c'));
    assert_eq!(bq.next(), None);
}

#[test]
fn can_unconsume() {
    let mut bq = BufferQueue::new();
    bq.push_back(~"abc");
    assert_eq!(bq.next(), Some('a'));

    bq.push_front(~"xy");
    assert_eq!(bq.next(), Some('x'));
    assert_eq!(bq.next(), Some('y'));
    assert_eq!(bq.next(), Some('b'));
    assert_eq!(bq.next(), Some('c'));
    assert_eq!(bq.next(), None);
}

#[test]
fn can_pop_data() {
    let mut bq = BufferQueue::new();
    bq.push_back(~"abc\0def");
    assert_eq!(bq.pop_data(), Some(DataRun(~"abc")));
    assert_eq!(bq.pop_data(), Some(OneChar('\0')));
    assert_eq!(bq.pop_data(), Some(DataRun(~"def")));
    assert_eq!(bq.pop_data(), None);
}

#[test]
fn data_span_test() {
    fn pad(s: &mut ~str, n: uint) {
        for _ in range(0, n) {
            s.push_char('x');
        }
    }

    for &c in ['&', '\0'].iter() {
        for x in range(0, 48u) {
            for y in range(0, 48u) {
                let mut s = ~"";
                pad(&mut s, x);
                s.push_char(c);
                pad(&mut s, y);

                assert_eq!(x, data_span(s.as_slice()));
            }
        }
    }
}
