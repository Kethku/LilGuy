mod ai;
mod classifiers;

use ai::{astro, greeter};
use classifiers::is_instruction;
use dotenvy::dotenv;
use serenity::prelude::GatewayIntents;
use std::env;
use tokio_cron_scheduler::{Job, JobScheduler};

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.mentions.iter().any(|user| user.bot)
            || msg
                .referenced_message
                .as_ref()
                .map(|message| message.author.bot)
                .unwrap_or_default()
            || (msg.content.to_lowercase().contains("astro") && !msg.author.bot)
        {
            msg.react(&ctx.http, '👀').await.unwrap();

            if msg.channel_id == 598338172958670862 {
                // react with a shooshing face
                msg.react(&ctx.http, '🤫').await.unwrap();
                return;
            }

            if msg.content.contains("transcript") {
                msg.react(&ctx.http, '📜').await.unwrap();
                msg.channel_id
                    .say(
                        &ctx.http,
                        std::fs::read_to_string("./astro.txt").unwrap_or_default(),
                    )
                    .await
                    .ok();
                return;
            }

            let response = if msg.content.contains("greet") {
                greeter().await
            } else {
                let instruction = is_instruction(&msg.content).await.unwrap_or_default();
                if instruction {
                    // react with a check mark
                    msg.react(&ctx.http, '✅').await.unwrap();
                };
                astro(&msg.content, instruction).await
            };

            match response {
                Ok(response) => {
                    if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
                        println!("Error sending message: {:?}", why);
                    }
                }
                Err(why) => {
                    println!("Error generating message: {:?}", why);
                    if let Err(why) = msg
                        .channel_id
                        .say(&ctx.http, "I'm sorry, I don't know what to say.")
                        .await
                    {
                        println!("Error sending message: {:?}", why);
                    }
                }
            }
        }
    }

    async fn ready(&self, context: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let scheduler = JobScheduler::new()
            .await
            .expect("Could not create scheduler");
        let job = Job::new_async("0 0 16 * * * *", move |_, _| {
            let http = context.http.clone();
            Box::pin(async move {
                let greeting = greeter().await.expect("Could not compute greeting");

                http.get_channel(598338172958670862)
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
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
