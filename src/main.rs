mod ai;

use ai::little_guy;
use dotenvy::dotenv;
use openai::chat::{ChatCompletionMessage, ChatCompletionMessageRole};
use serenity::futures::StreamExt;
use serenity::prelude::GatewayIntents;
use std::env;
use std::fs::OpenOptions;
use std::io::prelude::*;
use tokio_cron_scheduler::{Job, JobScheduler};

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use crate::ai::little_guy_greeter;

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
        {
            msg.react(&ctx.http, '👀').await.unwrap();

            let mut channel_messages = msg.channel_id.messages_iter(&ctx.http).boxed();
            let mut completion_messages = Vec::new();
            while let Some(Ok(channel_message)) = channel_messages.next().await {
                if channel_message.author.bot {
                    completion_messages.push(ChatCompletionMessage {
                        role: ChatCompletionMessageRole::Assistant,
                        content: channel_message.content,
                        name: None,
                    });
                } else {
                    completion_messages.push(ChatCompletionMessage {
                        role: ChatCompletionMessageRole::User,
                        content: channel_message.content.replace("<@598740888562302977>", ""),
                        name: None,
                    });
                }

                if completion_messages.len() > 3 {
                    break;
                }
            }
            completion_messages.reverse();

            let response = little_guy(&completion_messages).await;

            if let Ok(response) = response {
                if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
                    println!("Error sending message: {:?}", why);
                }
            } else {
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

    async fn ready(&self, context: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let scheduler = JobScheduler::new()
            .await
            .expect("Could not create scheduler");
        let job = Job::new_async("0 0 9 * * * *", move |_, _| {
            let http = context.http.clone();
            Box::pin(async move {
                println!("Computing greeting");
                // Read previous greetings from file at `./greetings.txt`
                let previous_greetings = std::fs::read_to_string("./greetings.txt")
                    .unwrap_or_else(|_| String::new())
                    .split('\n')
                    .map(|line| line.to_string())
                    .collect::<Vec<String>>();

                let greeting = little_guy_greeter(&previous_greetings)
                    .await
                    .expect("Could not compute greeting");

                println!("Sending greeting");
                http.get_channel(598744899030089728)
                    .await
                    .unwrap()
                    .id()
                    .say(&http, greeting.clone())
                    .await
                    .unwrap();

                let mut file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open("./greetings.txt")
                    .unwrap();
                writeln!(file, "{}", greeting).expect("Could not add greeting to the file");
            })
        })
        .expect("Could not create job");
        scheduler.add(job).await;
        println!("Starting greeting schedule");
        scheduler.start().await;
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
