#[derive(Debug, Clone)]
enum HeaderNameType {
    HChar, // '<...>'
    QChar, // '"..."'
}

#[derive(Debug, Clone)]
enum CharConstPrefix {
    None, // ''...''
    L,    // 'L'...''
    u,    // 'u'...''
    U,    // 'U'...''
}

#[derive(Debug, Clone)]
enum StrLitPrefix {
    None,    // '"..."'
    Unicode, // 'u8"..."'
    Wide,    // 'u"..."', 'U"..."', 'L"..."'
}

// Section 6.4.7, C18 spec
// String cannot contain new-line.
//
// If HChar, cannot contain '>'. If it contains ''', '\', '"', '//' or '/*',
// undefined behaviour.
//
// If QChar, cannot contain '"'. If it contains ''', '\', '//', or '/*',
// undefined behaviour.
#[derive(Debug, Clone)]
struct HeaderName(HeaderNameType, String);

// Section 6.4.8, C18 spec
//
// Must start with digit, or digit followed by period.
//
// Then can have any universal-character-name ('\u[a-fA-F0-9]`, '\U[a-fA-F0-9]{2}'),
// non-digit ('_', '[a-zA-Z]'), dot, or '(e|E|p|P)(+|-)`.
//
// According to C++ spec, can also have `,` anywhere not at the start.
//
// Look, if starts with a dot or digit, it's a number to me!
#[derive(Debug, Clone)]
struct PPNumber(String);

// Must start with non-digit. Can have any digit, non digit or '_' afterwards.
#[derive(Debug, Clone)]
struct Identifer(String);

// Any sequence, except ''', '\' and newline.
//
// May contain '\'', '\"', '\?', '\\', '\a', '\b', '\f', '\n', '\r',
// '\t', '\v', '\[0-7]{1-3}', '\x[a-fA-F0-9]*', universal-character-name
// ('\u[a-fA-F0-9]`, '\U[a-fA-F0-9]{2}').
#[derive(Debug, Clone)]
struct CharConst(CharConstPrefix, String);

// 'prefx? " char? "'
//
// May have encoding prefix: 'u8', 'u', 'U', 'L'. Adjacent StrLits will not
// have both 'u8' and the other 3.
//
// Between quotation marks, any sequence, except ''', '\' and newline.
// May contain '\'', '\"', '\?', '\\', '\a', '\b', '\f', '\n', '\r',
// '\t', '\v', '\[0-7]{1-3}', '\x[a-fA-F0-9]*', universal-character-name
// ('\u[a-fA-F0-9]`, '\U[a-fA-F0-9]{2}').
#[derive(Debug, Clone)]
struct StrLit(StrLitPrefix, String);

// Section 6.4.6, C18 spec,
// with some exclusions lifted to PPToken.
#[derive(Debug, Clone)]
enum Punct {
    LSqrtBrk,  // '[', '<:'
    RSqrtBrk,  // ']', ':>'
    LCurlyBrk, // '{', '<%'
    RCurlyBrk, // '}', '%>'

    Dot,     // '.'
    Pointer, // '->'

    Quest,  // '?'
    Colon,  // ':'
    SColon, // ';'

    DoubleAmp,  // '&&'
    DoublePipe, // '||'

    DoubleAdd, // '++'
    DoubleSub, // '--'

    Eq,          // '='
    DoubleEq,    // '=='
    Add,         // '+'
    PlusEq,      // '+='
    Sub,         // '-'
    MinusEq,     // '-='
    Mul,         // '*'
    MulEq,       // '*='
    Div,         // '/'
    DivEq,       // '/='
    Mod,         // '%'
    ModEq,       // '%='
    LAngleBrk,   // '<'
    LAngleBrkEq, // '<='
    RAngleBrk,   // '>'
    RAngleBrkEq, // '>='

    Not,               // '!'
    NotEq,             // '!='
    Pipe,              // '|'
    PipeEq,            // '|='
    Comp,              // '~'
    Amp,               // '&'
    AmpEq,             // '&='
    Pow,               // '^'
    PowEq,             // '^='
    LDoubleAngleBrk,   // '<<'
    LDoubleAngleBrkEq, // '<<='
    RDoubleAngleBrk,   // '>>'
    RDoubleAngleBrkEq, // '>>='
    TripleDot,         // '...'
    LParen,            // '('
    RParen,            // ')'
    Hash,              // '#', '%:'
    DoubleHash,        // '##', '%:%:'
    Comma,             // ','
}

#[derive(Debug, Clone)]
enum PPToken {
    NonPreProc(String),
    HeaderName(HeaderName),
    PPNumber(PPNumber),
    Identifer(Identifer),
    CharConst(CharConst),
    StrLit(StrLit),
    Punct(Punct),
    NewLine,
}

#[derive(Debug, Clone)]
struct Filename(String);

#[derive(Debug, Clone)]
enum DiagnosticType {
    Warning,
    Error,
    Ignored,
    Fatal,
}

#[derive(Debug, Clone)]
enum PragmaTokens {
    None,
    Once,

    // https://gcc.gnu.org/onlinedocs/gcc/Push_002fPop-Macro-Pragmas.html#Push_002fPop-Macro-Pragmas
    PushMacro(Identifer),
    PopMacro(Identifer),

    // https://gcc.gnu.org/onlinedocs/gcc/Diagnostic-Pragmas.html
    // http://www.keil.com/support/man/docs/armclang_ref/armclang_ref_vvi1455119010903.htm
    Diagnostic(DiagnosticType, StrLit),
    DiagnosticPush,
    DiagnosticPop,

    // https://gcc.gnu.org/onlinedocs/cpp/System-Headers.html#System-Headers
    // http://www.keil.com/support/man/docs/armclang_ref/armclang_ref_chr1383580039043.htm
    //
    // '#pragma SystemHeader newline' or '#pragma clang system_header newline'
    SystemHeader,

    Other(Vec<PPToken>),
}

#[derive(Debug, Clone)]
enum MessageType {
    Warning,
    Error,
    PragmaError,
    PragmaWarning,
}

#[derive(Debug, Clone)]
enum IncludeType {
    Import,
    Include,
    IncludeNext,
}

// Cannot contain assignment, increment, decrement, function-call, or
// comma operators unless when they are not evaluated.
//
// Needs to also handle defined, __has_attribute, and __has_cpp_attribute.
#[derive(Debug, Clone)]
struct ConstExpr(Vec<PPToken>);

#[derive(Debug, Clone)]
enum Token {
    NonPreProc(String),
    NullDirective, // '# newline'
    NonStdDefined,
    // '#error error_message newline' and '#warning warning_message newline'.
    //
    // There is also '#pragma GCC warning warning_message newline',
    // and '#pragma GCC error error_message newline',  which will all be `Token::Msg(...)`, not
    // `Token::Pragma(...)`.
    //
    // If we encounter an error, or a warning when -Werror is set, we should simply
    // stop and run the preproccesor locally to get the error output, and cache
    // the failure. If the preproccesor doesn't error as predicted, then we've
    // encountered a bug in our impl!
    //
    // https://gcc.gnu.org/onlinedocs/cpp/Diagnostics.html#Diagnostics
    // https://gcc.gnu.org/onlinedocs/gcc/Diagnostic-Pragmas.html
    Msg(MessageType, Vec<PPToken>),

    // https://gcc.gnu.org/onlinedocs/cpp/Pragmas.html
    Pragma(PragmaTokens), // '#pragma tokens newline', also, '_Pragma ("str") newline'

    // https://gcc.gnu.org/onlinedocs/cpp/Line-Control.html#Line-Control
    // '#line lineno "filename"? newline' or '#line <tokens>`.
    // StrLitPrefix == None, if present.
    Line(Option<Vec<PPToken>>, Option<u64>, Option<StrLit>),

    // https://gcc.gnu.org/onlinedocs/cpp/Include-Syntax.html
    // https://gcc.gnu.org/onlinedocs/gcc-3.1/cpp/Obsolete-once-only-headers.html
    // https://gcc.gnu.org/onlinedocs/cpp/System-Headers.html
    // https://gcc.gnu.org/onlinedocs/gcc-3.2.3/cpp/Wrapper-Headers.html
    //
    // Both '#import ...', '#include_next ...' and '#include ...'.
    Include(IncludeType, HeaderName),

    // https://gcc.gnu.org/onlinedocs/cpp/_005f_005fhas_005finclude.html#g_t_005f_005fhas_005finclude
    // https://clang.llvm.org/docs/LanguageExtensions.html#include-file-checking-macros
    HasInclude(IncludeType, HeaderName),

    // https://gcc.gnu.org/onlinedocs/cpp/Macros.html#Macros
    Define(Identifer, Vec<PPToken>),
    DefineFn(Identifer, Vec<Identifer>, Vec<PPToken>),
    Undef(Identifer),

    // https://gcc.gnu.org/onlinedocs/cpp/Conditional-Syntax.html#Conditional-Syntax
    Ifdef(Identifer),
    Ifndef(Identifer),

    If(ConstExpr),
    Else,
    ElseIf(ConstExpr),
    EndIf,

    Other(String),
}
