extern crate rayon;
extern crate reqwest;
#[macro_use]
extern crate quicli;

use quicli::prelude::*;
use rayon::prelude::*;
use reqwest::header;
use reqwest::RedirectPolicy;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, StructOpt)]
struct Cli {
    /// Specify a user agent string to send in the request header
    #[structopt(long = "user-agent", short = "a", default_value = "")]
    user_agent: String,
    /// Follow redirects
    #[structopt(long = "redirects", short = "r")]
    redirects: bool,
    /// Path to the wordlist
    #[structopt(long = "wordlist", short = "w", parse(from_os_str))]
    wordlist: PathBuf,
    /// The target URL or Domain
    #[structopt(long = "url", short = "u")]
    url: String,
    /// Positive status codes coma-separated
    #[structopt(long = "statuscodes", short = "s", default_value = "200,204,301,302,307")]
    status_codes: String,
    // Quick and easy logging setup you get for free with quicli
    #[structopt(flatten)]
    verbosity: Verbosity,
}

struct State {
    client: reqwest::Client,
    url: String,
    status_codes: Vec<u16>,
    wordlist: PathBuf,
}

impl State {
    fn new(args: &Cli) -> State {
        State {
            client: reqwest::Client::builder()
                .redirect(RedirectPolicy::none())
                .build()
                .unwrap(),
            url: args.url.clone(),
            status_codes: args
                .status_codes
                .split(",")
                .map(|c| c.parse::<u16>().unwrap())
                .collect::<Vec<u16>>(),
            wordlist: args.wordlist.clone(),
        }
    }

    fn validate_args(&mut self, args: &Cli) {
        if self.url.chars().last().unwrap() != '/' {
            self.url += "/";
        };
        let mut clientb = reqwest::Client::builder();
        let mut headers = header::Headers::new();
        if args.redirects {
            let custom = RedirectPolicy::custom(|attempt| {
                if attempt.previous().len() > 5 {
                    attempt.too_many_redirects()
                } else {
                    attempt.follow()
                }
            });
            clientb.redirect(custom);
        }
        if !args.user_agent.is_empty() {
            headers.set(header::UserAgent::new(args.user_agent.clone()));
        }
        clientb.default_headers(headers);
        self.client = clientb.build().unwrap();
    }

    fn print_config(&self, len: usize) {
        println!("Rbuster 0.1.2                         Vadim Smirnov");
        println!("=====================================================");
        println!("Url/Domain    : {}", self.url);
        println!(
            "Wordlist      : {}",
            self.wordlist.as_path().to_str().unwrap()
        );
        println!("Words         : {}", len);
        println!("Status        : {:?}", self.status_codes);

        println!("=====================================================");
    }
}

fn lines_from_file<P>(filename: P) -> Vec<String>
where
    P: AsRef<Path>,
{
    let file = File::open(filename).expect("no such file");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not parse line"))
        .collect()
}

main!(|args: Cli, log_level: verbosity| {
    let mut state = State::new(&args);
    state.validate_args(&args);
    let wordlist = lines_from_file(&state.wordlist);
    state.print_config(wordlist.len());
    match state.client.get(&state.url).send() {
        Ok(_) => (),
        Err(err) => {
            error!("{}", err);
            ::std::process::exit(1);
        }
    };
    wordlist.into_par_iter().for_each_with(&state, |c, s| {
        let url = format!("{}{}", state.url, s).to_string();
        let res = c.client.head(&url).send().unwrap();
        warn!("/{} (Status: {}) ", s, &res.status());
        if state.status_codes.contains(&res.status().as_u16()) {
            let mut res = c.client.get(&url).send().unwrap();
            let len = &res.text().unwrap().len();
            println!(
                "/{} (Status: {} | Content-Length: {})",
                s,
                &res.status(),
                len
            );
        };
    });
});
