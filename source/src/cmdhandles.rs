use telebot::functions::*;
use tokio_core::reactor::Handle;
use futures::future;
use futures::prelude::*;
use tokio_curl::Session;
use telebot;
use telebot::functions::File;
use pinyin_order;

use errors::*;
use feed;
use utlis::{Escape, EscapeUrl, send_multiple_messages, format_and_split_msgs, to_chinese_error_msg, log_error, gen_ua};
use data::Database;
use opml::to_opml;

pub fn register_commands(bot: &telebot::RcBot, db: &Database, lphandle: Handle) {
	register_rss(bot, db.clone());
	register_sub(bot, db.clone(), lphandle);
	register_unsub(bot, db.clone());
	register_unsubthis(bot, db.clone());
	register_export(bot, db.clone());
}

fn register_rss(bot: &telebot::RcBot, db: Database) {
	let handle = bot.new_cmd("/list")
		.map_err(Some)
		.and_then(move |(bot, msg)| {
			let text = msg.text.unwrap();
			let args: Vec<&str> = text.split_whitespace().collect();
			let raw: bool;
			let subscriber: future::Either<_, _>;
			match args.len() {
				0 => {
					raw = false;
					subscriber = future::Either::A(future::ok(Some(msg.chat.id)));
				}
				1 => {
					if args[0] == "display-url" {
						raw = true;
						subscriber = future::Either::A(future::ok(Some(msg.chat.id)));
					} else {
						raw = false;
						let channel = args[0];
						let channel_id =
							check_channel(&bot, channel, msg.chat.id, msg.from.unwrap().id);
						subscriber = future::Either::B(channel_id);
					}
				}
				2 => {
					raw = true;
					let channel = args[0];
					let channel_id =
						check_channel(&bot, channel, msg.chat.id, msg.from.unwrap().id);
					subscriber = future::Either::B(channel_id);
				}
				_ => {
					let r = bot.message(
						msg.chat.id,
						"usage: /list (${Channel-Username}) (display-url)".to_string(),
					).send()
						.then(|result| match result {
							Ok(_) => Err(None),
							Err(e) => Err(Some(e)),
						});
					return future::Either::A(r);
				}
			}
			let db = db.clone();
			let chat_id = msg.chat.id;
			let r = subscriber
				.then(|result| match result {
					Ok(Some(ok)) => Ok(ok),
					Ok(None) => Err(None),
					Err(err) => Err(Some(err)),
				})
				.map(move |subscriber| (bot, db, subscriber, raw, chat_id));
			future::Either::B(r)
		})
		.and_then(|(bot, db, subscriber, raw, chat_id)| {
			match db.get_subscribed_feeds(subscriber) {
				Some(feeds) => Ok((bot, raw, chat_id, feeds)),
				None => Err((bot, chat_id)),
			}.into_future()
				.or_else(|(bot, chat_id)| {
					bot.message(chat_id, "you subscribe nothing.".to_string())
						.send()
						.then(|r| match r {
							Ok(_) => Err(None),
							Err(e) => Err(Some(e)),
						})
				})
		})
		.and_then(|(bot, raw, chat_id, mut feeds)| {
			let text = String::from("subscribe list:");
			if !raw {
				feeds.sort_by_key(|feed| pinyin_order::as_pinyin(&feed.title));
				let msgs = format_and_split_msgs(text, &feeds, |feed| {
					format!(
						"<a href=\"{}\">{}</a>",
						EscapeUrl(&feed.link),
						Escape(&feed.title)
					)
				});
				send_multiple_messages(&bot, chat_id, msgs)
			} else {
				feeds.sort_by(|a, b| a.link.cmp(&b.link));
				let msgs = format_and_split_msgs(text, &feeds, |feed| {
					format!("{}: {}", Escape(&feed.title), Escape(&feed.link))
				});
				send_multiple_messages(&bot, chat_id, msgs)
			}.map_err(Some)
		})
		.then(|result| match result {
			Err(Some(err)) => {
				error!("telebot: {:?}", err);
				Ok::<(), ()>(())
			}
			_ => Ok(()),
		});

	bot.register(handle);
}

fn register_sub(bot: &telebot::RcBot, db: Database, lphandle: Handle) {
	let handle = bot.new_cmd("/subscribe")
		.map_err(Some)
		.and_then(move |(bot, msg)| {
			let text = msg.text.unwrap();
			let args: Vec<&str> = text.split_whitespace().collect();
			let feed_link: &str;
			let subscriber: future::Either<_, _>;
			match args.len() {
				1 => {
					feed_link = args[0];
					subscriber = future::Either::A(future::ok(Some(msg.chat.id)));
				}
				2 => {
					let channel = args[0];
					let channel_id =
						check_channel(&bot, channel, msg.chat.id, msg.from.unwrap().id);
					subscriber = future::Either::B(channel_id);
					feed_link = args[1];
				}
				_ => {
					let r = bot.message(
						msg.chat.id,
						"usage: /subscribe (${Channel-Username}) ${rss-url}".to_string(),
					).send()
						.then(|result| match result {
							Ok(_) => Err(None),
							Err(e) => Err(Some(e)),
						});
					return future::Either::A(r);
				}
			}
			let db = db.clone();
			let feed_link = feed_link.to_owned();
			let chat_id = msg.chat.id;
			let lphandle = lphandle.clone();
			let r = subscriber
				.then(|result| match result {
					Ok(Some(ok)) => Ok(ok),
					Ok(None) => Err(None),
					Err(err) => Err(Some(err)),
				})
				.map(move |subscriber| {
					(bot, db, subscriber, feed_link, chat_id, lphandle)
				});
			future::Either::B(r)
		})
		.and_then(|(bot, db, subscriber, feed_link, chat_id, lphandle)| {
			if db.is_subscribed(subscriber, &feed_link) {
				Err((bot, chat_id))
			} else {
				Ok((bot, db, subscriber, feed_link, chat_id, lphandle))
			}.into_future()
				.or_else(|(bot, chat_id)| {
					bot.message(chat_id, "you've already subscribed this.".to_string())
						.send()
						.then(|result| match result {
							Ok(_) => Err(None),
							Err(e) => Err(Some(e)),
						})
				})
		})
		.and_then(|(bot, db, subscriber, feed_link, chat_id, lphandle)| {
			bot.message(chat_id, "dealing...".to_owned())
				.send()
				.map_err(Some)
				.map(move |(bot, msg)| {
					(
						bot,
						db,
						subscriber,
						feed_link,
						chat_id,
						msg.message_id,
						lphandle,
					)
				})
		})
		.and_then(|(bot,
		  db,
		  subscriber,
		  feed_link,
		  chat_id,
		  msg_id,
		  lphandle)| {
			let session = Session::new(lphandle);
			let bot2 = bot.clone();
			feed::fetch_feed(session, gen_ua(&bot), feed_link)
				.map(move |feed| (bot2, db, subscriber, chat_id, msg_id, feed))
				.or_else(move |e| {
					bot.edit_message_text(
						chat_id,
						msg_id,
						format!("subscribe failed : {}", to_chinese_error_msg(e)),
					).send()
						.then(|result| match result {
							Ok(_) => Err(None),
							Err(e) => Err(Some(e)),
						})
				})
		})
		.and_then(|(bot, db, subscriber, chat_id, msg_id, feed)| {
			let source = feed.source.as_ref().unwrap();
			match db.subscribe(subscriber, source, &feed) {
				Ok(_) => {
					bot.edit_message_text(
						chat_id,
						msg_id,
						format!(
							"《<a href=\"{}\">{}</a>》subscribe success !",
							EscapeUrl(source),
							Escape(&feed.title)
						),
					).parse_mode("HTML")
						.disable_web_page_preview(true)
						.send()
				}
				Err(Error(ErrorKind::AlreadySubscribed, _)) => {
					bot.edit_message_text(chat_id, msg_id, "you've already subscribed this.".to_string())
						.send()
				}
				Err(e) => {
					log_error(&e);
					bot.edit_message_text(chat_id, msg_id, format!("error: {}", e))
						.send()
				}
			}.map_err(Some)
		})
		.then(|result| match result {
			Err(Some(err)) => {
				error!("telebot: {:?}", err);
				Ok::<(), ()>(())
			}
			_ => Ok(()),
		});

	bot.register(handle);
}

fn register_unsub(bot: &telebot::RcBot, db: Database) {
	let handle = bot.new_cmd("/unsubscribe")
		.map_err(Some)
		.and_then(move |(bot, msg)| {
			let text = msg.text.unwrap();
			let args: Vec<&str> = text.split_whitespace().collect();
			let feed_link: &str;
			let subscriber: future::Either<_, _>;
			match args.len() {
				1 => {
					feed_link = args[0];
					subscriber = future::Either::A(future::ok(Some(msg.chat.id)));
				}
				2 => {
					let channel = args[0];
					let channel_id =
						check_channel(&bot, channel, msg.chat.id, msg.from.unwrap().id);
					subscriber = future::Either::B(channel_id);
					feed_link = args[1];
				}
				_ => {
					let r = bot.message(
						msg.chat.id,
						"usage: /unsubscribe (${Channel-Username}) ${rss-url}".to_string(),
					).send()
						.then(|result| match result {
							Ok(_) => Err(None),
							Err(e) => Err(Some(e)),
						});
					return future::Either::A(r);
				}
			}
			let db = db.clone();
			let feed_link = feed_link.to_owned();
			let chat_id = msg.chat.id;
			let r = subscriber
				.then(|result| match result {
					Ok(Some(ok)) => Ok(ok),
					Ok(None) => Err(None),
					Err(err) => Err(Some(err)),
				})
				.map(move |subscriber| (bot, db, subscriber, feed_link, chat_id));
			future::Either::B(r)
		})
		.and_then(|(bot, db, subscriber, feed_link, chat_id)| {
			match db.unsubscribe(subscriber, &feed_link) {
				Ok(feed) => {
					bot.message(
						chat_id,
						format!(
							"《<a href=\"{}\">{}</a>》unsubscribe success !",
							EscapeUrl(&feed.link),
							Escape(&feed.title)
						),
					).parse_mode("HTML")
						.disable_web_page_preview(true)
						.send()
				}
				Err(Error(ErrorKind::NotSubscribed, _)) => {
					bot.message(chat_id, "you do not subscribe this.".to_string())
						.send()
				}
				Err(e) => {
					log_error(&e);
					bot.message(chat_id, format!("error: {}", e)).send()
				}
			}.map_err(Some)
		})
		.then(|result| match result {
			Err(Some(err)) => {
				error!("telebot: {:?}", err);
				Ok::<(), ()>(())
			}
			_ => Ok(()),
		});

	bot.register(handle);
}

fn register_unsubthis(bot: &telebot::RcBot, db: Database) {
	let handle = bot.new_cmd("/unsubscribethis")
		.map_err(Some)
		.and_then(move |(bot, msg)| {
			if let Some(reply_msg) = msg.reply_to_message {
				Ok((bot, db.clone(), msg.chat.id, reply_msg))
			} else {
				Err((bot, msg.chat.id))
			}.into_future()
				.or_else(|(bot, chat_id)| {
					bot.message(
						chat_id,
						"usage: \
						 use this directive to reply to RSS messege to unsubscribe that,\
						 not support Channel"
							.to_string(),
					).send()
						.then(|result| match result {
							Ok(_) => Err(None),
							Err(e) => Err(Some(e)),
						})
				})

		})
		.and_then(|(bot, db, chat_id, reply_msg)| {
			if let Some(m) = reply_msg.text {
				if let Some(title) = m.lines().next() {
					Ok((bot, db, chat_id, title.to_string()))
				} else {
					Err((bot, chat_id))
				}
			} else {
				Err((bot, chat_id))
			}.into_future()
				.or_else(|(bot, chat_id)| {
					bot.message(chat_id, "unknown messege".to_string())
						.send()
						.then(|result| match result {
							Ok(_) => Err(None),
							Err(e) => Err(Some(e)),
						})
				})
		})
		.and_then(|(bot, db, chat_id, title)| {
			if let Some(feed_link) = db.get_subscribed_feeds(chat_id)
				.unwrap_or_default()
				.iter()
				.filter(|feed| feed.title == title)
				.map(|feed| feed.link.clone())
				.next()
			{
				Ok((bot, db, chat_id, feed_link))
			} else {
				Err((bot, chat_id))
			}.into_future()
				.or_else(|(bot, chat_id)| {
					bot.message(chat_id, "can not find the subscription".to_string())
						.send()
						.then(|result| match result {
							Ok(_) => Err(None),
							Err(e) => Err(Some(e)),
						})
				})
		})
		.and_then(|(bot, db, chat_id, feed_link)| {
			match db.unsubscribe(chat_id, &feed_link) {
				Ok(feed) => {
					bot.message(
						chat_id,
						format!(
							"《<a href=\"{}\">{}</a>》unsubscribe successfully",
							EscapeUrl(&feed.link),
							Escape(&feed.title)
						),
					).parse_mode("HTML")
						.disable_web_page_preview(true)
						.send()
				}
				Err(e) => {
					log_error(&e);
					bot.message(chat_id, format!("error: {}", e)).send()
				}
			}.map_err(Some)
		})
		.then(|result| match result {
			Err(Some(err)) => {
				error!("telebot: {:?}", err);
				Ok::<(), ()>(())
			}
			_ => Ok(()),
		});

	bot.register(handle);
}

fn check_channel<'a>(
	bot: &telebot::RcBot,
	channel: &str,
	chat_id: i64,
	user_id: i64,
) -> impl Future<Item = Option<i64>, Error = telebot::Error> + 'a {
	let channel = channel
		.parse::<i64>()
		.map(|_| if !channel.starts_with("-100") {
			format!("-100{}", channel)
		} else {
			channel.to_owned()
		})
		.unwrap_or_else(|_| if !channel.starts_with('@') {
			format!("@{}", channel)
		} else {
			channel.to_owned()
		});
	let bot = bot.clone();
	async_block! {
		let msg = await!(bot.message(chat_id, "checking Channel".to_string()).send())?.1;
		let msg_id = msg.message_id;
		let channel = match await!(bot.get_chat(channel).send()) {
			Ok((_, channel)) => channel,
			Err(telebot::Error::Telegram(_, err_msg, _)) => {
				let msg = format!("can not find the Channel: {}", err_msg);
				await!(bot.edit_message_text(chat_id, msg_id, msg).send())?;
				return Ok(None);
			}
			Err(e) => return Err(e),
		};
		if channel.kind != "channel" {
			let msg = "the target must be Channel".to_string();
			await!(bot.edit_message_text(chat_id, msg_id, msg).send())?;
			return Ok(None);
		}
		let channel_id = channel.id;

		let admins_list = match await!(bot.get_chat_administrators(channel_id).send()) {
			Ok((_, admins)) => admins
				.iter()
				.map(|member| member.user.id)
				.collect::<Vec<i64>>(),
			Err(telebot::Error::Telegram(_, err_msg, _)) => {
				let msg = format!("you should set Bot as this Channel's admin : {}", err_msg);
				await!(bot.edit_message_text(chat_id, msg_id, msg).send())?;
				return Ok(None);
			}
			Err(e) => return Err(e),
		};

		if !admins_list.contains(&bot.inner.id) {
			let msg = "should set Bot as admin.".to_string();
			await!(bot.edit_message_text(chat_id, msg_id, msg).send())?;
			return Ok(None);
		}

		if !admins_list.contains(&user_id) {
			let msg = "this command only available for Channel's admin.".to_string();
			await!(bot.edit_message_text(chat_id, msg_id, msg).send())?;
			return Ok(None);
		}

		await!(bot.delete_message(chat_id, msg_id).send())?;

		Ok(Some(channel_id))
	}
}

fn register_export(bot: &telebot::RcBot, db: Database) {
	let handle = bot.new_cmd("/exporttoopml")
		.map_err(Some)
		.and_then(move |(bot, msg)| {
			let text = msg.text.unwrap();
			let args: Vec<&str> = text.split_whitespace().collect();
			let subscriber: future::Either<_, _>;
			match args.len() {
				0 => {
					subscriber = future::Either::A(future::ok(Some(msg.chat.id)));
				}
				1 => {
					let channel = args[0];
					let channel_id =
						check_channel(&bot, channel, msg.chat.id, msg.from.unwrap().id);
					subscriber = future::Either::B(channel_id);
				}
				_ => {
					let r = bot.message(
						msg.chat.id,
						"usage: /export <Channel ID>".to_string(),
					).send()
						.then(|result| match result {
							Ok(_) => Err(None),
							Err(e) => Err(Some(e)),
						});
					return future::Either::A(r);
				}
			}
			let db = db.clone();
			let chat_id = msg.chat.id;
			let r = subscriber
				.then(|result| match result {
					Ok(Some(ok)) => Ok(ok),
					Ok(None) => Err(None),
					Err(err) => Err(Some(err)),
				})
				.map(move |subscriber| (bot, db, subscriber, chat_id));
			future::Either::B(r)
		})
		.and_then(|(bot, db, subscriber, chat_id)| {
			match db.get_subscribed_feeds(subscriber) {
				Some(feeds) => Ok((bot, chat_id, feeds)),
				None => Err((bot, chat_id)),
			}.into_future()
				.or_else(|(bot, chat_id)| {
					bot.message(chat_id, "the subscription list is empty.".to_string())
						.send()
						.then(|r| match r {
							Ok(_) => Err(None),
							Err(e) => Err(Some(e)),
						})
				})
		})
		.and_then(|(bot, chat_id, feeds)| {
			bot.document(
				chat_id,
				File::new("feeds.opml".into(), to_opml(feeds).into_bytes()),
			).send()
				.map_err(Some)
		})
		.then(|result| match result {
			Err(Some(err)) => {
				error!("telebot: {:?}", err);
				Ok::<(), ()>(())
			}
			_ => Ok(()),
		});

	bot.register(handle);
}
