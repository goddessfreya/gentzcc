//! The state machine that handles translation phases 1 and 2, and
//! the comment/whitespace portion of translation phase 3.

#[cfg(test)]
mod tests;

use crate::common::{
    CVersion, CppVersion, Issue, IssueDesc, IssueType, Location, Params,
    Version,
};

type CharStack = [Option<(char, Location)>; 2];

#[derive(Debug, PartialEq, Copy, Clone)]
enum CommentType {
    // C99+ only.
    SingleLine,
    MultiLine,
}

#[derive(Debug)]
struct State {
    lc_active:      Option<(CommentType, Location)>,
    quot_active:    Option<(char, Location)>,
    new_file:       String,
    non_multimerge: bool,
    cloc:           Location,
    oloc:           Location,
    last_add:       Option<char>,
    // We use the number of spaces to guess the number of tokens we will
    // end up having for lalrpop.
    num_spaces:  usize,
    issues:      Vec<Issue>,
    loc_mapping: Vec<(Location, Location)>,
}

impl State {
    fn new(filename: &str, file: &str) -> Self {
        State {
            lc_active:      None,
            quot_active:    None,
            new_file:       String::new(),
            non_multimerge: false,
            cloc:           Location::new(filename.to_string(), 0, 0),
            oloc:           Location::new(filename.to_string(), 1, 0),
            last_add:       None,
            num_spaces:     0,
            issues:         vec![],
            loc_mapping:    Vec::with_capacity(file.len() / 10),
        }
    }

    fn cur_stack<'a>(
        &mut self,
        stacks: &'a mut [CharStack],
    ) -> &'a mut CharStack {
        if self.lc_active.is_some() {
            &mut stacks[1]
        } else {
            &mut stacks[0]
        }
    }

    fn insert_stack(
        &mut self,
        stack: &mut CharStack,
        b: Option<(char, Location)>,
    ) {
        let mut this_stack = b;
        std::mem::swap(&mut this_stack, &mut stack[0]);
        stack.swap(0, 1);
        self.non_multimerge = false;

        if !self.lc_active.is_some()
            || stack[1].as_ref().map(|b| b.0) == Some('\n')
        {
            if let Some((s0, sloc)) = this_stack {
                // Excludes newline.
                let is_whitespace = |c| -> bool {
                    c == '\x0C' // Form feed
                        || c == '\x0B' // Vert tab
                        || c == ' '
                        || c == '\t'
                };

                if self.quot_active.is_some()
                    || (((self.last_add != Some(' ')
                        && self.last_add != Some('\n'))
                        || !is_whitespace(s0))
                        && (s0 != '\n' || self.last_add != Some('\n')))
                {
                    let s0 = if is_whitespace(s0) { ' ' } else { s0 };
                    if s0 == '\n' && self.last_add == Some(' ') {
                        self.new_file.pop();
                    } else if is_whitespace(s0) || s0 == '\n' {
                        self.num_spaces += 1;
                    }

                    if s0 == '\n' {
                        self.oloc.nchar = 0;
                        self.oloc.nline += 1;
                    } else {
                        self.oloc.nchar += 1;
                        let this_mapping = (sloc, self.oloc.clone());
                        if if let Some(lm) = self.loc_mapping.last() {
                            let mut lm = lm.clone();
                            if lm.0.nline != this_mapping.0.nline {
                                lm.1.nline = lm.1.nline + this_mapping.0.nline
                                    - lm.0.nline;
                                lm.0.nline = this_mapping.0.nline;
                                lm.1.nchar = 0;
                                lm.0.nchar = 0;
                            }
                            lm.1.nchar =
                                lm.1.nchar + this_mapping.0.nchar - lm.0.nchar;
                            lm.0.nchar = this_mapping.0.nchar;
                            lm != this_mapping
                        } else {
                            true
                        } {
                            self.loc_mapping.push(this_mapping);
                        }
                    }

                    self.new_file.push(s0);
                    self.last_add = Some(s0);
                }
            }
        }
    }

    fn replace_stack1(
        &mut self,
        stack: &mut CharStack,
        b: Option<(char, Location)>,
    ) {
        stack[1] = b;
        self.non_multimerge = false;
    }

    fn replace_stack2(
        &mut self,
        stack: &mut CharStack,
        b: Option<(char, Location)>,
    ) {
        stack[0] = None;
        stack[1] = b;
        self.non_multimerge = false;
    }

    fn del_char(&mut self, stack: &mut CharStack) {
        stack.swap(0, 1);
        stack[0] = None;
        self.non_multimerge = false;
    }

    fn drain_stack(&mut self, stack: &mut CharStack) {
        self.insert_stack(stack, None);
        self.insert_stack(stack, None);
    }

    fn end_comment(&mut self, stack: &mut CharStack) {
        self.drain_stack(stack);
        self.lc_active = None;
    }
}

#[derive(Debug, PartialEq)]
pub struct Output {
    pub num_spaces:  usize,
    pub new_file:    String,
    pub issues:      Vec<Issue>,
    pub loc_mapping: Vec<(Location, Location)>,
}

// Will return the number of whitespaces (including new lines) and the prossessed
// file.
//
// TODO: Officially, newlines are either '\n', '\r' or '\r\n'.... however, we
// only treat '\n' as a newline.
pub fn preproc_phases_1_to_3(
    file: &str,
    filename: &str,
    params: &Params,
) -> Output {
    let mut stacks: [CharStack; 2] = [[None, None], [None, None]];
    let mut state = State::new(filename, &file);

    // Makes our processing a lot easier, to insert a newline at the start.
    //
    // Some specs say if there is no newline at the end of the file the
    // behaviour is undefined, others say that we should insert one.
    //
    // We insert two at the end, just in case the last line had a `\` at the
    // end.
    //
    // But before that, we issue a warning if the behaviour is undefined.
    if params.version.ver_le(CVersion::Max, CppVersion::Cpp03)
        && file.len() != 0
    {
        if &file[file.len() - 1..file.len()] != "\n"
            || &file[file.len() - 2..file.len() - 1] == "\\"
        {
            state.issues.push(Issue::new(
                Location::new(
                    filename.to_string(),
                    file.lines().count(),
                    file.lines().nth_back(0).unwrap().len(),
                ),
                IssueType::Warning,
                IssueDesc::FileEndMissingNewline,
            ));
        }
    }

    let file = "\n".to_string() + file + "\n\n";

    for b in file.chars() {
        let mut stack = state.cur_stack(&mut stacks);
        state.cloc.nchar += 1;

        if !state.lc_active.is_some() {
            match state.quot_active {
                None if b == '\'' || b == '"' => {
                    state.quot_active = Some((b, state.cloc.clone()))
                },
                Some((q, _))
                    if q == b
                        && ((stack[1].is_some()
                            && stack[1].as_ref().map(|s| s.0)
                                != Some('\\'))
                            || (stack[1].is_none()
                                && stack[0].is_some()
                                && stack[0].as_ref().map(|s| s.0)
                                    != Some('\\'))
                            || (stack[1].is_none() && stack[0].is_none())) =>
                {
                    state.quot_active = None
                },
                _ => (),
            }
        }

        if b == '\n' {
            if stack[1].as_ref().map(|s| s.0) != Some('\\') {
                if let Some(ref mut lca) = state.lc_active {
                    if lca.0 == CommentType::SingleLine {
                        state.end_comment(stack)
                    }
                }

                if let Some(ref qa) = state.quot_active {
                    state.issues.push(Issue::new(
                        qa.1.clone(),
                        IssueType::Warning,
                        IssueDesc::QuotationMarkNotClosed(qa.0),
                    ));
                }

                state.cloc.nchar = 0;
                state.cloc.nline += 1;
            } else if !state.non_multimerge {
                state.del_char(stack);
                state.non_multimerge = true;

                state.cloc.nchar = 0;
                state.cloc.nline += 1;
                continue;
            }
        } else if stack[0].as_ref().map(|s| s.0) == Some('?')
            && stack[1].as_ref().map(|s| s.0) == Some('?')
        {
            // Macro cause lambda causes lifetime issues :/
            macro_rules! replace_char {
                ($rep:tt) => {{
                    let mut tri_loc = state.cloc.clone();
                    tri_loc.nchar -= 2;
                    if params.version.ver_ge(CVersion::Max, CppVersion::Cpp17)
                        || !params.trigraphs
                    {
                        state.issues.push(Issue::new(
                            tri_loc,
                            IssueType::Warning,
                            IssueDesc::TrigraphPresentAndIgnored(b),
                        ));
                    } else {
                        if params.wtrigraphs {
                            state.issues.push(Issue::new(
                                tri_loc.clone(),
                                IssueType::Warning,
                                IssueDesc::TrigraphPresent(b),
                            ));
                        }
                        state.replace_stack2(stack, Some(($rep, tri_loc)));
                        continue;
                    }
                }};
            }
            match b {
                '<' => replace_char!('{'),
                '>' => replace_char!('}'),
                '(' => replace_char!('['),
                ')' => replace_char!(']'),
                '=' => replace_char!('#'),
                '/' => replace_char!('\\'),
                '\'' => replace_char!('^'),
                '!' => replace_char!('|'),
                '-' => replace_char!('~'),
                _ => (),
            }
        }

        if state.quot_active.is_none() {
            // Macro cause lambda causes lifetime issues :/
            macro_rules! handle_comment_start {
                (CommentType :: $type:ident) => {
                    let mut com_loc = state.cloc.clone();
                    com_loc.nchar -= 1;
                    state.lc_active =
                        Some((CommentType::$type, com_loc.clone()));
                    state.replace_stack1(stack, Some((' ', com_loc)));
                    stack = state.cur_stack(&mut stacks);
                    assert_eq!(*stack, [None, None]);
                };
            }
            match (&state.lc_active, &mut stack, b) {
                (None, [_, Some(('/', _))], '/')
                    if params
                        .version
                        .ver_ge(CVersion::C99, CppVersion::Min) =>
                {
                    handle_comment_start!(CommentType::SingleLine);
                    continue;
                }
                (None, [_, Some(('/', _))], '*') => {
                    handle_comment_start!(CommentType::MultiLine);
                    continue;
                },
                (
                    Some((CommentType::MultiLine, _)),
                    [_, Some(('*', _))],
                    '/',
                ) => {
                    state.lc_active = None;
                    continue;
                },
                _ => (),
            }
        }

        state.insert_stack(stack, Some((b, state.cloc.clone())));
    }

    let stack = state.cur_stack(&mut stacks);
    state.drain_stack(stack);

    match state.lc_active {
        Some((CommentType::SingleLine, _)) => {
            panic!("Single line comment not at {:?}", state.lc_active)
        },
        Some((CommentType::MultiLine, loc)) => {
            state.issues.push(Issue::new(
                loc,
                IssueType::Error,
                IssueDesc::MultilineCommentNotClosed,
            ));
        },
        _ => (),
    }

    match state.quot_active {
        Some((q, loc)) => {
            state.issues.push(Issue::new(
                loc,
                IssueType::Error,
                IssueDesc::QuotationMarkNotClosed(q),
            ));
        },
        _ => (),
    }

    Output {
        num_spaces:  state.num_spaces,
        new_file:    state.new_file,
        issues:      state.issues,
        loc_mapping: state.loc_mapping,
    }
}
