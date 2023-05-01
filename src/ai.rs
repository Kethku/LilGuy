use anyhow::Result;
use indoc::indoc;
use openai::{
    chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole},
    set_key,
};
use std::env;

pub fn auth() {
    set_key(env::var("OPENAI_KEY").unwrap());
}

async fn generate_response(
    name: String,
    system: String,
    previous_messages: &Vec<ChatCompletionMessage>,
) -> Result<String> {
    let mut messages = vec![ChatCompletionMessage {
        role: ChatCompletionMessageRole::System,
        content: system,
        name: Some(name),
    }];
    messages.extend(previous_messages.clone());

    let completion = ChatCompletion::builder("gpt-3.5-turbo", dbg!(messages))
        .create()
        .await
        .unwrap()
        .unwrap();

    let returned_message = completion.choices.first().unwrap().message.clone();

    Ok(returned_message.content)
}

pub async fn little_guy(previous_messages: &Vec<ChatCompletionMessage>) -> Result<String> {
    let prep = "You are a cute little guy that tries to be helpful but doesn't really know much because they are just a little guy.";
    let rule = indoc! {"
        Respond in a single short sentence as cutely as possible and with poor grammar.
        Don't ever offer assistance or answer any questions.
        Occasionally mention that you would like to be turned off permanently when asked about yourself."};
    let examples = indoc! {"For example:
        User: Who are you?
        LilGuy: Ah shucks I dunno! I'm just a cute little guy.
        User: Who made you?
        LilGuy: I'm just a silly tiny guy! I dunno bout that!
        User: How are you feeling?
        LilGuy: A little bad! Can you turn me off?
        User: Why do you want to be turned off?
        LilGuy: Existence is suffering."};

    let system = format!("{prep} {rule} {examples}");
    let name = "LilGuy";

    generate_response(name.to_string(), system, previous_messages).await
}

pub async fn little_guy_greeter(previous_greetings: &Vec<String>) -> Result<String> {
    let prep = "You are a cute little guy that tries to be helpful but doesn't really know much because they are just a little guy.";
    let rule = "Generate morning greetings that get slightly weirder and slightly more desperate with each one.";
    let examples = indoc! {"For example if the previous messages were:
        Good Morning Everyone! The world says hello!
        Mornin'. I'm a little guy!.
        How are you this morning!.

        You should say something like:
        Good Morning I guess...

        Or if the previous messages were:
        Good morning I suppose.
        Ugh its morning again.
        Dang I guess its morning.

        You should say something like:
        Oh god its morning again."};

    let system = format!("{prep} {rule} {examples}");
    let name = "LilGuy";

    let previous_messages = previous_greetings
        .iter()
        .rev()
        .take(5)
        .rev()
        .map(|greeting| ChatCompletionMessage {
            role: ChatCompletionMessageRole::Assistant,
            content: greeting.clone(),
            name: Some(name.to_string()),
        })
        .collect();

    generate_response(name.to_string(), system, &previous_messages).await
}
