use telebot;
use telebot::functions::*;
use futures::{self, Future, Stream};

use errors;

pub const TELEGRAM_MAX_MSG_LEN: usize = 4096;

pub struct Escape<'a>(pub &'a str);

impl<'a> ::std::fmt::Display for Escape<'a> {
	fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
		// https://core.telegram.org/bots/api#html-style
		let Escape(s) = *self;
		let pile_o_bits = s;
		let mut last = 0;
		for (i, ch) in s.bytes().enumerate() {
			match ch as char {
				'<' | '>' | '&' | '"' => {
					fmt.write_str(&pile_o_bits[last..i])?;
					let s = match ch as char {
						'>' => "&gt;",
						'<' => "&lt;",
						'&' => "&amp;",
						'"' => "&quot;",
						_ => unreachable!(),
					};
					fmt.write_str(s)?;
					last = i + 1;
				}
				_ => {}
			}
		}

		if last < s.len() {
			fmt.write_str(&pile_o_bits[last..])?;
		}
		Ok(())
	}
}

pub struct EscapeUrl<'a>(pub &'a str);

impl<'a> ::std::fmt::Display for EscapeUrl<'a> {
	fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
		// https://core.telegram.org/bots/api#html-style
		let EscapeUrl(s) = *self;
		let pile_o_bits = s;
		let mut last = 0;
		for (i, ch) in s.bytes().enumerate() {
			match ch as char {
				'<' | '>' | '"' => {
					fmt.write_str(&pile_o_bits[last..i])?;
					let s = match ch as char {
						'>' => "%3E",
						'<' => "%3C",
						'"' => "%22",
						_ => unreachable!(),
					};
					fmt.write_str(s)?;
					last = i + 1;
				}
				_ => {}
			}
		}

		if last < s.len() {
			fmt.write_str(&pile_o_bits[last..])?;
		}
		Ok(())
	}
}

pub fn send_multiple_messages<'a>(
	bot: &telebot::RcBot,
	target: i64,
	messages: Vec<String>,
) -> impl Future<Item = (), Error = telebot::Error> + 'a {
	let bot = bot.clone();
	futures::stream::iter_ok(messages).for_each(move |msg| {
		bot.message(target, msg)
			.parse_mode("HTML")
			.disable_web_page_preview(true)
			.send()
			.map(|_| ())
	})
}

pub fn truncate_message(s: &str, max: usize) -> String {
	if s.chars().count() > max {
		format!("{:.1$}...", s, max - 3)
	} else {
		s.to_owned()
	}
}

pub fn format_and_split_msgs<T, F>(head: String, data: &[T], line_format_fn: F) -> Vec<String>
where
	F: Fn(&T) -> String,
{
	let mut msgs = vec![head];
	for item in data {
		let line = line_format_fn(item);
		if msgs.last_mut().unwrap().len() + line.len() > TELEGRAM_MAX_MSG_LEN {
			msgs.push(line);
		} else {
			let msg = msgs.last_mut().unwrap();
			msg.push('\n');
			msg.push_str(&line);
		}
	}
	msgs
}

pub fn to_chinese_error_msg(e: errors::Error) -> String {
	match e {
		errors::Error(errors::ErrorKind::Curl(e), _) => {
			format!("network error: ({})", e.into_error())
		}
		errors::Error(errors::ErrorKind::Utf8(e), _) => format!("encode error: ({})", e),
		errors::Error(errors::ErrorKind::Xml(e), _) => {
			let s = e.to_string();
			let msg = truncate_message(&s, 500);
			format!("resolve error: ({})", msg)
		}
		_ => format!("{}", e),
	}
}

pub fn chat_is_unavailable(e: &telebot::error::Error) -> bool {
	match *e {
		telebot::error::Error::Telegram(_, ref s, _)
			if s.contains("Forbidden") || s.contains("chat not found") ||
				   s.contains("group chat was migrated to a supergroup chat") => true,
		_ => false,
	}
}

pub fn log_error(e: &errors::Error) {
	warn!("error: {}", e);
	for e in e.iter().skip(1) {
		warn!("caused by: {}", e);
	}
	if let Some(backtrace) = e.backtrace() {
		warn!("backtrace: {:?}", backtrace);
	}
}

pub fn gen_ua(bot: &telebot::RcBot) -> String {
	format!(
		concat!(
			env!("CARGO_PKG_NAME"),
			"/",
			env!("CARGO_PKG_VERSION"),
			" (+https://t.me/{})"
		),
		bot.inner.username
	)
}
