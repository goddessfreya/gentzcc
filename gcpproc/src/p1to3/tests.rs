use super::*;
use crate::common::{CVersion, Issue, IssueDesc, IssueType, Params, Version};

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

fn do_every<F: Fn(Version)>(f: F) {
    for v in &EVERY_VERSION {
        eprintln!("Testing for version {:?}", v);
        f(*v);
    }
}

#[test]
fn blank() {
    do_every(|version| {
        assert_eq!(
            preproc_phases_1_to_3(
                "",
                &*FILENAME,
                &Params {
                    version,
                    ..Default::default()
                }
            ),
            Output {
                num_spaces:  1,
                new_file:    String::from("\n"),
                issues:      vec![],
                loc_mapping: vec![],
            },
        )
    });
}

#[test]
fn singleline_merge() {
    const NEWLINES: usize = 5;
    const SLASHES: usize = 8;
    do_every(|version| {
        assert_eq!(
            preproc_phases_1_to_3(
                &(String::from("a")
                    + &"\\".repeat(SLASHES)
                    + &"\n".repeat(NEWLINES)
                    + "b\n"),
                &*FILENAME,
                &Params {
                    version,
                    ..Default::default()
                },
            ),
            Output {
                num_spaces:  3,
                new_file:    String::from("\na")
                    + &"\\".repeat(SLASHES - 1)
                    + "\nb\n",
                issues:      vec![],
                loc_mapping: vec![
                    (
                        Location::new(FILENAME.clone(), 1, 1),
                        Location::new(FILENAME.clone(), 2, 1),
                    ),
                    (
                        Location::new(FILENAME.clone(), NEWLINES, 1),
                        Location::new(FILENAME.clone(), 3, 1),
                    ),
                ],
            },
        )
    });
}

#[test]
fn singleline_merge_nonblank() {
    const NEWLINES: usize = 5;
    const SLASHES: usize = 8;
    do_every(|version| {
        assert_eq!(
            preproc_phases_1_to_3(
                &(String::from("a")
                    + &"\\".repeat(SLASHES)
                    + &"\nb".repeat(NEWLINES)
                    + "\n"),
                &*FILENAME,
                &Params {
                    version,
                    ..Default::default()
                },
            ),
            Output {
                num_spaces:  NEWLINES + 1,
                new_file:    String::from("\na")
                    + &"\\".repeat(SLASHES - 1)
                    + &"b\n".repeat(NEWLINES),
                issues:      vec![],
                loc_mapping: vec![
                    (
                        Location::new(FILENAME.clone(), 1, 1),
                        Location::new(FILENAME.clone(), 2, 1),
                    ),
                    (
                        Location::new(FILENAME.clone(), 2, 1),
                        Location::new(FILENAME.clone(), 2, 9),
                    ),
                ],
            },
        )
    });
}

#[test]
fn header_comment() {
    do_every(|version| {
        assert_eq!(
            preproc_phases_1_to_3(
                "a < b // c > d\n",
                &*FILENAME,
                &Params {
                    version,
                    ..Default::default()
                }
            ),
            Output {
                num_spaces:  match version {
                    Version::C(CVersion::C89) => 8,
                    _ => 4,
                },
                new_file:    String::from(match version {
                    Version::C(CVersion::C89) => "\na < b // c > d\n",
                    _ => "\na < b\n",
                }),
                issues:      vec![],
                loc_mapping: vec![(
                    Location::new(FILENAME.clone(), 1, 1),
                    Location::new(FILENAME.clone(), 2, 1),
                ),],
            },
        )
    });
}

#[test]
fn header_multicomment() {
    do_every(|version| {
        assert_eq!(
            preproc_phases_1_to_3(
                "a < b /* c > d */ e\n",
                &*FILENAME,
                &Params {
                    version,
                    ..Default::default()
                }
            ),
            Output {
                num_spaces:  5,
                new_file:    String::from("\na < b e\n"),
                issues:      vec![],
                loc_mapping: vec![
                    (
                        Location::new(FILENAME.clone(), 1, 1),
                        Location::new(FILENAME.clone(), 2, 1),
                    ),
                    (
                        Location::new(FILENAME.clone(), 1, 19),
                        Location::new(FILENAME.clone(), 2, 7),
                    ),
                ],
            },
        )
    });
}

#[test]
fn double_quote_comment() {
    do_every(|version| {
        assert_eq!(
            preproc_phases_1_to_3(
                "a \"b // c /* d */ e /* f\" g */ h\n",
                &*FILENAME,
                &Params {
                    version,
                    ..Default::default()
                }
            ),
            Output {
                num_spaces:  14,
                new_file:    String::from(
                    "\na \"b // c /* d */ e /* f\" g */ h\n"
                ),
                issues:      vec![],
                loc_mapping: vec![(
                    Location::new(FILENAME.clone(), 1, 1),
                    Location::new(FILENAME.clone(), 2, 1),
                ),],
            },
        )
    });
}

#[test]
fn single_quote_comment() {
    do_every(|version| {
        assert_eq!(
            preproc_phases_1_to_3(
                "a 'b // c /* d */ e /* f' g */ h\n",
                &*FILENAME,
                &Params {
                    version,
                    ..Default::default()
                }
            ),
            Output {
                num_spaces:  14,
                new_file:    String::from(
                    "\na 'b // c /* d */ e /* f' g */ h\n"
                ),
                issues:      vec![],
                loc_mapping: vec![(
                    Location::new(FILENAME.clone(), 1, 1),
                    Location::new(FILENAME.clone(), 2, 1),
                ),],
            },
        )
    });
}
