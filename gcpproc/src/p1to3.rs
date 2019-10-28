//! The state machine that handles translation phases 1 and 2, and
//! the comment/whitespace portion of translation phase 3.
//!
//! Garuntees a newline at both the start and end of the file.

#[cfg(test)]
mod tests;

use crate::common::{
    CVersion, CppVersion, Issue, IssueDesc, IssueType, Location, Params,
    Version,
};

type CharStack = [Option<(char, Location)>; 3];

#[derive(Debug, PartialEq, Copy, Clone)]
enum CommentType {
    // C99+ only.
    SingleLine,
    MultiLine,
}

#[derive(Debug, PartialEq)]
enum LineState {
    SeekingHash,
    FoundHash,
    FoundNonHash,
}

#[derive(Debug)]
struct State {
    lc_active: Option<(CommentType, Location)>,
    quot_active: Option<(char, Location)>,
    new_file: String,
    non_multimerge: bool,
    cloc: Location,
    oloc: Location,
    last_add: Option<char>,
    issues: Vec<Issue>,
    loc_mapping: Vec<(Location, Location)>,
    line_state: LineState,
}

// Excludes newline.
fn is_whitespace(c: char) -> bool {
    c == '\x0C' // Form feed
        || c == '\x0B' // Vert tab
        || c == ' '
        || c == '\t'
}

impl State {
    fn new(filename: &str, file: &str) -> Self {
        State {
            lc_active: None,
            quot_active: None,
            new_file: String::with_capacity(file.len()),
            non_multimerge: false,
            cloc: Location::new(filename.to_string(), 0, 0),
            oloc: Location::new(filename.to_string(), 1, 0),
            last_add: None,
            issues: vec![],
            loc_mapping: Vec::with_capacity(file.len() / 10),
            line_state: LineState::SeekingHash,
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
        match (&stack, &self.line_state) {
            ([_, _, Some(('\n', _))], _) => {
                if self.quot_active.is_none() && self.lc_active.is_none() {
                    self.line_state = LineState::SeekingHash;
                } else {
                    self.line_state = LineState::FoundNonHash;
                }
            }
            ([None, s1, Some((s2, _))], LineState::SeekingHash) => {
                let s1 = s1.as_ref().map(|(s1, _)| *s1);
                if is_whitespace(*s2) {
                    return;
                } else if *s2 == '#' {
                    self.line_state = LineState::FoundHash
                } else if *s2 != '?'
                    || (s1.is_some() && s1 != Some('?') && s1 != Some('\n'))
                {
                    self.line_state = LineState::FoundNonHash;
                }
            }
            _ => (),
        }

        let mut this_stack = b;
        std::mem::swap(&mut this_stack, &mut stack[0]);
        stack.swap(0, 1);
        stack.swap(1, 2);
        self.non_multimerge = false;

        if let Some((s0, sloc)) = this_stack {
            let s0 = if self.line_state == LineState::FoundNonHash && s0 == '\n'
            {
                ' '
            } else {
                s0
            };

            if !self.lc_active.is_some() {
                if self.quot_active.is_some()
                    || (((self.last_add != Some(' ')
                        && self.last_add != Some('\n'))
                        || !is_whitespace(s0))
                        && (s0 != '\n' || self.last_add != Some('\n')))
                {
                    let s0 = if is_whitespace(s0) { ' ' } else { s0 };
                    if s0 == '\n' && self.last_add == Some(' ') {
                        self.new_file.pop();
                    } else {
                        *self.oloc.nchar.as_mut().unwrap() += 1;
                    }

                    if s0 == '\n' {
                        *self.oloc.nchar.as_mut().unwrap() = 0;
                        *self.oloc.nline.as_mut().unwrap() += 1;
                    } else if s0 != ' ' {
                        let this_mapping = (sloc, self.oloc.clone());
                        if if let Some(lm) = self.loc_mapping.last() {
                            let mut lm = lm.clone();
                            if lm.0.nline != this_mapping.0.nline {
                                *lm.1.nline.as_mut().unwrap() =
                                    lm.1.nline.unwrap()
                                        + this_mapping.0.nline.unwrap()
                                        - lm.0.nline.unwrap();
                                *lm.0.nline.as_mut().unwrap() =
                                    this_mapping.0.nline.unwrap();
                                *lm.1.nchar.as_mut().unwrap() = 1;
                                *lm.0.nchar.as_mut().unwrap() = 1;
                            }
                            *lm.1.nchar.as_mut().unwrap() = lm.1.nchar.unwrap()
                                + this_mapping.0.nchar.unwrap()
                                - lm.0.nchar.unwrap();
                            *lm.0.nchar.as_mut().unwrap() =
                                this_mapping.0.nchar.unwrap();
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
        stack[2] = b;
        self.non_multimerge = false;
    }

    fn replace_stack2(
        &mut self,
        stack: &mut CharStack,
        b: Option<(char, Location)>,
    ) {
        stack[1] = None;
        stack.swap(0, 1);
        stack[2] = b;
        self.non_multimerge = false;
    }

    fn del_char(&mut self, stack: &mut CharStack) {
        stack.swap(1, 2);
        stack.swap(0, 1);
        stack[0] = None;
        self.non_multimerge = false;
    }

    fn drain_stack(&mut self, stack: &mut CharStack) {
        for _ in 0..stack.len() {
            self.insert_stack(stack, None);
        }
    }

    fn end_comment(&mut self, stack: &mut CharStack) {
        self.drain_stack(stack);
        self.lc_active = None;
    }

    fn process_char(
        &mut self,
        params: &Params,
        stacks: &mut [CharStack],
        b: char,
    ) {
        let mut stack = self.cur_stack(stacks);
        *self.cloc.nchar.as_mut().unwrap() += 1;

        if !self.lc_active.is_some() {
            match self.quot_active {
                None if b == '\'' || b == '"' => {
                    self.quot_active = Some((b, self.cloc.clone()))
                }
                Some((q, _))
                    if q == b
                        && ((stack[2].is_some()
                            && stack[2].as_ref().map(|s| s.0)
                                != Some('\\'))
                            || (stack[2].is_none()
                                && stack[1].is_some()
                                && stack[1].as_ref().map(|s| s.0)
                                    != Some('\\'))
                            || (stack[2].is_none() && stack[1].is_none())) =>
                {
                    self.quot_active = None
                }
                _ => (),
            }
        }

        if b == '\n' {
            if stack[2].as_ref().map(|s| s.0) != Some('\\') {
                if let Some(ref mut lca) = self.lc_active {
                    if lca.0 == CommentType::SingleLine {
                        self.end_comment(stack)
                    }
                }

                if let Some(ref qa) = self.quot_active {
                    self.issues.push(Issue::new(
                        Some(qa.1.clone()),
                        IssueType::Warning,
                        IssueDesc::QuotationMarkNotClosed(qa.0),
                    ));
                }

                *self.cloc.nchar.as_mut().unwrap() = 0;
                *self.cloc.nline.as_mut().unwrap() += 1;
            } else if !self.non_multimerge {
                self.del_char(stack);
                self.non_multimerge = true;

                *self.cloc.nchar.as_mut().unwrap() = 0;
                *self.cloc.nline.as_mut().unwrap() += 1;
                return;
            }
        } else if stack[1].as_ref().map(|s| s.0) == Some('?')
            && stack[2].as_ref().map(|s| s.0) == Some('?')
        {
            // Macro cause lambda causes lifetime issues :/
            macro_rules! replace_char {
                ($rep:tt) => {{
                    let mut tri_loc = self.cloc.clone();
                    *tri_loc.nchar.as_mut().unwrap() -= 2;
                    if params.wtrigraphs {
                        self.issues.push(Issue::new(
                            Some(tri_loc.clone()),
                            IssueType::Warning,
                            IssueDesc::TrigraphPresent(b),
                        ));
                    }
                    if params.version.ver_ge(CVersion::Max, CppVersion::Cpp14)
                        || !params.trigraphs
                    {
                        self.issues.push(Issue::new(
                            Some(tri_loc),
                            IssueType::Warning,
                            IssueDesc::TrigraphPresentAndIgnored(b),
                        ));
                    } else {
                        self.replace_stack2(stack, Some(($rep, tri_loc)));
                        return;
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

        if self.quot_active.is_none() {
            // Macro cause lambda causes lifetime issues :/
            macro_rules! handle_comment_start {
                (CommentType :: $type:ident) => {
                    let mut com_loc = self.cloc.clone();
                    *com_loc.nchar.as_mut().unwrap() -= 1;
                    self.lc_active =
                        Some((CommentType::$type, com_loc.clone()));
                    self.replace_stack1(stack, Some((' ', com_loc)));
                    stack = self.cur_stack(stacks);
                    assert_eq!(*stack, [None, None, None]);
                };
            }
            match (&self.lc_active, &mut stack, b) {
                (None, [_, _, Some(('/', _))], '/')
                    if params
                        .version
                        .ver_ge(CVersion::C99, CppVersion::Min) =>
                {
                    handle_comment_start!(CommentType::SingleLine);
                    return;
                }
                (None, [_, _, Some(('/', _))], '*') => {
                    handle_comment_start!(CommentType::MultiLine);
                    return;
                }
                (
                    Some((CommentType::MultiLine, _)),
                    [_, _, Some(('*', _))],
                    '/',
                ) => {
                    self.lc_active = None;
                    return;
                }
                _ => (),
            }
        }

        self.insert_stack(stack, Some((b, self.cloc.clone())));
    }
}

#[derive(Debug, PartialEq)]
pub struct Output {
    pub new_file: String,
    pub issues: Vec<Issue>,
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
    let mut stacks: [CharStack; 2] = [[None, None, None], [None, None, None]];
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
            || &file[file.len() - 4..file.len() - 1] == "??/"
        {
            state.issues.push(Issue::new(
                None,
                IssueType::Warning,
                IssueDesc::FileEndMissingNewline,
            ));
        }
    }

    if params.trigraphs
        && params.version.ver_ge(CVersion::Max, CppVersion::Cpp14)
    {
        state.issues.push(Issue::new(
            None,
            IssueType::Warning,
            IssueDesc::TrigraphAndVersionConflict,
        ));
    }

    state.process_char(params, &mut stacks, '\n');
    for b in file.chars() {
        state.process_char(params, &mut stacks, b);
    }
    state.process_char(params, &mut stacks, '\n');
    state.process_char(params, &mut stacks, '\n');

    let stack = state.cur_stack(&mut stacks);
    state.drain_stack(stack);

    match state.lc_active {
        Some((CommentType::SingleLine, _)) => {
            panic!("Single line comment not at {:?}", state.lc_active)
        }
        Some((CommentType::MultiLine, loc)) => {
            state.issues.push(Issue::new(
                Some(loc),
                IssueType::Error,
                IssueDesc::MultilineCommentNotClosed,
            ));
        }
        _ => (),
    }

    match state.quot_active {
        Some((q, loc)) => {
            state.issues.push(Issue::new(
                Some(loc),
                IssueType::Error,
                IssueDesc::QuotationMarkNotClosed(q),
            ));
        }
        _ => (),
    }

    Output {
        new_file: state.new_file,
        issues: state.issues,
        loc_mapping: state.loc_mapping,
    }
}
