#[macro_use]
extern crate log;
#[macro_use]
extern crate lalrpop_util;
#[macro_use]
extern crate lazy_static;

lalrpop_mod!(pub preproc);

mod common;
mod p1to3;

use common::{CVersion, Issue, IssueDesc, IssueType, Params, Version};

use env_logger::Env;
use logos::Logos;

// These are the tokens we will pass to Lalrpop to convert to an AST.
#[derive(Debug, PartialEq)]
enum LalrpopToken {
    PreprocStart,
    NonPreproc(String),
    Space,
    Ident(String),
    Number(String),
    Punct(char),
}

#[derive(Logos, Debug, PartialEq)]
enum SourceLex {
    #[end]
    End,

    #[error]
    Error,

    #[token = "\n#"]
    PreprocStart,

    // Works as long as there is a new line at both the start and end of the
    // file.
    #[regex = "\n[^#\n][^\n]*"]
    NonPreprocStart,
}

#[derive(Logos, Debug, PartialEq)]
enum PreprocLex {
    #[end]
    End,

    #[error]
    Error,

    //     preprocessing-token:
    //                  header-name
    //                  identifier
    //                  pp-number
    //                  character-constant
    //                  string-literal
    //                  operator
    //                  punctuator
    //                  each non-white-space character that cannot be one of
    //                  the above
    #[token = " "]
    Space,

    #[regex = "[a-zA-Z_]+"]
    Ident,

    #[regex = "[0-9]+"]
    Number,

    #[regex = "[!\"#%&'()*+,-./:;<=>?\\[\\\\\\]^_{}|~]"]
    Punct,
}

fn main() {
    env_logger::from_env(Env::default().default_filter_or("warn")).init();

    // Inputs
    let file = "
\"\\\\
n\"

";
    let params = Params::default();
    let filename = "a";

    let ret = p1to3::preproc_phases_1_to_3(file, filename, &params);

    dbg!(&ret);
    println!("{}", ret.new_file);

    enum ActiveLexer {}

    let mut lexer = SourceLex::lexer(&file as &str);

    while lexer.token != SourceLex::End && lexer.token != SourceLex::Error {
        dbg!((&lexer.token, lexer.slice(), lexer.range()));
        lexer.advance();
    }

    //dbg!(preproc::GroupParser::new().parse("22"));
    //dbg!(preproc::GroupParser::new().parse("(  22)"));
    //dbg!(preproc::GroupParser::new().parse("(  22) \n 22"));
}
