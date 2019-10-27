use super::*;
use crate::common::{
    CVersion, CharNumber, Issue, IssueDesc, IssueType, LineNumber, Params, Version,
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
    for trigraphs in &[true, false] {
        for conv in &[true, false] {
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
        if !params.trigraphs || params.version.ver_ge(CVersion::Max, CppVersion::Cpp14) {
            true
        } else {
            false
        }
    } else {
        false
    }
}

fn add_tri_issue(mut issues: Vec<(bool, Issue)>, params: &Params, conv: bool) -> Vec<Issue> {
    if params.trigraphs && params.version.ver_ge(CVersion::Max, CppVersion::Cpp14) {
        issues.insert(
            0,
            (
                false,
                Issue::new(
                    None,
                    IssueType::Warning,
                    IssueDesc::TrigraphAndVersionConflict,
                ),
            ),
        );
    }

    issues
        .into_iter()
        .filter_map(|(preserved_only, i)| match i.desc {
            IssueDesc::TrigraphPresent(_) if !params.wtrigraphs || !conv => None,
            IssueDesc::TrigraphPresentAndIgnored(_)
                if (params.version.ver_ls(CVersion::Max, CppVersion::Cpp14)
                    && params.trigraphs)
                    || !conv =>
            {
                None
            }
            _ if preserved_only && !output_will_preserve_trigraphs(params, conv) => None,
            _ => Some(i),
        })
        .collect()
}

fn location(nline: LineNumber, nchar: CharNumber, tris: CharNumber, params: &Params) -> Location {
    Location::new(
        FILENAME.clone(),
        nline,
        nchar + if params.trigraphs { tris } else { 0 },
    )
}

fn locations(
    params: &Params,
    loc_mapping: Vec<(bool, (Location, Location))>,
) -> Vec<(Location, Location)> {
    loc_mapping
        .into_iter()
        .filter_map(|(always, l)| {
            if params.trigraphs || always {
                Some(l)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn blank() {
    do_every("", Default::default(), |src, params, conv| {
        assert_eq!(
            preproc_phases_1_to_3(src, &*FILENAME, params,),
            Output {
                num_spaces: 1,
                new_file: String::from("\n"),
                issues: add_tri_issue(vec![], params, conv),
                loc_mapping: locations(params, vec![]),
            },
        )
    });
}

#[test]
fn singleline_merge() {
    const NEWLINES: usize = 8;
    const SLASHES: usize = 5;
    do_every(
        &(String::from("a") + &"\\".repeat(SLASHES) + &"\n".repeat(NEWLINES) + "b\n"),
        Default::default(),
        |src, params, conv| {
            assert_eq!(
                preproc_phases_1_to_3(src, &*FILENAME, params,),
                Output {
                    num_spaces: 3,
                    new_file: String::from("\na") + &"\\".repeat(SLASHES - 1) + "\nb\n",
                    issues: add_tri_issue(vec![], params, conv),
                    loc_mapping: locations(
                        params,
                        vec![
                            (
                                true,
                                (location(1, 1, 0, params), location(2, 1, 0, params),)
                            ),
                            (
                                true,
                                (location(NEWLINES, 1, 0, params), location(3, 1, 0, params),)
                            ),
                        ]
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
        &(String::from("a") + &"\\".repeat(SLASHES) + &"\nb".repeat(NEWLINES) + "\n"),
        Default::default(),
        |src, params, conv| {
            assert_eq!(
                preproc_phases_1_to_3(src, &*FILENAME, params,),
                Output {
                    num_spaces: NEWLINES + 1,
                    new_file: String::from("\na")
                        + &"\\".repeat(SLASHES - 1)
                        + &"b\n".repeat(NEWLINES),
                    issues: add_tri_issue(vec![], params, conv),
                    loc_mapping: locations(
                        params,
                        vec![
                            (
                                true,
                                (location(1, 1, 0, params), location(2, 1, 0, params),)
                            ),
                            (
                                true,
                                (location(2, 1, 0, params), location(2, 6, 0, params),)
                            ),
                        ]
                    ),
                },
            )
        },
    );
}

#[test]
fn header_comment() {
    do_every(
        "a < b // c > d\n",
        Default::default(),
        |src, params, conv| {
            assert_eq!(
                preproc_phases_1_to_3(src, &*FILENAME, params,),
                Output {
                    num_spaces: match params.version {
                        Version::C(CVersion::C89) => 8,
                        _ => 4,
                    },
                    new_file: String::from(match params.version {
                        Version::C(CVersion::C89) => "\na < b // c > d\n",
                        _ => "\na < b\n",
                    }),
                    issues: add_tri_issue(vec![], params, conv),
                    loc_mapping: locations(
                        params,
                        vec![(
                            true,
                            (location(1, 1, 0, params), location(2, 1, 0, params),)
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
        "a < b /* c > d */ e\n",
        Default::default(),
        |src, params, conv| {
            assert_eq!(
                preproc_phases_1_to_3(src, &*FILENAME, params,),
                Output {
                    num_spaces: 5,
                    new_file: String::from("\na < b e\n"),
                    issues: add_tri_issue(vec![], params, conv),
                    loc_mapping: locations(
                        params,
                        vec![
                            (
                                true,
                                (location(1, 1, 0, params), location(2, 1, 0, params),)
                            ),
                            (
                                true,
                                (location(1, 19, 0, params), location(2, 7, 0, params),)
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
            &(String::from("a ") + quo + " b // c /* d */ e /* f " + quo + " g */ h\n"),
            Default::default(),
            |src, params, conv| {
                assert_eq!(
                    preproc_phases_1_to_3(src, &*FILENAME, params,),
                    Output {
                        num_spaces: 16,
                        new_file: String::from("\na ")
                            + quo
                            + " b // c /* d */ e /* f "
                            + quo
                            + " g */ h\n",
                        issues: add_tri_issue(vec![], params, conv),
                        loc_mapping: locations(
                            params,
                            vec![(
                                true,
                                (location(1, 1, 0, params), location(2, 1, 0, params),)
                            ),]
                        ),
                    },
                )
            },
        );
    }
}

/*/*/*/*/*

a // b

a // \\\\
b
c
d

a // \\
b \\
c
d

a b \\
c d e

a /* b
c */ d

a /* b \\
c */ d

a /* b */ c

a /\\
* b */ c

a /\\
/ b // c

*/*/*/*/

#[test]
fn cross_quote_line_merge() {
    for m in &[" ", ""] {
        for qchar in &[(true, '\'', '\''), (false, '<', '>'), (true, '"', '"')] {
            eprintln!("Testing for m {:?}, qchar {:?}", m, qchar);
            do_every(
                &(String::from("a ")
                    + &qchar.1.to_string()
                    + " b"
                    + m
                    + "\\\nc "
                    + &qchar.2.to_string()
                    + " d\n"),
                Default::default(),
                |src, params, conv| {
                    let owpt = output_will_preserve_trigraphs(params, conv);
                    let ms = 6 + if *m == " " { 1 } else { 0 };
                    assert_eq!(
                        preproc_phases_1_to_3(src, &*FILENAME, params,),
                        Output {
                            num_spaces: ms + if owpt { 1 } else { 0 },
                            new_file: if owpt {
                                "\n".to_string() + src
                            } else {
                                String::from("\na ")
                                    + &qchar.1.to_string()
                                    + " b"
                                    + m
                                    + "c "
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
                                    (location(1, 1, 0, params), location(2, 1, 0, params),)
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
