#![feature(proc_macro, generators)]

#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate error_chain;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate quick_xml;
extern crate curl;
extern crate futures_await as futures;
extern crate tokio_core;
extern crate tokio_curl;
extern crate telebot;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate pinyin_order;
extern crate chrono;

use tokio_core::reactor::Core;
use futures::Stream;

mod errors;
mod feed;
mod data;
mod utlis;
mod cmdhandles;
mod fetcher;
mod checker;
mod opml;

fn main() {
	let args: Vec<String> = std::env::args().collect();
	if args.len() < 3 {
		eprintln!("usage: ./rustrssbot ${DATAFILE-name} ${TELEGRAM-BOT-TOKEN}", args[0]);
		std::process::exit(1);
	}
	let datafile = &args[1];
	let token = &args[2];
	let period = args.get(3)
		.map(|s| {
			s.parse().unwrap_or_else(|_| {
				eprintln!("period must be unsigned");
				std::process::exit(1);
			})
		})
		.unwrap_or(3600);

	let db = data::Database::open(datafile)
		.map_err(|e| {
			eprintln!("error: {}", e);
			for e in e.iter().skip(1) {
				eprintln!("caused by: {}", e);
			}
			if let Some(backtrace) = e.backtrace() {
				eprintln!("backtrace: {:?}", backtrace);
			}
			std::process::exit(1);
		})
		.unwrap();

	env_logger::init().unwrap();

	let mut lp = Core::new().unwrap();
	let lphandle = lp.handle();
	let bot = lp.run(telebot::RcBot::new(lphandle, token))
		.expect("failed to initialize bot")
		.update_interval(200);

	cmdhandles::register_commands(&bot, &db, lp.handle());

	fetcher::spawn_fetcher(bot.clone(), db.clone(), period);

	checker::spawn_subscriber_alive_checker(bot.clone(), db, lp.handle());

	let s = bot.get_stream()
		.map(|_| ())
		.or_else(|e| {
			error!("telebot: {:?}", e);
			Ok::<(), ()>(())
		})
		.for_each(|_| Ok(()));
	lp.run(s).unwrap();
}
