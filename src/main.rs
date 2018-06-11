#![allow(warnings)] // remove when error_chain is fixed

//! `cargo run --example simple`
extern crate rayon;
extern crate reqwest;
#[macro_use]
extern crate error_chain;
#[macro_use] extern crate quicli;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use rayon::prelude::*;
use reqwest::header::ContentLength;
use reqwest::RedirectPolicy;
use quicli::prelude::*;


#[derive(Debug, StructOpt)]
struct Cli {
    /// Path to the wordlist
    #[structopt(long = "wordlist", short = "w", default_value = "", parse(from_os_str))]
    wordlist: PathBuf,
    /// The target URL or Domain
    #[structopt(long = "url", short = "u", default_value = "")]
    url: String,
    /// Positive status codes
    #[structopt(long = "statuscodes", short = "s", default_value = "200,204,301,302,307")] 
    statusCodes: String,
    // Quick and easy logging setup you get for free with quicli
    #[structopt(flatten)]
    verbosity: Verbosity,
}

struct State {
    client: reqwest::Client,
    url: String,
    statusCodes: Vec<u16>,
    wordlist: PathBuf,
}

impl State {
    fn new(args: Cli) -> State {
         State{client: reqwest::Client::builder()
                 .redirect(RedirectPolicy::none())
                 .build().unwrap(),
         url: args.url,
         statusCodes: args.statusCodes.split(",").map(|c| c.parse::<u16>()
                                                      .unwrap())
                                                      .collect::<Vec<u16>>(),
         wordlist: args.wordlist}
    }

    fn validate_args(&mut self) {
        if self.url.chars().last().unwrap() != '/' {
            self.url+= "/" ;
        };
    }

    fn printConfig(&self, len: usize) {
        println!("Rbuster 0.1.0                 Vadim Smirnov");
        println!("=====================================================");
        println!("Url/Domain    : {}", self.url);
        println!("Wordlist      : {}", self.wordlist.as_path().to_str().unwrap());
        println!("Words         : {}", len);

        println!("=====================================================");
    }
}


fn lines_from_file<P>(filename: P) -> Vec<String>
where P: AsRef<Path>,
      {
        let file = File::open(filename).expect("no such file");
        let buf = BufReader::new(file);
        buf.lines()
            .map(|l| l.expect("Could not parse line"))
            .collect()
      }


main!(|args: Cli, log_level: verbosity| {
    let mut state = State::new(args); 
    state.validate_args();
    let wordlist = lines_from_file(&state.wordlist);
    state.printConfig(wordlist.len());
    wordlist.par_iter()
        .for_each_with(&state, |c, s| {
            let url = format!("{}{}", state.url, s).to_string();
            let mut res = c.client.get(&url).send().unwrap();
            //let len = res.headers().get::<ContentLength>().map(|ct_len| **ct_len).unwrap_or(0);
            let len = &res.text().unwrap().len();
            info!("/{} (Status: {}) ", s, &res.status());
            if state.statusCodes.contains(&res.status().as_u16()) {
                println!("/{} (Status: {} | Content-Length: {})", s, &res.status(), len);

            };
        });
});
