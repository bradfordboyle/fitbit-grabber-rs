extern crate chrono;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate tokio_core;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use chrono::{NaiveDate, Utc};
use chrono::format::ParseError;
use futures::{Future, Stream};
use hyper::{Client, Method, Request, Uri};
use hyper::error::UriError;
use hyper::header::{Authorization, Bearer};
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;

fn main() {
    let start = start_arg(env::args()).expect("error parsing start date");
    let dates = DateRange::from(start);

    for d in dates {
        activities_heart(&d).expect("error fetching data for date")
    }
}

fn start_arg(mut argv: env::Args) -> Result<NaiveDate, String> {
    argv.nth(1)
        .ok_or("please give a starting date".to_string())
        .and_then(|arg| NaiveDate::parse_from_str(&arg, "%Y-%m-%d").map_err(err_handler))
}

fn activities_heart(date: &NaiveDate) -> Result<(), String> {
    let token = load_token(".token");

    let url = format!("https://api.fitbit.com/1/user/-/activities/heart/date/{}/1d.json",
                      format!("{}", date.format("%Y-%m-%d")));
    // TODO read from config file
    // let url = format!("http://localhost:8080/{}/1d.json", date);
    let uri = url.parse::<Uri>().map_err(|err: UriError| String::from(err.description()))?;

    let mut req = Request::new(Method::Get, uri);
    req.headers_mut().set(Authorization(Bearer { token: token.to_string() }));


    let mut core = Core::new().map_err(err_handler)?;
    let handle = core.handle();
    let connector = HttpsConnector::new(4, &handle).map_err(err_handler)?;
    let client = Client::configure().connector(connector).build(&handle);

    let filename = format!("{}.json", date);
    let path = Path::new(&filename);
    let mut file = File::create(&path).map_err(err_handler)?;

    let work = client.request(req).and_then(|res| {
        println!("Response: {}", res.status());


        println!("Writing to {}", path.display());
        res.body().for_each(|chunk| file.write_all(&chunk).map_err(From::from))
    });

    core.run(work).map_err(err_handler)?;
    Ok(())
}

fn load_token(filename: &str) -> String {
    let mut f = File::open(filename).expect("file not found");
    let mut contents = String::new();
    f.read_to_string(&mut contents).expect("unable to read file");

    contents.trim().to_string()
}

fn err_handler<E: Error>(err: E) -> String {
    String::from(err.description())
}

struct DateRange {
    start: NaiveDate,
    end: NaiveDate,
}

impl DateRange {
    fn new(start: &str, end: &str) -> Result<DateRange, ParseError> {
        let start = NaiveDate::parse_from_str(start, "%Y-%m-%d")?;
        let end = NaiveDate::parse_from_str(end, "%Y-%m-%d")?;
        Ok(DateRange {
               start: start,
               end: end,
           })
    }

    fn from(start: NaiveDate) -> DateRange {
        DateRange {
            start: start,
            end: Utc::today().naive_utc(),
        }
    }
}

impl Iterator for DateRange {
    type Item = NaiveDate;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            None
        } else {
            let curr = self.start;
            self.start = curr.succ();
            Some(curr)
        }
    }
}

#[cfg(test)]
mod tests {
    use DateRange;

    use chrono::{NaiveDate, Utc};

    #[test]
    fn daterange() {
        let d = DateRange::new("2017-09-01", "2017-09-30").unwrap();
        assert_eq!(d.start, NaiveDate::from_ymd(2017, 9, 1));
        assert_eq!(d.end, NaiveDate::from_ymd(2017, 9, 30));
    }

    #[test]
    fn daterange_from() {
        let d = DateRange::from(NaiveDate::from_ymd(2017, 9, 1));
        assert_eq!(d.start, NaiveDate::from_ymd(2017, 9, 1));
        assert_eq!(d.end, Utc::today().naive_utc());
    }

    #[test]
    fn daterange_iter() {
        let d = DateRange::new("2017-09-01", "2017-09-03").unwrap();
        let dates: Vec<NaiveDate> = d.collect();
        assert_eq!(dates,
                   vec![NaiveDate::from_ymd(2017, 9, 1),
                        NaiveDate::from_ymd(2017, 9, 2),
                        NaiveDate::from_ymd(2017, 9, 3)])
    }
}
