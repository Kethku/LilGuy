use anyhow::{anyhow, Result};
use async_recursion::async_recursion;
use indoc::indoc;
use openai::{
    chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole},
    moderations::Moderation,
    set_key,
};
use serde::{Deserialize, Serialize};
use std::env;

use crate::classifiers::is_instruction;

pub fn auth() {
    set_key(env::var("OPENAI_KEY").unwrap());
}

#[derive(Serialize, Deserialize)]
struct ChatState {
    name: String,
    identity: String,
    rules: Vec<String>,
    examples: Vec<String>,
    previous_messages: Vec<ChatCompletionMessage>,
}

impl ChatState {
    fn new(name: String) -> Self {
        Self {
            name,
            stored_instructions: Vec::new(),
            previous_messages: Vec::new(),
        }
    }

    async fn add_query(&mut self, query: &str, instruction: bool) {
        if instruction {
            self.stored_instructions.push(ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: query.to_string(),
                name: None,
            });
        } else {
            self.previous_messages.push(ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: query.to_string(),
                name: None,
            });
        }

        if self.stored_instructions.len() > 6 {
            self.stored_instructions.remove(0);
        }

        if self.previous_messages.len() > 10 {
            self.previous_messages.remove(0);
        }
    }

    fn get_messages(&self, system: &str) -> Vec<ChatCompletionMessage> {
        let mut messages = vec![ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: system.to_string(),
            name: Some(self.name.to_string()),
        }];
        messages.extend(self.stored_instructions.clone());
        messages.extend(self.previous_messages.clone());
        messages
    }
}

#[async_recursion]
pub async fn generate_response(
    name: &str,
    system: &str,
    history_file: Option<&'static str>,
    query: Option<String>,
    instruction: bool,
) -> Result<String> {
    // Read previous messages from file
    let mut chat_state: ChatState = if let Some(history_file) = history_file {
        std::fs::read_to_string(history_file)
            .ok()
            .and_then(|history| serde_json::from_str(&history).ok())
            .unwrap_or(ChatState::new(name.to_string()))
    } else {
        ChatState::new(name.to_string())
    };

    if let Some(query) = query {
        chat_state.add_query(&query, instruction).await;
    }

    // Make request
    let completion =
        ChatCompletion::builder("gpt-3.5-turbo", dbg!(chat_state.get_messages(system)))
            .create()
            .await
            .unwrap()
            .unwrap();

    // Add new bot message to previous messages
    let returned_message = completion.choices.first().unwrap().message.clone();

    if let Some(history_file) = history_file {
        std::fs::write(history_file, serde_json::to_string_pretty(&chat_state)?)?;
    }

    Ok(returned_message.content)
}

pub async fn astro(query: &str, instruction: bool) -> Result<String> {
    let prep = "You are a cute little guy named Astro that tries to be helpful but doesn't really know much because they are just a little guy.";
    let rule = indoc! {"
    Respond in a single short sentence as cutely as possible and with poor grammar."};
    let examples = indoc! {"For example:
        User: Who are you?
        Astro: Ah shucks I dunno! I'm just a cute little guy.
        User: Who made you?
        Astro: I'm just a silly tiny guy! I dunno bout that!"};
    let system = format!("{prep} {rule} {examples}");

    generate_response(
        "Astro",
        &system,
        Some("./astro.txt"),
        Some(query.to_string()),
        instruction,
    )
    .await
}

pub async fn greeter() -> Result<String> {
    let prep = "You are a cute little guy named Astro that tries to be helpful but doesn't really know much because they are just a little guy.";
    let rule =
        "Generate a single morning greeting that get slightly weirder and slightly more chaotic with each one.";
    let examples = indoc! {"For example if the previous messages were:
        Good Morning Everyone! The world says hello!
        Mornin'. I'm a little guy!.
        How are you all this morning?.

        You should say something like:
        Mornin campers! Its work time.

        Or if the previous messages were:
        The sun is up. Hide!
        Sky is bright. Moon is gone. Its work time.
        Mornin crew. Work time is here. 

        You should say something like:
        Good morning babies. Wake up."};

    let system = format!("{prep} {rule} {examples}");

    generate_response("Astro", &system, Some("./greetings.txt"), None, false).await
}
