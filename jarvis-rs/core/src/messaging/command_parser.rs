//! Parser de comandos de mensageria
//!
//! Este módulo parseia mensagens de texto e identifica comandos do Jarvis,
//! como `/exec`, `/read`, `/list`, etc.

use jarvis_messaging::message::MessageType;

/// Comando identificado a partir de uma mensagem
#[derive(Debug, Clone)]
pub enum ParsedCommand {
    Exec { command: String, args: Vec<String> },
    Read { path: String },
    List { path: String },
    Search { query: String },
    Help,
    Unknown { text: String },
}

/// Parser de comandos de mensageria
pub struct CommandParser;

impl CommandParser {
    /// Parseia uma mensagem e retorna o comando identificado
    pub fn parse(message_type: &MessageType) -> ParsedCommand {
        let text = match message_type {
            MessageType::Text(text) => text,
            MessageType::Command { command, args } => {
                return Self::parse_command(command, args);
            }
            _ => {
                return ParsedCommand::Unknown {
                    text: format!("Unsupported message type: {:?}", message_type),
                };
            }
        };

        let text = text.trim();
        if text.is_empty() {
            return ParsedCommand::Unknown {
                text: "Empty message".to_string(),
            };
        }

        // Verifica se é um comando (começa com /)
        if text.starts_with('/') {
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.is_empty() {
                return ParsedCommand::Unknown {
                    text: "Empty command".to_string(),
                };
            }

            let command = parts[0].trim_start_matches('/');
            let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

            Self::parse_command(command, &args)
        } else {
            // Mensagem de texto livre - pode ser tratada como comando exec ou conversa natural
            ParsedCommand::Unknown {
                text: text.to_string(),
            }
        }
    }

    fn parse_command(command: &str, args: &[String]) -> ParsedCommand {
        match command.to_lowercase().as_str() {
            "exec" | "shell" | "run" => {
                if args.is_empty() {
                    ParsedCommand::Unknown {
                        text: "Usage: /exec <command>".to_string(),
                    }
                } else {
                    ParsedCommand::Exec {
                        command: args[0].clone(),
                        args: args[1..].to_vec(),
                    }
                }
            }
            "read" | "cat" => {
                if args.is_empty() {
                    ParsedCommand::Unknown {
                        text: "Usage: /read <file>".to_string(),
                    }
                } else {
                    ParsedCommand::Read {
                        path: args[0].clone(),
                    }
                }
            }
            "list" | "ls" | "dir" => {
                let path = args.first().cloned().unwrap_or_else(|| ".".to_string());
                ParsedCommand::List { path }
            }
            "search" | "grep" | "find" => {
                if args.is_empty() {
                    ParsedCommand::Unknown {
                        text: "Usage: /search <query>".to_string(),
                    }
                } else {
                    ParsedCommand::Search {
                        query: args.join(" "),
                    }
                }
            }
            "help" | "?" => ParsedCommand::Help,
            _ => ParsedCommand::Unknown {
                text: format!("Unknown command: /{}", command),
            },
        }
    }

    /// Retorna a mensagem de ajuda
    pub fn help_message() -> String {
        r#"Comandos disponíveis:

/exec <command> [args...] - Executa um comando shell
/read <file>              - Lê o conteúdo de um arquivo
/list [path]              - Lista arquivos em um diretório
/search <query>           - Busca texto em arquivos
/help                     - Mostra esta mensagem de ajuda

Exemplos:
/exec ls -la
/read README.md
/list src/
/search function"#
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exec_command() {
        let msg = MessageType::Text("/exec ls -la".to_string());
        match CommandParser::parse(&msg) {
            ParsedCommand::Exec { command, args } => {
                assert_eq!(command, "ls");
                assert_eq!(args, vec!["-la"]);
            }
            _ => panic!("Expected Exec command"),
        }
    }

    #[test]
    fn test_parse_read_command() {
        let msg = MessageType::Text("/read Cargo.toml".to_string());
        match CommandParser::parse(&msg) {
            ParsedCommand::Read { path } => {
                assert_eq!(path, "Cargo.toml");
            }
            _ => panic!("Expected Read command"),
        }
    }

    #[test]
    fn test_parse_help_command() {
        let msg = MessageType::Text("/help".to_string());
        match CommandParser::parse(&msg) {
            ParsedCommand::Help => {}
            _ => panic!("Expected Help command"),
        }
    }
}
