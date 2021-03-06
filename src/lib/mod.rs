pub mod token;
pub mod parser;

use std::{env, fs};
use std::error::Error;
use std::fs::read_to_string;
use std::io::{stdin, stdout, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use logos::{Logos, Lexer};
use subst::substitute;
use crate::lib::parser::Parser;
use crate::lib::token::Token;


/// Main run function with an optional config file
/// string slice you can specify
pub fn run(config_file: Option<&str>) {
    match config_file {
        None => {
            match fs::read("../../.clararc") {
                Ok(_) => {
                    setup(".clararc").unwrap()
                }
                Err(_) => {
                    fs::write("../../.clararc", "").unwrap();
                }
            }
        }
        Some(file) => {
            setup(file).unwrap();
        }
    }

    loop {
        print!("> ");
        stdout().flush().expect("Error while pushing to stdout");

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();

        let mut san_input = String::new();
        for i in input.split(" ") {
            if i.contains("$") {

                match substitute(i, &subst::Env) {
                    Ok(sub) => {
                        san_input.push_str(sub.as_str());
                        san_input.push_str(" ");
                    }

                    Err(err) => {
                        println!("{}", err )
                    }
                }

            } else {
                san_input.push_str(i);
                san_input.push_str(" ");
            }
        }

        // must be peekable so we know when we are on the last command
        let mut commands = san_input.trim().split(" | ").peekable();
        let mut previous_command = None;

        while let Some(command) = commands.next() {
            let mut parts = command.trim().split_whitespace();

            let command = match parts.next() {
                Some(data) => data,
                None => break,
            };
            let args = parts;

            match command {
                "cd" => {
                    let new_dir = args.peekable().peek().map_or("/", |x| *x);
                    let root = Path::new(new_dir);
                    if let Err(e) = env::set_current_dir(&root) {
                        eprintln!("{}", e);
                    }

                    previous_command = None;
                }
                "exit" => return,
                command => {
                    let stdin = previous_command.map_or(Stdio::inherit(), |output: Child| {
                        Stdio::from(output.stdout.unwrap())
                    });

                    let stdout = if commands.peek().is_some() {
                        // there is another command piped behind this one
                        // prepare to send output to the next command
                        Stdio::piped()
                    } else {
                        // there are no more commands piped behind this one
                        // send output to shell stdout
                        Stdio::inherit()
                    };

                    let output = Command::new(command)
                        .args(args)
                        .stdin(stdin)
                        .stdout(stdout)
                        .spawn();

                    match output {
                        Ok(output) => {
                            previous_command = Some(output);
                        }
                        Err(e) => {
                            previous_command = None;
                            eprintln!("{}", e);
                        }
                    };
                }
            }
        }

        if let Some(mut final_command) = previous_command {
            // block until the final command has finished
            final_command.wait().expect(format!("Error while waiting for task {:?} with PID {}",
                                                final_command,
                                                final_command.id()).as_str());
        }
    }
}


fn setup(config_name: &str) -> Result<(), Box<dyn Error>> {
    match read_to_string(config_name) {
        Ok(data) => {
            let lex: Lexer<Token> = Token::lexer(data.as_str());
            let mut token_list = vec![];

            for i in lex {
                if i != Token::Error {
                    token_list.push(i);
                }
            }


            let mut parser = Parser::new(token_list);
            parser.run().expect("TODO: Error message");

            Ok(())
        }
        Err(_) => {
            Err(Box::from("Error: config file not found"))
        }
    }
}


#[cfg(test)]
mod tests {
    use serial_test::serial;
    use std::fs::{remove_file, write};
    use super::*;
    #[serial(".clararc_temp")]
    #[test]
    fn test_empty_config() {
        write(".clararc_temp", "").unwrap();
        setup(".clararc_temp").unwrap();
        remove_file(".clararc_temp").unwrap();
    }

    #[serial(".clararc_temp")]
    #[test]
    fn test_simple_config() {
        write(".clararc_temp", "export TEST=\"test\"").unwrap();
        setup(".clararc_temp").unwrap();
        remove_file(".clararc_temp").unwrap();
    }
}