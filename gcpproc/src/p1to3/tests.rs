use super::*;
use crate::common::{
    CVersion, CharNumber, Issue, IssueDesc, IssueType, LineNumber, Params,
    Version,
};

lazy_static! {
    static ref FILENAME: String = String::from("filename");
}

const EVERY_VERSION: [Version; 10] = [
    Version::Cpp(CppVersion::Cpp98),
    Version::Cpp(CppVersion::Cpp03),
    Version::Cpp(CppVersion::Cpp11),
    Version::Cpp(CppVersion::Cpp14),
    Version::Cpp(CppVersion::Cpp17),
    Version::Cpp(CppVersion::Cpp20),
    Version::C(CVersion::C89),
    Version::C(CVersion::C99),
    Version::C(CVersion::C11),
    Version::C(CVersion::C18),
];

fn trigraphify(s: &str) -> String {
    s.replace("{", "??<")
        .replace("}", "??>")
        .replace("[", "??(")
        .replace("]", "??)")
        .replace("#", "??=")
        .replace("\\", "??/")
        .replace("^", "??\'")
        .replace("|", "??!")
        .replace("~", "??-")
}

fn do_every<F: Fn(&str, &Params, bool)>(src: &str, mut params: Params, f: F) {
    for trigraphs in &[false, true] {
        for conv in &[false, true] {
            for wtrigraphs in &[true, false] {
                params.wtrigraphs = *wtrigraphs;
                for v in &EVERY_VERSION {
                    params.version = *v;
                    params.trigraphs = *trigraphs;
                    eprintln!(
                        "Testing for version {:?}, trigraphs {:?}, wtrigraphs {:?}, conv {:?}",
                        v, params.trigraphs, params.wtrigraphs, conv,
                    );
                    let src = if *conv {
                        trigraphify(src)
                    } else {
                        src.to_string()
                    };
                    f(&src, &params, *conv);
                }
            }
        }
    }
}

fn output_will_preserve_trigraphs(params: &Params, conv: bool) -> bool {
    if conv {
        if !params.trigraphs
            || params.version.ver_ge(CVersion::Max, CppVersion::Cpp14)
        {
            true
        } else {
            false
        }
    } else {
        false
    }
}

fn add_tri_issue(
    mut issues: Vec<(NeedConv, NeedOWPT, Issue)>,
    params: &Params,
    conv: bool,
) -> Vec<Issue> {
    if params.trigraphs
        && params.version.ver_ge(CVersion::Max, CppVersion::Cpp14)
    {
        issues.insert(
            0,
            (
                NeedConv::No,
                NeedOWPT::No,
                Issue::new(
                    None,
                    IssueType::Warning,
                    IssueDesc::TrigraphAndVersionConflict,
                ),
            ),
        );
    }

    let owpt = output_will_preserve_trigraphs(params, conv);
    issues
        .into_iter()
        .filter_map(|(nconv, nowpt, i)| match nowpt {
            NeedOWPT::Yes if !owpt => None,
            NeedOWPT::Invalid if owpt => None,
            _ => match nconv {
                NeedConv::Yes if !conv => None,
                NeedConv::Invalid if conv => None,
                _ => Some(i),
            },
        })
        .filter_map(|mut i| match i.desc {
            IssueDesc::TrigraphPresent(c) => {
                let tp = if !params.wtrigraphs || !conv {
                    None
                } else {
                    Some(i.clone())
                };
                let tpi =
                    if (params.version.ver_ls(CVersion::Max, CppVersion::Cpp14)
                        && params.trigraphs)
                        || !conv
                    {
                        None
                    } else {
                        i.desc = IssueDesc::TrigraphPresentAndIgnored(c);
                        Some(i)
                    };
                Some((tp, tpi))
            }
            _ => Some((Some(i), None)),
        })
        .flat_map(|(a, b)| vec![a, b])
        .filter_map(|a| a)
        .collect()
}

macro_rules! location {
    ($nline:expr, $nchar:expr) => {{
        Location::new(FILENAME.clone(), $nline, $nchar)
    }};
}

#[derive(Debug, PartialEq)]
enum NeedConv {
    Yes,
    No,
    Invalid,
}

#[derive(Debug, PartialEq)]
enum NeedOWPT {
    Yes,
    No,
    Invalid,
}

fn locations(
    params: &Params,
    conv: bool,
    loc_mapping: Vec<(NeedConv, NeedOWPT, (Location, Location))>,
) -> Vec<(Location, Location)> {
    let owpt = output_will_preserve_trigraphs(params, conv);
    loc_mapping
        .into_iter()
        .filter_map(|(nconv, nowpt, l)| match nowpt {
            NeedOWPT::Yes if !owpt => None,
            NeedOWPT::Invalid if owpt => None,
            _ => match nconv {
                NeedConv::Yes if !conv => None,
                NeedConv::Invalid if conv => None,
                _ => Some(l),
            },
        })
        .collect()
}

#[test]
fn blank() {
    do_every("", Default::default(), |src, params, conv| {
        fassert_eq!(
            preproc_phases_1_to_3(src, &*FILENAME, params),
            Output {
                new_file: String::from("\n"),
                issues: add_tri_issue(vec![], params, conv),
                loc_mapping: locations(params, conv, vec![]),
            },
        )
    });
}

#[test]
fn absent_newline() {
    for ending in &["", "\\\n"] {
        eprintln!("Testing for ending {:?}", ending);
        do_every(
            &(String::from("#a\n#a") + ending),
            Default::default(),
            |src, params, conv| {
                let owpt = output_will_preserve_trigraphs(params, conv);
                fassert_eq!(
                    preproc_phases_1_to_3(src, &*FILENAME, params),
                    Output {
                        new_file: if owpt {
                            "\n".to_string()
                                + src
                                + if !owpt || *ending == "" { "\n" } else { "" }
                        } else {
                            String::from("\n#a\n#a\n")
                        },
                        issues: add_tri_issue(
                            if params
                                .version
                                .ver_le(CVersion::Max, CppVersion::Cpp03)
                            {
                                Some((
                                    NeedConv::No,
                                    NeedOWPT::No,
                                    Issue::new(
                                        None,
                                        IssueType::Warning,
                                        IssueDesc::FileEndMissingNewline,
                                    ),
                                ))
                            } else {
                                None
                            }
                            .into_iter()
                            .chain(vec![
                                (
                                    NeedConv::No,
                                    NeedOWPT::No,
                                    Issue::new(
                                        Some(location!(1, 1)),
                                        IssueType::Warning,
                                        IssueDesc::TrigraphPresent('=')
                                    )
                                ),
                                (
                                    NeedConv::No,
                                    NeedOWPT::No,
                                    Issue::new(
                                        Some(location!(2, 1)),
                                        IssueType::Warning,
                                        IssueDesc::TrigraphPresent('=')
                                    )
                                ),
                            ])
                            .chain(if *ending == "\\\n" {
                                vec![(
                                    NeedConv::No,
                                    NeedOWPT::No,
                                    Issue::new(
                                        Some(location!(2, 5)),
                                        IssueType::Warning,
                                        IssueDesc::TrigraphPresent('/'),
                                    ),
                                )]
                            } else {
                                vec![]
                            })
                            .collect(),
                            params,
                            conv
                        ),
                        loc_mapping: locations(
                            params,
                            conv,
                            vec![
                                (
                                    NeedConv::No,
                                    NeedOWPT::No,
                                    (location!(1, 1), location!(2, 1))
                                ),
                                (
                                    NeedConv::Yes,
                                    NeedOWPT::Invalid,
                                    (location!(1, 4), location!(2, 2))
                                ),
                                (
                                    NeedConv::Yes,
                                    NeedOWPT::Invalid,
                                    (location!(2, 4), location!(3, 2))
                                ),
                            ]
                        ),
                    },
                )
            },
        );
    }
}

#[test]
fn unifty_non_preproc() {
    do_every("a\nb\nc\nd\n", Default::default(), |src, params, conv| {
        fassert_eq!(
            preproc_phases_1_to_3(src, &*FILENAME, params),
            Output {
                new_file: String::from("\na\nb\nc\nd\n"),
                issues: add_tri_issue(vec![], params, conv),
                loc_mapping: locations(
                    params,
                    conv,
                    vec![(
                        NeedConv::No,
                        NeedOWPT::No,
                        (location!(1, 1), location!(2, 1))
                    ),]
                ),
            },
        )
    });
}

#[test]
fn unifty_mixed_preproc() {
    do_every(
        "a\n#b\\\nc\\\nd\ne\nf\n#g\\\nh\\\n#i\\\n#j\nk\nl#\nm\nn\n",
        Default::default(),
        |src, params, conv| {
            let owpt = output_will_preserve_trigraphs(params, conv);
            fassert_eq!(
                preproc_phases_1_to_3(src, &*FILENAME, params),
                Output {
                    new_file: if owpt {
                        String::from("\n") + src
                    } else {
                        String::from("\na\n#bcd\ne\nf\n#gh#i#j\nk\nl#\nm\nn\n")
                    },
                    issues: add_tri_issue(
                        vec![
                            (
                                NeedConv::No,
                                NeedOWPT::No,
                                Issue::new(
                                    Some(location!(2, 1)),
                                    IssueType::Warning,
                                    IssueDesc::TrigraphPresent('=')
                                )
                            ),
                            (
                                NeedConv::No,
                                NeedOWPT::No,
                                Issue::new(
                                    Some(location!(2, 5)),
                                    IssueType::Warning,
                                    IssueDesc::TrigraphPresent('/')
                                )
                            ),
                            (
                                NeedConv::No,
                                NeedOWPT::No,
                                Issue::new(
                                    Some(location!(3, 2)),
                                    IssueType::Warning,
                                    IssueDesc::TrigraphPresent('/')
                                )
                            ),
                            (
                                NeedConv::No,
                                NeedOWPT::No,
                                Issue::new(
                                    Some(location!(7, 1)),
                                    IssueType::Warning,
                                    IssueDesc::TrigraphPresent('=')
                                )
                            ),
                            (
                                NeedConv::No,
                                NeedOWPT::No,
                                Issue::new(
                                    Some(location!(7, 5)),
                                    IssueType::Warning,
                                    IssueDesc::TrigraphPresent('/')
                                )
                            ),
                            (
                                NeedConv::No,
                                NeedOWPT::No,
                                Issue::new(
                                    Some(location!(8, 2)),
                                    IssueType::Warning,
                                    IssueDesc::TrigraphPresent('/')
                                )
                            ),
                            (
                                NeedConv::No,
                                NeedOWPT::No,
                                Issue::new(
                                    Some(location!(9, 1)),
                                    IssueType::Warning,
                                    IssueDesc::TrigraphPresent('=')
                                )
                            ),
                            (
                                NeedConv::No,
                                NeedOWPT::No,
                                Issue::new(
                                    Some(location!(9, 5)),
                                    IssueType::Warning,
                                    IssueDesc::TrigraphPresent('/')
                                )
                            ),
                            (
                                NeedConv::No,
                                NeedOWPT::No,
                                Issue::new(
                                    Some(location!(10, 1)),
                                    IssueType::Warning,
                                    IssueDesc::TrigraphPresent('=')
                                )
                            ),
                            (
                                NeedConv::No,
                                NeedOWPT::No,
                                Issue::new(
                                    Some(location!(12, 2)),
                                    IssueType::Warning,
                                    IssueDesc::TrigraphPresent('=')
                                )
                            ),
                        ],
                        params,
                        conv
                    ),
                    loc_mapping: locations(
                        params,
                        conv,
                        vec![
                            (
                                NeedConv::No,
                                NeedOWPT::No,
                                (location!(1, 1), location!(2, 1))
                            ),
                            (
                                NeedConv::Yes,
                                NeedOWPT::Invalid,
                                (location!(2, 4), location!(3, 2))
                            ),
                            (
                                NeedConv::No,
                                NeedOWPT::Invalid,
                                (location!(3, 1), location!(3, 3))
                            ),
                            (
                                NeedConv::No,
                                NeedOWPT::Invalid,
                                (location!(4, 1), location!(3, 4))
                            ),
                            (
                                NeedConv::Yes,
                                NeedOWPT::Invalid,
                                (location!(7, 4), location!(6, 2))
                            ),
                            (
                                NeedConv::No,
                                NeedOWPT::Invalid,
                                (location!(8, 1), location!(6, 3))
                            ),
                            (
                                NeedConv::No,
                                NeedOWPT::Invalid,
                                (location!(9, 1), location!(6, 4))
                            ),
                            (
                                NeedConv::Yes,
                                NeedOWPT::Invalid,
                                (location!(9, 4), location!(6, 5))
                            ),
                            (
                                NeedConv::No,
                                NeedOWPT::Invalid,
                                (location!(10, 1), location!(6, 6))
                            ),
                            (
                                NeedConv::Yes,
                                NeedOWPT::Invalid,
                                (location!(10, 4), location!(6, 7))
                            ),
                        ]
                    ),
                },
            )
        },
    );
}

#[test]
fn singleline_merge() {
    const NEWLINES: usize = 8;
    const SLASHES: usize = 5;
    do_every(
        &(String::from("#a")
            + &"\\".repeat(SLASHES)
            + &"\n".repeat(NEWLINES)
            + "b\n"),
        Default::default(),
        |src, params, conv| {
            let owpt = output_will_preserve_trigraphs(params, conv);
            fassert_eq!(
                preproc_phases_1_to_3(src, &*FILENAME, params),
                Output {
                    new_file: if owpt {
                        trigraphify(
                            &(String::from("\n#a")
                                + &"\\".repeat(SLASHES)
                                + "\nb\n"),
                        )
                    } else {
                        String::from("\n#a")
                            + &"\\".repeat(SLASHES - 1)
                            + "\nb\n"
                    },
                    issues: add_tri_issue(
                        vec![(
                            NeedConv::No,
                            NeedOWPT::No,
                            Issue::new(
                                Some(location!(1, 1)),
                                IssueType::Warning,
                                IssueDesc::TrigraphPresent('=')
                            )
                        )]
                        .into_iter()
                        .chain((0..SLASHES).into_iter().map(|i| (
                            NeedConv::No,
                            NeedOWPT::No,
                            Issue::new(
                                Some(location!(1, 5 + i * 3)),
                                IssueType::Warning,
                                IssueDesc::TrigraphPresent('/')
                            )
                        )))
                        .collect(),
                        params,
                        conv
                    ),
                    loc_mapping: locations(
                        params,
                        conv,
                        vec![
                            (
                                NeedConv::No,
                                NeedOWPT::No,
                                (location!(1, 1), location!(2, 1))
                            ),
                            (
                                NeedConv::Yes,
                                NeedOWPT::Invalid,
                                (location!(1, 4), location!(2, 2))
                            ),
                        ]
                        .into_iter()
                        .chain((0..SLASHES - 2).into_iter().map(|i| (
                            NeedConv::Yes,
                            NeedOWPT::Invalid,
                            (location!(1, 8 + i * 3), location!(2, 4 + i))
                        )))
                        .chain(vec![
                            (
                                NeedConv::No,
                                NeedOWPT::Invalid,
                                (location!(NEWLINES, 1), location!(3, 1))
                            ),
                            (
                                NeedConv::Yes,
                                NeedOWPT::Yes,
                                (location!(NEWLINES + 1, 1), location!(3, 1))
                            )
                        ])
                        .collect(),
                    ),
                },
            )
        },
    );
}

#[test]
fn singleline_merge_nonblank() {
    const NEWLINES: usize = 8;
    const SLASHES: usize = 5;
    do_every(
        dbg!(
            &(String::from("#a")
                + &"\\".repeat(SLASHES)
                + &"\n#b".repeat(NEWLINES)
                + "\n")
        ),
        Default::default(),
        |src, params, conv| {
            let owpt = output_will_preserve_trigraphs(params, conv);
            fassert_eq!(
                preproc_phases_1_to_3(src, &*FILENAME, params),
                Output {
                    new_file: if owpt {
                        trigraphify(
                            &(String::from("\n")
                                + src))
                    } else {
                        String::from("\n#a")
                            + &"\\".repeat(SLASHES - 1)
                            + &"#b\n".repeat(NEWLINES)
                    },
                    issues: add_tri_issue(vec![
                            (
                                NeedConv::No,
                                NeedOWPT::No,
                                Issue::new(
                                    Some(location!(1, 1)),
                                    IssueType::Warning,
                                    IssueDesc::TrigraphPresent('=')
                                )
                            ),
                        ]
                        .into_iter()
                        .chain((0..SLASHES).into_iter().map(|i| (
                                NeedConv::No,
                                NeedOWPT::No,
                                Issue::new(
                                    Some(location!(1, 5 + i * 3)),
                                    IssueType::Warning,
                                    IssueDesc::TrigraphPresent('/')
                                )
                            )))
                        .chain((0..NEWLINES).into_iter().map(|i| (
                                NeedConv::No,
                                NeedOWPT::No,
                                Issue::new(
                                    Some(location!(2 + i, 1)),
                                    IssueType::Warning,
                                    IssueDesc::TrigraphPresent('=')
                                )
                            )))
                        .collect()
                    , params, conv),
                    loc_mapping: locations(
                        params,
                        conv,
                        vec![
                            (
                                NeedConv::No,
                                NeedOWPT::No,
                                (location!(1, 1), location!(2, 1))
                            ),
                            (
                                NeedConv::Yes,
                                NeedOWPT::Invalid,
                                (location!(1, 4), location!(2, 2))
                            ),
                        ]
                        .into_iter()
                        .chain((0..SLASHES - 2).into_iter().map(|i| (
                                NeedConv::Yes,
                                NeedOWPT::Invalid,
                                (location!(1, 8 + i * 3), location!(2, 4 + i))
                        )))
                        .chain(vec![
                            (
                                // No Invalid
                                // Invalid No, Invalid Invalid
                                NeedConv::No,
                                NeedOWPT::Invalid,
                                (location!(2, 1), location!(2, 7))
                            ),
                            (
                                NeedConv::Yes,
                                NeedOWPT::Invalid,
                                (location!(2, 4), location!(2, 8))
                            ),
                        ])
                        .chain((0..NEWLINES - 1).into_iter().map(|i| (
                                NeedConv::Yes,
                                NeedOWPT::Invalid,
                                (location!(3 + i, 4), location!(3 + i, 2))
                        )))
                        .collect()
                    ),
                },
            )
        },
    );
}

/*
#[test]
fn header_comment() {
    do_every(
        "#a < b // c > d\n",
        Default::default(),
        |src, params, conv| {
            fassert_eq!(
                preproc_phases_1_to_3(src, &*FILENAME, params),
                Output {
                    new_file: String::from(match params.version {
                        Version::C(CVersion::C89) => "\n#a < b // c > d\n",
                        _ => "\n#a < b\n",
                    }),
                    issues: add_tri_issue(vec![], params, conv),
                    loc_mapping: locations(
                        params,
                        vec![(
                            true,
                            (location(1, 1, 0, params), location(2, 1, 0, params))
                        ),]
                    ),
                },
            )
        },
    );
}

#[test]
fn header_multicomment() {
    do_every(
        "#a < b /* c > d */ e\n",
        Default::default(),
        |src, params, conv| {
            fassert_eq!(
                preproc_phases_1_to_3(src, &*FILENAME, params),
                Output {
                    new_file: String::from("\n#a < b e\n"),
                    issues: add_tri_issue(vec![], params, conv),
                    loc_mapping: locations(
                        params,
                        vec![
                            (
                                true,
                                (location(1, 1, 0, params), location(2, 1, 0, params))
                            ),
                            (
                                true,
                                (location(1, 19, 0, params), location(2, 7, 0, params))
                            ),
                        ]
                    ),
                },
            )
        },
    );
}

#[test]
fn quote_comment() {
    for quo in &["'", "\""] {
        do_every(
            &(String::from("#a ") + quo + " b // c /* d */ e /* f " + quo + " g */ h\n"),
            Default::default(),
            |src, params, conv| {
                fassert_eq!(
                    preproc_phases_1_to_3(src, &*FILENAME, params),
                    Output {
                        new_file: String::from("\n#a ")
                            + quo
                            + " b // c /* d */ e /* f "
                            + quo
                            + " g */ h\n",
                        issues: add_tri_issue(vec![], params, conv),
                        loc_mapping: locations(
                            params,
                            vec![(
                                true,
                                (location(1, 1, 0, params), location(2, 1, 0, params))
                            ),]
                        ),
                    },
                )
            },
        );
    }
}

#[test]
fn line_merge_simple() {
    for m in &[" ", ""] {
        do_every(
            &(String::from("#b") + m + "\\\n#c\n"),
            Default::default(),
            |src, params, conv| {
                fassert_eq!(
                    preproc_phases_1_to_3(src, &*FILENAME, params),
                    Output {
                        new_file: String::from("\n"),
                        issues: add_tri_issue(vec![], params, conv),
                        loc_mapping: locations(params, vec![]),
                    },
                )
            },
        );
    }
}

#[test]
fn singleline_merge_comment() {
    do_every(
        "#a // b \\\\\n#c\n#d\n#e\n",
        Default::default(),
        |src, params, conv| {
            fassert_eq!(
                preproc_phases_1_to_3(src, &*FILENAME, params),
                Output {
                    new_file: String::from("\n"),
                    issues: add_tri_issue(vec![], params, conv),
                    loc_mapping: locations(params, vec![]),
                },
            )
        },
    );
}

#[test]
fn multilineline_merge_comment() {
    do_every(
        "#a // b \\\n#c \\\n#d\n#e\n",
        Default::default(),
        |src, params, conv| {
            fassert_eq!(
                preproc_phases_1_to_3(src, &*FILENAME, params),
                Output {
                    new_file: String::from("\n"),
                    issues: add_tri_issue(vec![], params, conv),
                    loc_mapping: locations(params, vec![]),
                },
            )
        },
    );
}

#[test]
fn multiline_comment() {
    do_every("#a /* b */ c\n", Default::default(), |src, params, conv| {
        fassert_eq!(
            preproc_phases_1_to_3(src, &*FILENAME, params),
            Output {
                new_file: String::from("\n"),
                issues: add_tri_issue(vec![], params, conv),
                loc_mapping: locations(params, vec![]),
            },
        )
    });
}

#[test]
fn multiline_comment_split() {
    do_every(
        "#a /* b\n#c */ d\n",
        Default::default(),
        |src, params, conv| {
            fassert_eq!(
                preproc_phases_1_to_3(src, &*FILENAME, params),
                Output {
                    new_file: String::from("\n"),
                    issues: add_tri_issue(vec![], params, conv),
                    loc_mapping: locations(params, vec![]),
                },
            )
        },
    );
}

#[test]
fn multiline_comment_merge() {
    do_every(
        "#a /* b \\\n#c */ d\n",
        Default::default(),
        |src, params, conv| {
            fassert_eq!(
                preproc_phases_1_to_3(src, &*FILENAME, params),
                Output {
                    new_file: String::from("\n"),
                    issues: add_tri_issue(vec![], params, conv),
                    loc_mapping: locations(params, vec![]),
                },
            )
        },
    );
}

#[test]
fn singleline_comment_merge_split() {
    do_every(
        "#a /\\\n/ b /\\\n/ c\n",
        Default::default(),
        |src, params, conv| {
            fassert_eq!(
                preproc_phases_1_to_3(src, &*FILENAME, params),
                Output {
                    new_file: String::from("\n"),
                    issues: add_tri_issue(vec![], params, conv),
                    loc_mapping: locations(params, vec![]),
                },
            )
        },
    );
}

#[test]
fn multiline_comment_merge_split() {
    do_every(
        "#a /\\\n* b *\\\n/ c\n",
        Default::default(),
        |src, params, conv| {
            fassert_eq!(
                preproc_phases_1_to_3(src, &*FILENAME, params),
                Output {
                    new_file: String::from("\n"),
                    issues: add_tri_issue(vec![], params, conv),
                    loc_mapping: locations(params, vec![]),
                },
            )
        },
    );
}

#[test]
fn cross_quote_line_merge() {
    for m in &[" ", ""] {
        for qchar in &[(true, '\'', '\''), (false, '<', '>'), (true, '"', '"')] {
            eprintln!("Testing for m {:?}, qchar {:?}", m, qchar);
            do_every(
                &(String::from("#a ")
                    + &qchar.1.to_string()
                    + " b"
                    + m
                    + "\\\n#c "
                    + &qchar.2.to_string()
                    + " d\n"),
                Default::default(),
                |src, params, conv| {
                    let owpt = output_will_preserve_trigraphs(params, conv);
                    let ms = 6 + if *m == " " { 1 } else { 0 };
                    fassert_eq!(
                        preproc_phases_1_to_3(src, &*FILENAME, params),
                        Output {
                            new_file: if owpt {
                                "\n".to_string() + src
                            } else {
                                String::from("\n#a ")
                                    + &qchar.1.to_string()
                                    + " b"
                                    + m
                                    + "#c "
                                    + &qchar.2.to_string()
                                    + " d\n"
                            },
                            issues: add_tri_issue(
                                vec![
                                    (
                                        false,
                                        Issue::new(
                                            Some(location(1, ms, 0, params)),
                                            IssueType::Warning,
                                            IssueDesc::TrigraphPresent('/')
                                        )
                                    ),
                                    (
                                        false,
                                        Issue::new(
                                            Some(location(1, ms, 0, params)),
                                            IssueType::Warning,
                                            IssueDesc::TrigraphPresentAndIgnored('/')
                                        )
                                    ),
                                ]
                                .into_iter()
                                .chain(if qchar.0 && owpt {
                                    Some((
                                        true,
                                        Issue::new(
                                            Some(location(1, 3, 0, params)),
                                            IssueType::Warning,
                                            IssueDesc::QuotationMarkNotClosed(qchar.1),
                                        ),
                                    ))
                                } else {
                                    None
                                })
                                .collect(),
                                params,
                                conv
                            ),
                            loc_mapping: locations(
                                params,
                                vec![(
                                    true,
                                    (location(1, 1, 0, params), location(2, 1, 0, params))
                                ),]
                                .into_iter()
                                .chain(if !owpt {
                                    Some((
                                        true,
                                        (location(2, 1, 0, params), location(2, ms, 0, params)),
                                    ))
                                } else {
                                    None
                                })
                                .collect()
                            ),
                        },
                    )
                },
            );
        }
    }
}
*/
