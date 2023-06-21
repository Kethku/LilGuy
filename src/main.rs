#![feature(let_chains)]

mod ai;
mod embeddings;
mod extensions;

use std::env;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use anyhow::Result;
use dotenvy::dotenv;
use serenity::{
    async_trait,
    model::prelude::{Reaction, ReactionType},
    prelude::{*, GatewayIntents},
    model::{channel::Message, gateway::Ready},
};
use tokio_cron_scheduler::{Job, JobScheduler};

use extensions::MessageExt;
use ai::{astro, emoji, greeter, Bot};

struct Handler;

pub static ACTIVE_CONVO: AtomicU64 = AtomicU64::new(0);
pub static ADDED_RULE: AtomicBool = AtomicBool::new(false);

const BRIDGE: u64 = 598338172958670862;
const MUTED_CHANNELS: &[u64] = &[598338172958670862, 636801468011249666];

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let result: Result<()> = (|| async {
            if msg.is_own(&ctx).await? {
                return Ok(());
            }

            if msg.requires_response(&ctx).await? {
                if MUTED_CHANNELS.contains(&msg.channel_id.0) {
                    ACTIVE_CONVO.store(0, Ordering::Relaxed);
                    // react with emoji response
                    let emoji = emoji(&msg.content).await?;
                    if let Some(possible_emoji) = emoji.chars().next() {
                        // Response might not be a valid emoji. Ignore if not
                        msg.react(&ctx.http, possible_emoji)
                            .await
                            .ok();
                    }
                    return Ok(());
                }

                let _typing = msg.channel_id.start_typing(&ctx.http)?;

                ACTIVE_CONVO.store(msg.channel_id.into(), Ordering::Relaxed);
                let response = if msg.content.contains("greet") {
                    greeter().await?
                } else {
                    astro(&msg.content).await?
                };

                msg.reply_maybe_long(&ctx, response).await?;

            } else {
                ACTIVE_CONVO.store(0, Ordering::Relaxed);
            }

            Ok(())
        })().await;

        if let Err(why) = result {
            println!("Error: {:?}", why);
        }
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        let result: Result<()> = (|| async {
            if let Some(user_id) = reaction.user_id {
                if user_id.to_user(&ctx.http).await?.bot {
                    return Ok(());
                }
            } else {
                return Ok(());
            }

            if let ReactionType::Custom { name, .. } = &reaction.emoji
                && name == &Some("purge".to_string()) {

                let mut purge_limit = 5;
                if let Ok(channel) = reaction.channel_id.to_channel(&ctx).await {
                    if channel.guild().and_then(|guild_channel| guild_channel.parent_id).map(|parent_id| parent_id != 598667330826010624).unwrap_or_default() {
                        purge_limit = 3;
                    }
                } 

                if reaction.users::<_, u64>(&ctx.http, reaction.emoji.clone(), None, None).await.unwrap().len() >= purge_limit  {
                    reaction.channel_id.delete_message(&ctx.http, reaction.message_id).await?;
                }
            }

            // Hammer react should use the message to mutate astro's identity
            if reaction.emoji == ReactionType::Unicode("🔨".to_string()) {
                let msg = reaction.message(&ctx.http).await?;

                if msg.channel_id == BRIDGE {
                    if !ADDED_RULE.load(Ordering::Relaxed) {
                        let bot = Bot::open("greeter")
                            .with_rule(&msg.content).save()?;
                        msg.react(&ctx.http, ReactionType::Unicode("🔨".to_string()))
                            .await?;
                        ADDED_RULE.store(true, Ordering::Relaxed);
                    }
                } else if !MUTED_CHANNELS.contains(&msg.channel_id.0) {
                    Bot::open("astro")
                        .with_identity(&msg.content)
                        .save()?;
                    msg.react(&ctx.http, ReactionType::Unicode("🔨".to_string()))
                        .await?;
                }
            }

            // If a bot message has a mind blown reaction, reset the bot
            if reaction.emoji == ReactionType::Unicode("🤯".to_string()) {
                let msg = reaction.message(&ctx.http).await?;
                if !msg.author.bot {
                    return Ok(());
                }

                Bot::open("astro").reset().save()?;

                msg.react(&ctx.http, ReactionType::Unicode("🤯".to_string()))
                    .await?;
            }

            if reaction.emoji == ReactionType::Unicode("🤖".to_string()) {
                let msg = reaction.message(&ctx.http).await?;
                let greeting = greeter().await.expect("Could not compute greeting");
                msg.reply_maybe_long(&ctx, greeting).await?;
            }

            Ok(())
        })().await;

        if let Err(why) = result {
            println!("Error: {:?}", why);
        }
    }

    async fn ready(&self, context: Context, _ready: Ready) {
        println!("Connected!");

        let scheduler = JobScheduler::new()
            .await
            .expect("Could not create scheduler");
        let job = Job::new_async("0 0 14 * * * *", move |_, _| {
            let http = context.http.clone();
            Box::pin(async move {
                ADDED_RULE.store(false, Ordering::Relaxed);
                let greeting = greeter().await.expect("Could not compute greeting");

                http.get_channel(BRIDGE)
                    .await
                    .unwrap()
                    .id()
                    .say(&http, greeting.clone())
                    .await
                    .unwrap();
            })
        })
        .expect("Could not create job");
        scheduler
            .add(job)
            .await
            .expect("Could not add job to schedule");
        scheduler
            .start()
            .await
            .expect("Could not start job scheduler");
    }
}

#[tokio::main]
async fn main() {
    dotenv().unwrap();
    ai::auth();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::DIRECT_MESSAGE_REACTIONS
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
