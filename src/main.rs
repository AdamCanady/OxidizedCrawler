#![feature(phase)]
#[phase(plugin)]
extern crate regex_macros;
extern crate regex;
extern crate hyper;
extern crate url;
extern crate mime;
extern crate getopts;
extern crate conrod; // get this to work

use std::sync::{Arc, Mutex};
use std::vec::Vec;
use std::collections::HashSet;
use std::string::String;
use hyper::client;
use hyper::header::common::ContentType;
use std::io::timer;
use std::time::duration::Duration;
use url::{Url, UrlParser};
use mime::{Mime, TopLevel, SubLevel};
use getopts::{reqopt, optopt, optflag, getopts, OptGroup, usage};
use std::os;

// macro_rules! trycontinue(
//     ($e:expr) => (match $e { Ok(e) => e, Err(e) => continue; })
// )

fn worker(id: int, ql: &mut Arc<Mutex<Vec<String>>>, sl: &mut Arc<Mutex<HashSet<String>>>) {
  let mut empty_number: uint = 0;

  loop {
    let mut q = ql.lock();
    let queue_is_empty = (*q).is_empty();

    if queue_is_empty {
      drop(q);
      println!("Thread: {} Queue empty, waiting...", id);
      timer::sleep(Duration::milliseconds(2000));
      empty_number += 1;

      if empty_number > 5 {
        return;
      }
      continue;
    }

    empty_number = 0;

    let url = (*q).remove(0).unwrap();
    let str_url = url.as_slice();

    println!("Thread: {}, found url: {}", id, str_url);

    drop(q);

    // Do request
    let req = hyper::client::Request::get(hyper::Url::parse(str_url).unwrap()).unwrap();
    let mut res_opt = req.start().unwrap().send();
    if res_opt.is_err() {
      timer::sleep(Duration::milliseconds(200));
      continue;
    }

    let mut res = res_opt.unwrap();

    if !(res.headers.has::<ContentType>()) {
      continue;
    }

    match **res.headers.get::<ContentType>().unwrap() {
      Mime(_, SubLevel::Html, _) => (),
      _ => continue
    }
    
    let html = res.read_to_string().unwrap();
    let str_html = html.as_slice();

    let links_re = regex!("href=[\'\"]?([^\'\" >]+)");

    for cap in links_re.captures_iter(str_html) {
      let base_url = Url::parse(url.as_slice().clone()).unwrap();
      let base_domain = base_url.domain();
      let url_link = UrlParser::new().base_url(&base_url).parse(cap.at(1)).unwrap();
      let string_link = url_link.serialize_no_fragment();
      let link = string_link.as_slice();
      let link_base_domain = url_link.domain();

      if (link_base_domain != base_domain) {
        continue;
      }

      let mut s = sl.lock();
      let link_exists_in_set = (*s).contains(link);
      drop(s);

      let string_link = String::from_str(link);

      if !link_exists_in_set {
        let mut s = sl.lock();
        (*s).insert(string_link.clone());
        drop(s);

        let mut q = ql.lock();
        (*q).push(string_link.clone());
        drop(q);
      }
    }
    timer::sleep(Duration::milliseconds(200));
  }
}

fn make_workers(q: &mut Arc<Mutex<Vec<String>>>, s: &mut Arc<Mutex<HashSet<String>>>) {
  for i in range(0,50) {
    let mut q_clone = q.clone();
    let mut s_clone = s.clone();

    spawn(proc() {
      worker(i, &mut q_clone, &mut s_clone);
    });
  }
}

fn print_usage(program: &str, opts: &[OptGroup]) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", usage(brief.as_slice(), opts));
}

fn main() {
  let args: Vec<String> = os::args();

  let program = args[0].clone();

  let opts = &[
    optopt("o", "", "set output file name", "NAME"),
    optflag("h", "help", "print this help menu")
  ];

  let matches = match getopts(args.tail(), opts) {
    Ok(m) => { m }
    Err(f) => { panic!(f.to_string()) }
  };

  if matches.opt_present("h") {
    print_usage(program.as_slice(), opts);
    return;
  }

  let starting_url = if(!matches.free.is_empty()){
    matches.free[0].as_slice()
  } else {
    print_usage(program.as_slice(), opts);
    return
  };

  let mut q = Arc::new(Mutex::new(Vec::<String>::new()));
  let mut s = Arc::new(Mutex::new(HashSet::<String>::new()));

  {
    let mut qq = q.lock();
    (*qq).push(String::from_str(starting_url));
  }

  make_workers(&mut q, &mut s);
}
