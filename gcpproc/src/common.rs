//! Common files for preprocessor.

/// All the parameters the preprocessor cares about.
pub struct Params {
    // -std=<version>
    pub version: Version,

    // TODO: -Wcomment/-Wcomments
    // -Wno-comment/-Wnocomments
    // wcomment: bool,

    // -Wtrigraphs
    // -Wno-trigraphs
    pub wtrigraphs: bool,

    // TODO:
    // -I <dir>
    // -iquote <dir>
    // -isystem <dir>
    // -idirafter <dir>
    // -iprefix <prefix>
    // -iwithprefix <dir>
    // -iwithprefixbefore <dir>
    // -isysroot <dir>
    // -imultilib <dir>

    // -trigraphs
    pub trigraphs: bool,
    // TODO: -Wundef
    // -Wno-undef
    // wundef: bool,

    // TODO: -Wexpansion-to-defined
    // -Wno-expansion-to-defined
    // wexpansion_to_defined: bool,

    // TODO: -Wunused-macros
    // -Wno-unused-macros
    // wunused-macros: bool,

    // TODO: -Wendif-labels
    // -Wno-endif-labels
    // wno_endif_labels: bool,

    // TODO: -D <name>/-D <name>=<definition>/-U <name>
    // macro_defines: Vec<(String, Option<String>)>,

    // TODO: -undef
    // undef: bool,

    // TODO: -pthread
    // pthread: bool,

    // TODO: -fpreprocessed
    // fpreprocessed: bool,

    // TODO: -fdirectives-only
    // fdirectives_only: bool,

    // TODO: -fworking-directory
    // fworking_directory: bool,

    // TODO: -finput-charset=

    // TODO: -C/-CC
    // no_discard_comments: CommentDiscardMode,

    // TODO: -P
    // disable_linemarkers: bool,

    // TODO: -include <file>
    // include: Vec<String>,

    // TODO: -imacros <file>
    // imacros: Vec<String>,

    // Do we want to support -M and others? I think not.
}

impl Default for Params {
    fn default() -> Self {
        Params {
            trigraphs: false,
            wtrigraphs: false,
            // NOTE: gcc's default for C is C11, and for C++ it's C++14.
            // Remember to decide between the two based on fileext.
            version: Version::C(CVersion::C11),
        }
    }
}

pub type LineNumber = usize;
pub type CharNumber = usize;

#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub filename: String,
    pub nline: LineNumber,
    pub nchar: CharNumber,
}

impl Location {
    pub fn new(filename: String, nline: LineNumber, nchar: CharNumber) -> Self {
        Location {
            filename,
            nline,
            nchar,
        }
    }
}

/// Describes a diagnostic message that we've encountered.
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub loc: Location,
    pub itype: IssueType,
    pub desc: IssueDesc,
}

impl Issue {
    pub fn new(loc: Location, itype: IssueType, desc: IssueDesc) -> Self {
        Issue { loc, itype, desc }
    }
}

/// Describes a diagnostic message that we've encountered.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum IssueType {
    Error,
    Warning,
    Info,
}

/// Describes a diagnostic message that we've encountered.
#[derive(Debug, Clone, PartialEq)]
pub enum IssueDesc {
    MultilineCommentNotClosed,
    // Type of quotation mark, e.g., '\'' or '"'.
    QuotationMarkNotClosed(char),
    // Last char in trigraph
    TrigraphPresentAndIgnored(char),
    TrigraphPresent(char),
    FileEndMissingNewline,
}

/// Describes to current version of the C/++ standard in use.
#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Version {
    C(CVersion),
    Cpp(CppVersion),
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub enum CppVersion {
    Min,
    Cpp98, // https://www.lirmm.fr/~ducour/Doc-objets/ISO+IEC+14882-1998.pdf
    Cpp03, // http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2005/n1804.pdf
    Cpp11, // http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2012/n3337.pdf
    Cpp14, // http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2014/n4296.pdf
    Cpp17, // http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2017/n4687.pdf
    Cpp20, // http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2019/n4835.pdf and up
    Max,
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub enum CVersion {
    Min,
    C89, // http://port70.net/~nsz/c/c89/c89-draft.html
    C99, // http://www.open-std.org/jtc1/sc22/wg14/www/docs/n869/n869.pdf.gz
    C11, // http://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf
    C18, // https://web.archive.org/web/20181230041359if_/http://www.open-std.org/jtc1/sc22/wg14/www/abq/c17_updated_proposed_fdis.pdf
    Max,
}

macro_rules! CVersionCmp {
    ($name:ident, $cmp:tt) => {
        #[allow(dead_code)]
        pub fn $name(&self, cver: CVersion, cppver: CppVersion) -> bool {
            match self {
                Version::Cpp(v) => v $cmp &cppver,
                Version::C(v) => v $cmp &cver,
            }
        }
    }
}

impl Version {
    CVersionCmp!(ver_gr, >);
    CVersionCmp!(ver_ls, <);
    CVersionCmp!(ver_ge, >=);
    CVersionCmp!(ver_le, <=);
    CVersionCmp!(ver_eq, ==);
    CVersionCmp!(ver_ne, !=);
}
