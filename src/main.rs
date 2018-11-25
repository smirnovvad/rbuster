extern crate nanoid;
extern crate rayon;
extern crate reqwest;
#[macro_use]
extern crate quicli;

use quicli::prelude::*;
use reqwest::header::*;
use reqwest::RedirectPolicy;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, StructOpt)]
struct Cli {
    /// Specify a user agent string to send in the request header
    #[structopt(long = "user-agent", short = "a", default_value = "")]
    user_agent: String,
    /// HTTP Authorization via Bearer token.
    #[structopt(long = "bearer", short = "b", default_value = "")]
    bearer: String,
    /// HTTP Authorization username (Basic Auth only).
    #[structopt(long = "username", short = "U", default_value = "")]
    username: String,
    /// HTTP Authorization password (Basic Auth only).
    #[structopt(long = "password", short = "P", default_value = "")]
    password: String,
    /// use this to specify any cookies that you might need (simulating auth).
    #[structopt(long = "cookies", short = "c", default_value = "")]
    cookies: String,
    /// Follow redirects
    #[structopt(long = "force-wildcard", short = "f")]
    wildcard_forced: bool,
    /// Follow redirects
    #[structopt(long = "redirects", short = "r")]
    redirects: bool,
    /// Show the length of the response
    #[structopt(long = "length", short = "l")]
    length: bool,
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
        let mut headers = reqwest::header::HeaderMap::new();
        if args.redirects {
            let custom = RedirectPolicy::custom(|attempt| {
                if attempt.previous().len() > 5 {
                    attempt.too_many_redirects()
                } else {
                    attempt.follow()
                }
            });
            clientb = clientb.redirect(custom);
        } else {
            clientb = clientb.redirect(RedirectPolicy::none());
        }
        if !args.user_agent.is_empty() {
            headers.insert(
                USER_AGENT,
                HeaderValue::from_str(&args.user_agent.clone()).unwrap(),
            );
        }
        if !args.cookies.is_empty() {
            headers.insert(
                COOKIE,
                HeaderValue::from_str(&args.cookies.clone()).unwrap(),
            );
        }
        clientb = clientb.default_headers(headers);
        self.client = clientb.build().unwrap();
    }

    fn print_config(&self, len: usize) {
        println!("Rbuster 0.2.1                         Vadim Smirnov");
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
    let uid = format!("{}{}", &state.url, nanoid::simple()).to_string();
    match state.client.get(&uid).send() {
        Ok(res) => {
            if state.status_codes.contains(&res.status().as_u16()) {
                println!("[-] Wildcard response found: {} => {}", &uid, &res.status());
                if !&args.wildcard_forced {
                    error!("To force processing of Wildcard responses, specify the '-f' switch.");
                    ::std::process::exit(1);
                }
            }
        }
        Err(err) => {
            error!("{}", err);
            ::std::process::exit(1);
        }
    };
    wordlist.par_iter().for_each(|s| {
        let url = format!("{}{}", &state.url, &s).to_string();
        let mut req = state.client.head(&url);
        if !args.username.is_empty() {
            req = req.basic_auth(&args.username, Some(&args.password));
        } else if !args.bearer.is_empty() {
            req = req.bearer_auth(&args.bearer);
        }
        let res = req.send().unwrap();
        warn!("/{} (Status: {}) ", &s, &res.status());
        if state.status_codes.contains(&res.status().as_u16()) {
            if args.length {
                let mut req = state.client.get(&url);
                if !args.username.is_empty() {
                    req = req.basic_auth(&args.username, Some(&args.password));
                } else if !args.bearer.is_empty() {
                    req = req.bearer_auth(&args.bearer);
                }
                let mut res = req.send().unwrap();
                let len = &res.text().unwrap().len();
                println!(
                    "/{} (Status: {} | Content-Length: {})",
                    &s,
                    &res.status(),
                    len
                );
            } else {
                println!("/{} (Status: {})", &s, &res.status(),);
            }
        };
    });
});
