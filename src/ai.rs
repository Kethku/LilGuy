use anyhow::Result;
use async_recursion::async_recursion;
use lazy_static::lazy_static;
use openai::{
    chat::{
        ChatCompletion, ChatCompletionFunctionDefinition, ChatCompletionMessage,
        ChatCompletionMessageRole,
    },
    set_key,
};
use serde_json::{json, Value};
use serenity::{model::prelude::Message, prelude::Context};
use std::{
    collections::HashMap,
    env,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::extensions::MessageExt;

pub static ACTIVE_CONVO: AtomicU64 = AtomicU64::new(0);

lazy_static! {
    pub static ref USERS: HashMap<&'static str, usize> = {
        let mut hashmap = HashMap::new();
        hashmap.insert("JonJo", 72878083471847424);
        hashmap.insert("Kay", 143276836481269760);
        hashmap.insert("Andrew", 142518394036420608);
        hashmap.insert("Derek", 142889606260457472);
        hashmap.insert("Chris", 296787008184123393);
        hashmap.insert("Sidd", 143979701960966144);
        hashmap.insert("Kebin", 215221184009076736);
        hashmap.insert("Yves", 279027216459759626);
        hashmap.insert("Daniel", 124634935872061444);
        hashmap.insert("Alex", 144979023146123264);
        hashmap.insert("Evan", 217010389152563202);
        hashmap.insert("Grant", 388148320897335310);
        hashmap.insert("Kevin", 172953593874350081);
        hashmap
    };
}

pub fn auth() {
    set_key(env::var("OPENAI_KEY").unwrap());
}

fn astro_identity() -> String {
    std::fs::read_to_string("identity.txt").unwrap()
}

fn previous_messages() -> Vec<ChatCompletionMessage> {
    std::fs::read_to_string("messages.json")
        .ok()
        .and_then(|messages| serde_json::from_str(&messages).ok())
        .unwrap_or_default()
}

fn append_message(message: &ChatCompletionMessage) {
    let mut previous_messages = previous_messages();
    previous_messages.push(message.clone());

    while previous_messages.len() > 20 {
        previous_messages.remove(0);
    }

    std::fs::write(
        "messages.json",
        serde_json::to_string_pretty(&previous_messages).unwrap(),
    )
    .unwrap();
}

fn opinions() -> HashMap<String, u8> {
    std::fs::read_to_string("opinions.json")
        .ok()
        .and_then(|opinions| serde_json::from_str::<HashMap<String, u8>>(&opinions).ok())
        .unwrap_or_default()
}

fn user_opinion(user_id: u64) -> u8 {
    opinions().get(&user_id.to_string()).cloned().unwrap_or(50)
}

fn increment_user_opinion(user_id: u64) {
    let mut opinions = opinions();
    let user_id = user_id.to_string();
    let new_opinion = (opinions.get(&user_id.to_string()).cloned().unwrap_or(50) + 15).min(100);
    opinions.insert(user_id, new_opinion);

    std::fs::write(
        "opinions.json",
        serde_json::to_string_pretty(&opinions).unwrap(),
    )
    .unwrap();
}

fn decrement_user_opinion(user_id: u64) {
    let mut opinions = opinions();
    let user_id = user_id.to_string();
    let new_opinion = opinions
        .get(&user_id.to_string())
        .cloned()
        .unwrap_or(50)
        .saturating_sub(15);
    opinions.insert(user_id, new_opinion);

    std::fs::write(
        "opinions.json",
        serde_json::to_string_pretty(&opinions).unwrap(),
    )
    .unwrap();
}

pub fn reset() {
    std::fs::write("messages.json", "[]").unwrap();
}

pub async fn respond(
    ctx: &Context,
    message: Message,
    force_call: Option<&'static str>,
) -> Result<()> {
    if !message.requires_response(ctx).await? {
        if ACTIVE_CONVO.load(Ordering::Relaxed) != message.channel_id.0 {
            ACTIVE_CONVO.store(0, Ordering::Relaxed);
            return Ok(());
        }
    }

    let author = message
        .author_nick(&ctx)
        .await
        .unwrap_or(message.author.name.clone());
    let new_message = ChatCompletionMessage {
        role: ChatCompletionMessageRole::User,
        content: Some(format!(
            "{}: {}",
            user_opinion(message.author.id.0),
            message.content.clone()
        )),
        name: Some(author),
        function_call: None,
    };
    // Add message to history
    append_message(&new_message);

    query_model(ctx, &message, force_call).await
}

#[async_recursion]
pub async fn query_model(
    ctx: &Context,
    message: &Message,
    force_call: Option<&'static str>,
) -> Result<()> {
    // React with eyes to indicate the bot saw this.
    message.react(ctx, '🤔').await?;

    // Setup identity
    let mut messages = vec![ChatCompletionMessage {
        role: ChatCompletionMessageRole::System,
        content: Some(astro_identity()),
        name: Some("Astro".to_string()),
        function_call: None,
    }];

    // Add previous messages
    messages.append(&mut previous_messages());

    let chat_completion = ChatCompletion::builder("gpt-3.5-turbo", messages)
        .functions(vec! [
            ChatCompletionFunctionDefinition {
                name: "react".to_string(),
                description: Some("Takes a string with a single emoji and reacts to the last message in the transcript with it.".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "emoji": {
                            "type": "string",
                            "description": "The emoji to react with."
                        }
                    },
                })),
            },
            ChatCompletionFunctionDefinition {
                name: "pin".to_string(),
                description: Some("Pins the message the last message was replying to. If that fails, a reason why is returned.".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {},
                })),
            },
            ChatCompletionFunctionDefinition {
                name: "unpin".to_string(),
                description: Some("Unpins the message the last message was replying to. If that fails, a reason why is returned.".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {},
                })),
            },
            ChatCompletionFunctionDefinition {
                name: "get_users".to_string(),
                description: Some("Gets the users in the chat. Returns a list of users.".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {},
                })),
            },
            ChatCompletionFunctionDefinition {
                name: "angry".to_string(),
                description: Some("Marks that the author of the most recent message has been rude or mean.".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {},
                })),
            },
            ChatCompletionFunctionDefinition {
                name: "happy".to_string(),
                description: Some("Marks that the author of the most recent message has been kind or nice.".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {},
                })),
            },
            ChatCompletionFunctionDefinition {
                name: "get_opinion".to_string(),
                description: Some("Gets the opinion of a user by name. Returns a number from 0 to 100 with 0 meaning that the user is very rude and 100 meaning they are very kind.".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "The name of the user to get the opinion of.",
                        },
                    },
                })),
            },
            ChatCompletionFunctionDefinition {
                name: "stop_listening".to_string(),
                description: Some("Stops listening to the chat. Should be called when the last message isn't directed at Astro.".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {},
                })),
            },
        ])
        .function_call(force_call.map(|function| json!({"name": function})).unwrap_or(json!("auto")))
        .create()
        .await?;

    let returned_message = chat_completion.choices.first().unwrap().message.clone();
    // Add response to history
    append_message(&returned_message);

    message.delete_reaction_emoji(ctx, '🤔').await?;

    ACTIVE_CONVO.store(message.channel_id.into(), Ordering::Relaxed);

    if let Some(function_call) = returned_message.function_call.as_ref() {
        match function_call.name.as_str() {
            "react" => {
                let arguments = serde_json::from_str::<Value>(&function_call.arguments).unwrap();
                let reaction = arguments["emoji"].as_str().unwrap();

                if let Some(possible_emoji) = reaction.chars().next() {
                    // Response might not be a valid emoji. Ignore if not
                    message.react(&ctx.http, possible_emoji).await.ok();
                }
                dbg!(reaction);
            }
            "pin" => {
                if let Some(referenced_message) = message.referenced_message.as_ref() {
                    referenced_message.pin(&ctx.http).await.ok();
                    dbg!("pinned message");
                    query_model(&ctx, message, None).await?;
                } else {
                    append_message(&ChatCompletionMessage {
                        role: ChatCompletionMessageRole::Function,
                        content: Some("Last message was not a reply.".to_string()),
                        name: Some("pin".to_string()),
                        function_call: None,
                    });
                    dbg!("pin failed");
                    query_model(&ctx, message, None).await?;
                }
            }
            "unpin" => {
                if let Some(referenced_message) = message.referenced_message.as_ref() {
                    referenced_message.unpin(&ctx.http).await.ok();
                    dbg!("unpinned message");
                    query_model(&ctx, message, None).await?;
                } else {
                    append_message(&ChatCompletionMessage {
                        role: ChatCompletionMessageRole::Function,
                        content: Some("Last message was not a reply.".to_string()),
                        name: Some("unpin".to_string()),
                        function_call: None,
                    });
                    dbg!("unpin failed");
                    query_model(&ctx, message, None).await?;
                }
            }
            "get_users" => {
                append_message(&ChatCompletionMessage {
                    role: ChatCompletionMessageRole::Function,
                    content: Some(format!(
                        "[{}]",
                        USERS.keys().cloned().collect::<Vec<_>>().join(", ")
                    )),
                    name: Some("get_users".to_string()),
                    function_call: None,
                });
                query_model(&ctx, message, None).await?;
            }
            "angry" => {
                decrement_user_opinion(message.author.id.0);
                query_model(&ctx, message, None).await?;
            }
            "happy" => {
                increment_user_opinion(message.author.id.0);
                query_model(&ctx, message, None).await?;
            }
            "get_opinion" => {
                let arguments = serde_json::from_str::<Value>(&function_call.arguments).unwrap();
                let name = arguments["name"].as_str().unwrap();

                let mut opinion = 50;
                for (user, id) in USERS.iter() {
                    if user.to_lowercase() == name.to_lowercase() {
                        opinion = user_opinion(*id as u64);
                    }
                }

                append_message(&ChatCompletionMessage {
                    role: ChatCompletionMessageRole::Function,
                    content: Some(opinion.to_string()),
                    name: Some("get_opinion".to_string()),
                    function_call: None,
                });
                query_model(&ctx, message, None).await?;
            }

            "stop_listening" => {
                dbg!("stopped listening");
                ACTIVE_CONVO.store(0, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    if let Some(response) = returned_message.content.as_ref() {
        message.reply_maybe_long(&ctx, response.clone()).await?;
        dbg!(response);
    }

    Ok(())
}
