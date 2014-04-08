/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use util::str::empty_str;

// FIXME: already exists in Servo DOM
#[deriving(Eq, Clone)]
pub struct Doctype {
    name: Option<~str>,
    public_id: Option<~str>,
    system_id: Option<~str>,
    force_quirks: bool,
}

impl Doctype {
    pub fn new() -> Doctype {
        Doctype {
            name: None,
            public_id: None,
            system_id: None,
            force_quirks: false,
        }
    }
}

#[deriving(Eq, Clone)]
pub struct Attribute {
    name: ~str,
    value: ~str,
}

impl Attribute {
    pub fn new() -> Attribute {
        Attribute {
            name: empty_str(),
            value: empty_str(),
        }
    }

    pub fn clear(&mut self) {
        self.name.truncate(0);
        self.value.truncate(0);
    }
}

#[deriving(Eq, Clone)]
pub enum TagKind {
    StartTag,
    EndTag,
}

#[deriving(Eq, Clone)]
pub struct Tag {
    kind: TagKind,
    name: ~str,
    self_closing: bool,
    attrs: ~[Attribute],
}

impl Tag {
    pub fn new(kind: TagKind) -> Tag {
        Tag {
            kind: kind,
            name: empty_str(),
            self_closing: false,
            attrs: ~[],
        }
    }
}

#[deriving(Eq, Clone)]
pub enum Token {
    DoctypeToken(Doctype),
    TagToken(Tag),
    CommentToken(~str),
    CharacterToken(char),
    MultiCharacterToken(~str),
    EOFToken,
    ParseError(~str),
}
