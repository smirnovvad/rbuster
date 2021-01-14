extern crate nanoid;
extern crate reqwest;
use quicli::prelude::*;
use structopt::StructOpt;

use futures::{stream, StreamExt};
use reqwest::header::*;
use reqwest::redirect::Policy;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

const PARALLEL_REQUESTS: usize = 100;

#[derive(Debug, StructOpt, Clone)]
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
    #[structopt(
        long = "statuscodes",
        short = "s",
        default_value = "200,204,301,302,307"
    )]
    status_codes: String,
    // Quick and easy logging setup you get for free with quicli
    #[structopt(flatten)]
    verbosity: Verbosity,
}

#[derive(Clone)]
struct State {
    client: reqwest::Client,
    url: String,
    status_codes: Vec<u16>,
    wordlist: PathBuf,
}

impl State {
    fn new(args: Cli) -> State {
        State {
            client: reqwest::Client::builder()
                .redirect(Policy::none())
                .build()
                .unwrap(),
            url: args.url,
            status_codes: args
                .status_codes
                .split(",")
                .map(|c| c.parse::<u16>().unwrap())
                .collect::<Vec<u16>>(),
            wordlist: args.wordlist.clone(),
        }
    }

    fn validate_args(&mut self, args: Cli) {
        if self.url.chars().last() != Some('/') {
            self.url = self.url.to_owned() + "/";
        };
        let mut clientb = reqwest::Client::builder();
        let mut headers = reqwest::header::HeaderMap::new();
        if args.redirects {
            let custom = Policy::custom(|attempt| {
                if attempt.previous().len() > 5 {
                    attempt.stop()
                } else {
                    attempt.follow()
                }
            });
            clientb = clientb.redirect(custom);
        } else {
            clientb = clientb.redirect(Policy::none());
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

#[tokio::main]
async fn main() {
    let args = Cli::from_args();
    let mut state = State::new(args.clone());
    state.validate_args(args.clone());
    let wordlist = lines_from_file(&state.wordlist);
    state.print_config(wordlist.len());
    match state.client.get(&state.url).send().await {
        Ok(_) => (),
        Err(err) => {
            error!("{}", err);
            ::std::process::exit(1);
        }
    };
    let uid = format!("{}{}", &state.url, nanoid::simple()).to_string();
    match state.client.get(&uid).send().await {
        Ok(res) => {
            if state.status_codes.contains(&res.status().as_u16()) {
                println!("[-] Wildcard response found: {} => {}", &uid, &res.status());
                if !args.wildcard_forced {
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

    let out = stream::iter(wordlist)
        .map(|s| {
            let state_ = state.clone();
            let args_ = args.clone();
            let url = format!("{}{}", &state_.url, &s).to_string();
            let mut req = state_.client.head(&url);

            tokio::spawn(async move {
                if !args_.username.is_empty() {
                    req = req.basic_auth(&args_.username, Some(&args_.password));
                } else if !args_.bearer.is_empty() {
                    req = req.bearer_auth(&args_.bearer);
                }
                let res = req.send().await.unwrap();
                warn!("/{} (Status: {}) ", &s, &res.status());
                if state_.status_codes.contains(&res.status().as_u16()) {
                    if args_.length {
                        let mut req = state_.client.get(&url);
                        if !args_.username.is_empty() {
                            req = req.basic_auth(&args_.username, Some(&args_.password));
                        } else if !args_.bearer.is_empty() {
                            req = req.bearer_auth(&args_.bearer);
                        }
                        let res = req.send().await.unwrap();
                        let status = &res.status();
                        let len = &res.text().await.unwrap().len();
                        Some(format!(
                            "/{} (Status: {} | Content-Length: {})",
                            &s, status, len
                        ))
                    } else {
                        Some(format!("/{} (Status: {})", &s, &res.status(),))
                    }
                } else {
                    None
                }
            })
        })
        .buffer_unordered(PARALLEL_REQUESTS);
    out.for_each(|res| async {
        match res {
            Ok(Some(res)) => println!("{}", res),
            Ok(None) => (),
            Err(e) => eprintln!("Got a tokio::JoinError: {}", e),
        }
    })
    .await;
}
