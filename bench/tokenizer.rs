/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::{io, os, str};
use std::default::Default;

use test::{black_box, BenchHarness, TestDesc, TestDescAndFn};
use test::{DynTestName, DynBenchFn, TDynBenchFn};

use html5::tokenizer::{TokenSink, Token, Tokenizer, TokenizerOpts};
use html5::DOMString;

struct Sink;

impl TokenSink for Sink {
    fn process_token(&mut self, token: Token) {
        // Don't use the token, but make sure we don't get
        // optimized out entirely.
        black_box(token);
    }
}

// This could almost be the TokenSink too, but it's not
// mut within run().
struct Bench {
    input: ~str,
    clone_only: bool,
    opts: TokenizerOpts,
}

impl Bench {
    fn new(name: &str, size: Option<uint>, clone_only: bool,
           opts: TokenizerOpts) -> Bench {
        let mut path = os::self_exe_path().expect("can't get exe path");
        path.push("../data/bench/");
        path.push(name);
        let mut file = io::File::open(&path).ok().expect("can't open file");
        let file_input = file.read_to_str().ok().expect("can't read file");

        let input = match size {
            None => file_input,
            Some(size) => {
                // Replicate the input in memory up to the desired size.
                let mut input = str::with_capacity(size);
                while input.len() < size {
                    input.push_str(file_input);
                }
                input
            }
        };

        Bench {
            input: input,
            clone_only: clone_only,
            opts: opts,
        }
    }
}

impl TDynBenchFn for Bench {
    fn run(&self, bh: &mut BenchHarness) {
        let input = DOMString::from_string(self.input.as_slice());
        bh.iter(|| {
            let my_input = input.clone();
            if self.clone_only {
                // Because the tokenizer consumes its buffers, we need
                // to clone inside iter().  We can benchmark this
                // separately and subtract it out.
                black_box(my_input);
            } else {
                let mut sink = Sink;
                let mut tok = Tokenizer::new(&mut sink, self.opts.clone());
                tok.feed(my_input);
                tok.end();
            }
        });
    }
}

fn make_bench(name: &str, size: Option<uint>, clone_only: bool,
              opts: TokenizerOpts) -> TestDescAndFn {
    TestDescAndFn {
        desc: TestDesc {
            name: DynTestName([
                ~"tokenize ",
                name.to_owned(),
                size.map_or(~"", |s| format!(" size {:7u}", s)),
                if clone_only { ~" (clone only)" } else { ~"" },
                if opts.exact_errors { ~" (exact errors)" } else { ~"" },
            ].concat()),
            ignore: false,
            should_fail: false,
        },
        testfn: DynBenchFn(~Bench::new(name, size, clone_only, opts)),
    }
}

pub fn tests() -> Vec<TestDescAndFn> {
    let mut tests = vec!(make_bench("lipsum.html", Some(1024*1024), true, Default::default()));

    let mut opts_vec = vec!(Default::default());
    if os::getenv("BENCH_EXACT_ERRORS").is_some() {
        opts_vec.push(TokenizerOpts {
            exact_errors: true,
            .. Default::default()
        });
    }

    for opts in opts_vec.iter() {
        for &file in ["lipsum.html", "lipsum-zh.html", "strong.html"].iter() {
            for &sz in [1024, 1024*1024].iter() {
                tests.push(make_bench(file, Some(sz), false, opts.clone()));
            }
        }

        if os::getenv("BENCH_UNCOMMITTED").is_some() {
            // Not checked into the repo, so don't include by default.
            for &file in ["webapps.html", "sina.com.cn.html", "wikipedia.html"].iter() {
                let name: ~str = (~"uncommitted/").append(file);
                tests.push(make_bench(name.as_slice(), None, false, opts.clone()));
            }
        }
    }

    tests
}
