/*
   Copyright 2026 Lee Scott Barney

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

use crate::errors::{CompilerError, Result};
use crate::lexer::{Lexer, Token, TokenKind};

pub fn rewrite_case_do_end(source: &str, _file_name: &str) -> Result<String> {
    let mut lexer = Lexer::new(source.to_string(), _file_name.to_string());
    let tokens = lexer.tokenize()?;
    let replacements = collect_replacements(&tokens)?;

    Ok(apply_replacements(source, &replacements))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Replacement {
    start: usize,
    end: usize,
    text: &'static str,
}

fn collect_replacements(tokens: &[Token]) -> Result<Vec<Replacement>> {
    let mut replacements = Vec::new();
    let mut handled_do_tokens = std::collections::HashSet::new();
    let case_ranges = collect_case_body_ranges(tokens)?;
    let mut index = 0;

    while index < tokens.len() {
        if tokens[index].kind == TokenKind::RightArrow {
            if let Some(body_start) = next_non_eof(tokens, index + 1) {
                if tokens[body_start].kind == TokenKind::Do && is_inside_case_body(index, &case_ranges) {
                    let body_end = find_matching_end(tokens, body_start)?;
                    replacements.push(replace_token(&tokens[body_start], "{"));
                    replacements.push(replace_token(&tokens[body_end], "}"));
                    handled_do_tokens.insert(body_start);
                    index = body_start + 1;
                    continue;
                }
            }
        } else if tokens[index].kind == TokenKind::Do && !handled_do_tokens.contains(&index) {
            let body_end = find_matching_end(tokens, index)?;
            replacements.push(replace_token(&tokens[index], ""));
            replacements.push(replace_token(&tokens[body_end], ""));
            handled_do_tokens.insert(index);
        }

        index += 1;
    }

    Ok(replacements)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TokenRange {
    start: usize,
    end: usize,
}

fn collect_case_body_ranges(tokens: &[Token]) -> Result<Vec<TokenRange>> {
    let mut ranges = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if tokens[index].kind == TokenKind::Case {
            if let Some(case_body_start) = find_case_body_start(tokens, index + 1) {
                let case_body_end = find_matching_right_brace(tokens, case_body_start)?;
                ranges.push(TokenRange {
                    start: case_body_start,
                    end: case_body_end,
                });
                index = case_body_start + 1;
                continue;
            }
        }

        index += 1;
    }

    Ok(ranges)
}

fn find_case_body_start(tokens: &[Token], start: usize) -> Option<usize> {
    let mut index = start;

    while index < tokens.len() {
        match tokens[index].kind {
            TokenKind::Of => {
                let next = next_non_eof(tokens, index + 1)?;
                return (tokens[next].kind == TokenKind::LeftBrace).then_some(next);
            }
            TokenKind::EOF | TokenKind::Semicolon | TokenKind::RightBrace | TokenKind::End => {
                return None;
            }
            _ => index += 1,
        }
    }

    None
}

fn find_matching_right_brace(tokens: &[Token], left_brace_index: usize) -> Result<usize> {
    let mut depth = 1usize;
    let mut index = left_brace_index + 1;

    while index < tokens.len() {
        match tokens[index].kind {
            TokenKind::LeftBrace => depth += 1,
            TokenKind::RightBrace => {
                depth -= 1;
                if depth == 0 {
                    return Ok(index);
                }
            }
            TokenKind::EOF => break,
            _ => {}
        }

        index += 1;
    }

    Err(CompilerError::parse_error(
        tokens[left_brace_index].location.clone(),
        "Expected matching '}' for case body".to_string(),
    ))
}

fn is_inside_case_body(index: usize, ranges: &[TokenRange]) -> bool {
    ranges
        .iter()
        .any(|range| index > range.start && index < range.end)
}

fn next_non_eof(tokens: &[Token], start: usize) -> Option<usize> {
    (start..tokens.len()).find(|&index| tokens[index].kind != TokenKind::EOF)
}

fn find_matching_end(tokens: &[Token], do_index: usize) -> Result<usize> {
    let mut depth = 1usize;
    let mut index = do_index + 1;

    while index < tokens.len() {
        match tokens[index].kind {
            TokenKind::Do => depth += 1,
            TokenKind::End => {
                depth -= 1;
                if depth == 0 {
                    return Ok(index);
                }
            }
            TokenKind::EOF => break,
            _ => {}
        }

        index += 1;
    }

    Err(CompilerError::parse_error(
        tokens[do_index].location.clone(),
        "Expected matching 'end' for case branch 'do' block".to_string(),
    ))
}

fn replace_token(token: &Token, text: &'static str) -> Replacement {
    let start = token.location.offset;
    Replacement {
        start,
        end: start + token.lexeme.len(),
        text,
    }
}

fn apply_replacements(source: &str, replacements: &[Replacement]) -> String {
    if replacements.is_empty() {
        return source.to_string();
    }

    let mut rewritten = source.to_string();
    let mut ordered = replacements.to_vec();
    ordered.sort_by_key(|replacement| replacement.start);
    ordered.dedup_by_key(|replacement| replacement.start);

    for replacement in ordered.iter().rev() {
        rewritten.replace_range(replacement.start..replacement.end, replacement.text);
    }

    rewritten
}
