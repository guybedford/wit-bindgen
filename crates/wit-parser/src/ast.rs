use anyhow::Result;
use lex::{Span, Token, Tokenizer};
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;

mod lex;
mod resolve;

pub use lex::validate_id;

pub struct Ast<'a> {
    pub items: Vec<Item<'a>>,
}

pub enum Item<'a> {
    Use(Use<'a>),
    TypeDef(TypeDef<'a>),
    Value(Value<'a>),
    Interface(Interface<'a>),
}

pub struct Id<'a> {
    pub name: Cow<'a, str>,
    pub span: Span,
}

impl<'a> From<&'a str> for Id<'a> {
    fn from(s: &'a str) -> Id<'a> {
        Id {
            name: s.into(),
            span: Span { start: 0, end: 0 },
        }
    }
}

impl<'a> From<String> for Id<'a> {
    fn from(s: String) -> Id<'a> {
        Id {
            name: s.into(),
            span: Span { start: 0, end: 0 },
        }
    }
}

pub struct Use<'a> {
    pub from: Vec<Id<'a>>,
    names: Option<Vec<UseName<'a>>>,
}

struct UseName<'a> {
    name: Id<'a>,
    as_: Option<Id<'a>>,
}

#[derive(Default)]
struct Docs<'a> {
    docs: Vec<Cow<'a, str>>,
}

pub struct TypeDef<'a> {
    docs: Docs<'a>,
    name: Id<'a>,
    ty: Type<'a>,
}

enum Type<'a> {
    Bool,
    U8,
    U16,
    U32,
    U64,
    S8,
    S16,
    S32,
    S64,
    Float32,
    Float64,
    Char,
    String,
    Name(Id<'a>),
    List(Box<Type<'a>>),
    Record(Record<'a>),
    Flags(Flags<'a>),
    Variant(Variant<'a>),
    Tuple(Vec<Type<'a>>),
    Enum(Enum<'a>),
    Option(Box<Type<'a>>),
    Result(Result_<'a>),
    Future(Option<Box<Type<'a>>>),
    Stream(Stream<'a>),
    Union(Union<'a>),
}

struct Record<'a> {
    fields: Vec<Field<'a>>,
}

struct Field<'a> {
    docs: Docs<'a>,
    name: Id<'a>,
    ty: Type<'a>,
}

struct Flags<'a> {
    flags: Vec<Flag<'a>>,
}

struct Flag<'a> {
    docs: Docs<'a>,
    name: Id<'a>,
}

struct Variant<'a> {
    span: Span,
    cases: Vec<Case<'a>>,
}

struct Case<'a> {
    docs: Docs<'a>,
    name: Id<'a>,
    ty: Option<Type<'a>>,
}

struct Enum<'a> {
    span: Span,
    cases: Vec<EnumCase<'a>>,
}

struct EnumCase<'a> {
    docs: Docs<'a>,
    name: Id<'a>,
}

struct Result_<'a> {
    ok: Option<Box<Type<'a>>>,
    err: Option<Box<Type<'a>>>,
}

struct Stream<'a> {
    element: Option<Box<Type<'a>>>,
    end: Option<Box<Type<'a>>>,
}

pub struct Value<'a> {
    docs: Docs<'a>,
    name: Id<'a>,
    kind: ValueKind<'a>,
}

struct Union<'a> {
    span: Span,
    cases: Vec<UnionCase<'a>>,
}

struct UnionCase<'a> {
    docs: Docs<'a>,
    ty: Type<'a>,
}

type ParamList<'a> = Vec<(Id<'a>, Type<'a>)>;

enum ResultList<'a> {
    Named(ParamList<'a>),
    Anon(Type<'a>),
}

enum ValueKind<'a> {
    Function {
        params: ParamList<'a>,
        results: ResultList<'a>,
    },
    Global(Type<'a>),
}

#[allow(dead_code)] // TODO
pub struct Interface<'a> {
    docs: Docs<'a>,
    name: Id<'a>,
    items: Vec<Item<'a>>,
}

impl<'a> Ast<'a> {
    pub fn parse(input: &'a str) -> Result<Ast<'a>> {
        let mut lexer = Tokenizer::new(input)?;
        let mut items = Vec::new();
        while lexer.clone().next()?.is_some() {
            let docs = parse_docs(&mut lexer)?;
            items.push(Item::parse(&mut lexer, docs)?);
        }
        Ok(Ast { items })
    }

    pub fn resolve(
        &self,
        name: &str,
        map: &HashMap<String, crate::Interface>,
    ) -> Result<crate::Interface> {
        let mut resolver = resolve::Resolver::default();
        let instance = resolver.resolve(name, &self.items, map)?;
        Ok(instance)
    }
}

impl<'a> Item<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Item<'a>> {
        match tokens.clone().next()? {
            Some((_span, Token::Use)) => Use::parse(tokens, docs).map(Item::Use),
            Some((_span, Token::Type)) => TypeDef::parse(tokens, docs).map(Item::TypeDef),
            Some((_span, Token::Flags)) => TypeDef::parse_flags(tokens, docs).map(Item::TypeDef),
            Some((_span, Token::Enum)) => TypeDef::parse_enum(tokens, docs).map(Item::TypeDef),
            Some((_span, Token::Variant)) => {
                TypeDef::parse_variant(tokens, docs).map(Item::TypeDef)
            }
            Some((_span, Token::Record)) => TypeDef::parse_record(tokens, docs).map(Item::TypeDef),
            Some((_span, Token::Union)) => TypeDef::parse_union(tokens, docs).map(Item::TypeDef),
            Some((_span, Token::Interface)) => Interface::parse(tokens, docs).map(Item::Interface),
            Some((_span, Token::Id)) | Some((_span, Token::ExplicitId)) => {
                Value::parse(tokens, docs).map(Item::Value)
            }
            other => Err(err_expected(tokens, "`type` or `func`", other).into()),
        }
    }
}

impl<'a> Use<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, _docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Use)?;
        let mut names = None;
        loop {
            if names.is_none() {
                if tokens.eat(Token::Star)? {
                    break;
                }
                tokens.expect(Token::LeftBrace)?;
                names = Some(Vec::new());
            }
            let names = names.as_mut().unwrap();
            let mut name = UseName {
                name: parse_id(tokens)?,
                as_: None,
            };
            if tokens.eat(Token::As)? {
                name.as_ = Some(parse_id(tokens)?);
            }
            names.push(name);
            if !tokens.eat(Token::Comma)? {
                break;
            }
        }
        if names.is_some() {
            tokens.expect(Token::RightBrace)?;
        }
        tokens.expect(Token::From_)?;
        let mut from = vec![parse_id(tokens)?];
        while tokens.eat(Token::Colon)? {
            tokens.expect_raw(Token::Colon)?;
            from.push(parse_id(tokens)?);
        }
        Ok(Use { from, names })
    }
}

impl<'a> TypeDef<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Type)?;
        let name = parse_id(tokens)?;
        tokens.expect(Token::Equals)?;
        let ty = Type::parse(tokens)?;
        Ok(TypeDef { docs, name, ty })
    }

    fn parse_flags(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Flags)?;
        let name = parse_id(tokens)?;
        let ty = Type::Flags(Flags {
            flags: parse_list(
                tokens,
                Token::LeftBrace,
                Token::RightBrace,
                |docs, tokens| {
                    let name = parse_id(tokens)?;
                    Ok(Flag { docs, name })
                },
            )?,
        });
        Ok(TypeDef { docs, name, ty })
    }

    fn parse_record(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Record)?;
        let name = parse_id(tokens)?;
        let ty = Type::Record(Record {
            fields: parse_list(
                tokens,
                Token::LeftBrace,
                Token::RightBrace,
                |docs, tokens| {
                    let name = parse_id(tokens)?;
                    tokens.expect(Token::Colon)?;
                    let ty = Type::parse(tokens)?;
                    Ok(Field { docs, name, ty })
                },
            )?,
        });
        Ok(TypeDef { docs, name, ty })
    }

    fn parse_variant(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Variant)?;
        let name = parse_id(tokens)?;
        let ty = Type::Variant(Variant {
            span: name.span,
            cases: parse_list(
                tokens,
                Token::LeftBrace,
                Token::RightBrace,
                |docs, tokens| {
                    let name = parse_id(tokens)?;
                    let ty = if tokens.eat(Token::LeftParen)? {
                        let ty = Type::parse(tokens)?;
                        tokens.expect(Token::RightParen)?;
                        Some(ty)
                    } else {
                        None
                    };
                    Ok(Case { docs, name, ty })
                },
            )?,
        });
        Ok(TypeDef { docs, name, ty })
    }

    fn parse_union(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Union)?;
        let name = parse_id(tokens)?;
        let ty = Type::Union(Union {
            span: name.span,
            cases: parse_list(
                tokens,
                Token::LeftBrace,
                Token::RightBrace,
                |docs, tokens| {
                    let ty = Type::parse(tokens)?;
                    Ok(UnionCase { docs, ty })
                },
            )?,
        });
        Ok(TypeDef { docs, name, ty })
    }

    fn parse_enum(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Enum)?;
        let name = parse_id(tokens)?;
        let ty = Type::Enum(Enum {
            span: name.span,
            cases: parse_list(
                tokens,
                Token::LeftBrace,
                Token::RightBrace,
                |docs, tokens| {
                    let name = parse_id(tokens)?;
                    Ok(EnumCase { docs, name })
                },
            )?,
        });
        Ok(TypeDef { docs, name, ty })
    }
}

impl<'a> Value<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        let name = parse_id(tokens)?;
        tokens.expect(Token::Colon)?;

        let kind = if tokens.eat(Token::Func)? {
            parse_func(tokens)?
        } else {
            ValueKind::Global(Type::parse(tokens)?)
        };
        return Ok(Value { docs, name, kind });

        fn parse_params<'a>(tokens: &mut Tokenizer<'a>, left_paren: bool) -> Result<ParamList<'a>> {
            if left_paren {
                tokens.expect(Token::LeftParen)?;
            };
            parse_list_trailer(tokens, Token::RightParen, |_docs, tokens| {
                let name = parse_id(tokens)?;
                tokens.expect(Token::Colon)?;
                let ty = Type::parse(tokens)?;
                Ok((name, ty))
            })
        }

        fn parse_func<'a>(tokens: &mut Tokenizer<'a>) -> Result<ValueKind<'a>> {
            let params = parse_params(tokens, true)?;
            let results = if tokens.eat(Token::RArrow)? {
                // If we eat a '(', parse the remainder of the named
                // result types. Otherwise parse a single anonymous type.
                if tokens.eat(Token::LeftParen)? {
                    let results = parse_params(tokens, false)?;
                    ResultList::Named(results)
                } else {
                    let ty = Type::parse(tokens)?;
                    ResultList::Anon(ty)
                }
            } else {
                ResultList::Named(Vec::new())
            };
            Ok(ValueKind::Function { params, results })
        }
    }
}

fn parse_id<'a>(tokens: &mut Tokenizer<'a>) -> Result<Id<'a>> {
    match tokens.next()? {
        Some((span, Token::Id)) => Ok(Id {
            name: tokens.parse_id(span)?.into(),
            span,
        }),
        Some((span, Token::ExplicitId)) => Ok(Id {
            name: tokens.parse_explicit_id(span)?.into(),
            span,
        }),
        other => Err(err_expected(tokens, "an identifier or string", other).into()),
    }
}

fn parse_docs<'a>(tokens: &mut Tokenizer<'a>) -> Result<Docs<'a>> {
    let mut docs = Docs::default();
    let mut clone = tokens.clone();
    while let Some((span, token)) = clone.next_raw()? {
        match token {
            Token::Whitespace => {}
            Token::Comment => docs.docs.push(tokens.get_span(span).into()),
            _ => break,
        };
        *tokens = clone.clone();
    }
    Ok(docs)
}

impl<'a> Type<'a> {
    fn parse(tokens: &mut Tokenizer<'a>) -> Result<Self> {
        match tokens.next()? {
            Some((_span, Token::U8)) => Ok(Type::U8),
            Some((_span, Token::U16)) => Ok(Type::U16),
            Some((_span, Token::U32)) => Ok(Type::U32),
            Some((_span, Token::U64)) => Ok(Type::U64),
            Some((_span, Token::S8)) => Ok(Type::S8),
            Some((_span, Token::S16)) => Ok(Type::S16),
            Some((_span, Token::S32)) => Ok(Type::S32),
            Some((_span, Token::S64)) => Ok(Type::S64),
            Some((_span, Token::Float32)) => Ok(Type::Float32),
            Some((_span, Token::Float64)) => Ok(Type::Float64),
            Some((_span, Token::Char)) => Ok(Type::Char),

            // tuple<T, U, ...>
            Some((_span, Token::Tuple)) => {
                let types = parse_list(
                    tokens,
                    Token::LessThan,
                    Token::GreaterThan,
                    |_docs, tokens| Type::parse(tokens),
                )?;
                Ok(Type::Tuple(types))
            }

            Some((_span, Token::Bool)) => Ok(Type::Bool),
            Some((_span, Token::String_)) => Ok(Type::String),

            // list<T>
            Some((_span, Token::List)) => {
                tokens.expect(Token::LessThan)?;
                let ty = Type::parse(tokens)?;
                tokens.expect(Token::GreaterThan)?;
                Ok(Type::List(Box::new(ty)))
            }

            // option<T>
            Some((_span, Token::Option_)) => {
                tokens.expect(Token::LessThan)?;
                let ty = Type::parse(tokens)?;
                tokens.expect(Token::GreaterThan)?;
                Ok(Type::Option(Box::new(ty)))
            }

            // result<T, E>
            // result<_, E>
            // result<T>
            // result
            Some((_span, Token::Result_)) => {
                let mut ok = None;
                let mut err = None;

                if tokens.eat(Token::LessThan)? {
                    if tokens.eat(Token::Underscore)? {
                        tokens.expect(Token::Comma)?;
                        err = Some(Box::new(Type::parse(tokens)?));
                    } else {
                        ok = Some(Box::new(Type::parse(tokens)?));
                        if tokens.eat(Token::Comma)? {
                            err = Some(Box::new(Type::parse(tokens)?));
                        }
                    };
                    tokens.expect(Token::GreaterThan)?;
                };
                Ok(Type::Result(Result_ { ok, err }))
            }

            // future<T>
            // future
            Some((_span, Token::Future)) => {
                let mut ty = None;

                if tokens.eat(Token::LessThan)? {
                    ty = Some(Box::new(Type::parse(tokens)?));
                    tokens.expect(Token::GreaterThan)?;
                };
                Ok(Type::Future(ty))
            }

            // stream<T, Z>
            // stream<_, Z>
            // stream<T>
            // stream
            Some((_span, Token::Stream)) => {
                let mut element = None;
                let mut end = None;

                if tokens.eat(Token::LessThan)? {
                    if tokens.eat(Token::Underscore)? {
                        tokens.expect(Token::Comma)?;
                        end = Some(Box::new(Type::parse(tokens)?));
                    } else {
                        element = Some(Box::new(Type::parse(tokens)?));
                        if tokens.eat(Token::Comma)? {
                            end = Some(Box::new(Type::parse(tokens)?));
                        }
                    };
                    tokens.expect(Token::GreaterThan)?;
                };
                Ok(Type::Stream(Stream { element, end }))
            }

            // `foo`
            Some((span, Token::Id)) => Ok(Type::Name(Id {
                name: tokens.parse_id(span)?.into(),
                span,
            })),
            // `%foo`
            Some((span, Token::ExplicitId)) => Ok(Type::Name(Id {
                name: tokens.parse_explicit_id(span)?.into(),
                span,
            })),

            other => Err(err_expected(tokens, "a type", other).into()),
        }
    }
}

impl<'a> Interface<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Interface)?;
        let name = parse_id(tokens)?;
        tokens.expect(Token::LeftBrace)?;
        let mut items = Vec::new();
        loop {
            let docs = parse_docs(tokens)?;
            if tokens.eat(Token::RightBrace)? {
                break;
            }
            items.push(Item::parse(tokens, docs)?);
        }
        Ok(Interface { docs, name, items })
    }
}

fn parse_list<'a, T>(
    tokens: &mut Tokenizer<'a>,
    start: Token,
    end: Token,
    parse: impl FnMut(Docs<'a>, &mut Tokenizer<'a>) -> Result<T>,
) -> Result<Vec<T>> {
    tokens.expect(start)?;
    parse_list_trailer(tokens, end, parse)
}

fn parse_list_trailer<'a, T>(
    tokens: &mut Tokenizer<'a>,
    end: Token,
    mut parse: impl FnMut(Docs<'a>, &mut Tokenizer<'a>) -> Result<T>,
) -> Result<Vec<T>> {
    let mut items = Vec::new();
    loop {
        // get docs before we skip them to try to eat the end token
        let docs = parse_docs(tokens)?;

        // if we found an end token then we're done
        if tokens.eat(end)? {
            break;
        }

        let item = parse(docs, tokens)?;
        items.push(item);

        // if there's no trailing comma then this is required to be the end,
        // otherwise we go through the loop to try to get another item
        if !tokens.eat(Token::Comma)? {
            tokens.expect(end)?;
            break;
        }
    }
    Ok(items)
}

fn err_expected(
    tokens: &Tokenizer<'_>,
    expected: &'static str,
    found: Option<(Span, Token)>,
) -> Error {
    match found {
        Some((span, token)) => Error {
            span,
            msg: format!("expected {}, found {}", expected, token.describe()),
        },
        None => Error {
            span: Span {
                start: u32::try_from(tokens.input().len()).unwrap(),
                end: u32::try_from(tokens.input().len()).unwrap(),
            },
            msg: format!("expected {}, found eof", expected),
        },
    }
}

#[derive(Debug)]
struct Error {
    span: Span,
    msg: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.msg.fmt(f)
    }
}

impl std::error::Error for Error {}

pub fn rewrite_error(err: &mut anyhow::Error, file: &str, contents: &str) {
    let parse = match err.downcast_mut::<Error>() {
        Some(err) => err,
        None => return lex::rewrite_error(err, file, contents),
    };
    let msg = highlight_err(
        parse.span.start as usize,
        Some(parse.span.end as usize),
        file,
        contents,
        &parse.msg,
    );
    *err = anyhow::anyhow!("{}", msg);
}

fn highlight_err(
    start: usize,
    end: Option<usize>,
    file: &str,
    input: &str,
    err: impl fmt::Display,
) -> String {
    let (line, col) = linecol_in(start, input);
    let snippet = input.lines().nth(line).unwrap_or("");
    let mut msg = format!(
        "\
{err}
     --> {file}:{line}:{col}
      |
 {line:4} | {snippet}
      | {marker:>0$}",
        col + 1,
        file = file,
        line = line + 1,
        col = col + 1,
        err = err,
        snippet = snippet,
        marker = "^",
    );
    if let Some(end) = end {
        if let Some(s) = input.get(start..end) {
            for _ in s.chars().skip(1) {
                msg.push('-');
            }
        }
    }
    return msg;

    fn linecol_in(pos: usize, text: &str) -> (usize, usize) {
        let mut cur = 0;
        // Use split_terminator instead of lines so that if there is a `\r`,
        // it is included in the offset calculation. The `+1` values below
        // account for the `\n`.
        for (i, line) in text.split_terminator('\n').enumerate() {
            if cur + line.len() + 1 > pos {
                return (i, pos - cur);
            }
            cur += line.len() + 1;
        }
        (text.lines().count(), 0)
    }
}
