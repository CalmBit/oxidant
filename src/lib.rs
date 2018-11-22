#![cfg_attr(feature = "cargo-clippy", deny(pedantic))]

extern crate json;

use std::error::Error;

#[derive(Debug)]
pub enum Command {
    Test,
    HealthCheck,
    Echo(String),
    Add(i32, i32)
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        match (&self, other) {
            (Command::Test, Command::Test) | (Command::HealthCheck, Command::HealthCheck) => true,
            (Command::Echo(x), Command::Echo(y)) => x == y,
            (_, _) => false,
        }
    }
}

impl Command {
    pub fn parse(cmd: &[String]) -> Result<Self, String> {
        let mut iter = cmd.into_iter();
        if let Some(s) = iter.next() {
            match s.as_ref() {
                "test" => Ok(Command::Test),
                "health" => Ok(Command::HealthCheck),
                "echo" => Ok(Command::Echo(
                    iter.map(|s| &**s).collect::<Vec<&str>>().join(" "),
                )), // holy shit
                "add" => {
                    let a = match iter.next() {
                        Some(s) => match s.parse::<i32>() {
                            Ok(i) => i,
                            Err(e) => return Err(e.to_string())
                        },
                        None => return Err("a was not present".to_string())
                    };

                    let b = match iter.next() {
                        Some(s) => match s.parse::<i32>() {
                            Ok(i) => i,
                            Err(e) => return Err(e.to_string())
                        },
                        None => return Err("a was not present".to_string())
                    };
                    Ok(Command::Add(a, b))
                }
                s => Err(format!("no such command {}", s)),
            }
        } else {
            Err(String::from("no command given"))
        }
    }

    pub fn name(&self) -> String {
        match self {
            Command::Test => "test",
            Command::HealthCheck => "health_check",
            Command::Echo(_) => "echo",
            Command::Add(_, _) => "add"
        }.to_string()
    }

    fn serialize_args(&self) -> Option<String> {
        match self {
            Command::Echo(s) => Some(format!("\"echoed\": \"{}\"", s)),
            Command::Add(a, b) => Some(format!("\"a\": {}, \"b\": {}", a, b)),
            _ => None,
        }
    }

    pub fn serialize(&self) -> String {
        let mut res = format!("{{\"command\": \"{}\"", self.name());
        if let Some(args) = self.serialize_args() {
            res.push_str(&format!(", {}", args));
        }
        res.push('}');
        res.push('\n');
        res
    }

    pub fn deserialize(blob: &str) -> Result<Self, String> {
        let cmd_parsed = match json::parse(blob) {
            Ok(c) => c,
            Err(e) => {
                return Err(e.description().to_string());
            }
        };

        if cmd_parsed.has_key("command") {
            if let Some(s) = cmd_parsed["command"].as_str() {
                return match s {
                    "test" => Ok(Command::Test),
                    "health" => Ok(Command::HealthCheck),
                    "echo" => match cmd_parsed["echoed"].as_str() {
                        Some(a) => Ok(Command::Echo(a.to_string())),
                        None => Err("bad echo - no key `echoed`".to_string()),
                    },
                    "add" => {
                        let a = cmd_parsed["a"].as_i32();
                        let b = cmd_parsed["b"].as_i32();
                        if a.is_none() || b.is_none() {
                            return Err("missing argument(s)".to_string());
                        }
                        Ok(Command::Add(a.expect("a was not i32"), b.expect("b was not i32")))
                    }
                    _ => Err("bad command".to_string()),
                };
            }
        }

        Err("no command".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stringify_vec(v: Vec<&str>) -> Vec<String> {
        v.into_iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_parse() {
        let test = stringify_vec(vec!["test"]);
        assert_eq!(Command::Test, Command::parse(&test).expect("Not test"));

        let echo = stringify_vec(vec!["echo", "1", "2", "3"]);
        assert_eq!(
            Command::Echo("1 2 3".to_string()),
            Command::parse(&echo).expect("Not echo")
        );

        let nothing: Vec<String> = Vec::new();
        assert!(Command::parse(&nothing).is_err());
    }
}
