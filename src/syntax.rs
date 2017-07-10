use std::string::*;
use utils::*;

use std::ops::Deref;
use std::ops::DerefMut;

use std::iter::IntoIterator;
use std::iter::DoubleEndedIterator;

pub type E = P<Form>;

/// Represents a top level definition,
/// `def` or `data` or `type`
#[derive(Clone, PartialEq, Debug)]
pub struct Def {
    pub ident: Name,
    pub node: Item,
    pub pos: Pos,
}

impl Def {
    /// Create a definition node define a form (value).
    pub fn value<S: ToString>(pos: Pos, name: S, body: E) -> P<Def> {
        P(Def {
              ident: name.to_string(),
              node: Item::Form(body),
              pos,
          })
    }

    /// If this is a form define
    pub fn is_form(&self) -> bool {
        match self.node {
            Item::Form(_) => true,
            _ => false,
        }
    }

    /// Set the type of form
    ///   only called if this is a value definition
    pub fn set_form_scheme(&mut self, scm: Scheme) {
        match self.node {
            Item::Form(ref mut f) => {
                (*f).deref_mut().tag.set_scheme(scm);
            },
            _ => panic!("Set scheme to a non-form definition")
        }
    }

    /// Get the type of form
    ///   only called if this is a value definition
    pub fn form_type<'a>(&'a self) -> &'a Scheme {
        match self.node {
            Item::Form(ref f) => f.tag.ref_scheme(),
            _ => panic!("Get scheme from a non-form definition")
        }
    }

    /// Get the form as a body of definition
    pub fn form_body_mut<'a>(&'a mut self) -> &'a mut Form {
        match self.node {
            Item::Form(ref mut f) => (*f).deref_mut(),
            _ => panic!("Get form from a non-form definition")
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Item {
    Form(E),
    Alias(P<Scheme>),
    Alg(Vec<Variant>),
}

/// Represents a variant of `data` type
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Variant {
    pub name: Name,
    pub body: VariantBody,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum VariantBody {
    Struct(Vec<Field>),
    Tuple(Vec<Field>),
    Unit,
}

/// A field definition of struct in `data`
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Field {
    pub pos: Pos,
    pub name: Option<Name>,
    pub ty: P<Type>,
}


/// Represents a form
#[derive(Clone, PartialEq,  Debug)]
pub struct Form {
    /// Expression content
    pub node: Expr,

    /// Form tag
    pub tag: FormTag,
}

impl Form {
    pub fn new(pos: Pos, exp: Expr) -> Form {
        Form {
            node: exp,
            tag: FormTag {
                pos: pos,
                ty: Scheme::slot(),
                annotate: None,
            },
        }
    }
    pub fn typed(pos: Pos, ty: Scheme, exp: Expr) -> Form {
        Form {
            node: exp,
            tag: FormTag {
                pos,
                ty,
                annotate: None,
            },
        }
    }
    pub fn annotated(pos: Pos, anno: Scheme, exp: Expr) -> Form {
        Form {
            node: exp,
            tag: FormTag {
                pos,
                ty: Scheme::slot(),
                annotate: Some(anno),
            },
        }
    }
    pub fn abs(pos: Pos, params: Vec<VarDecl>, to: P<Form>) -> Form {
        Form {
            node: Expr::Abs(Lambda {
                                param: params,
                                body: to,
                            }),
            tag: FormTag {
                pos: pos,
                ty: Scheme::slot(),
                annotate: None,
            },
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct FormTag {
    /// Position in source
    pub pos: Pos,
    /// Type of node
    pub ty: Scheme,
    /// Annotate type
    pub annotate: Option<Scheme>,
}

impl FormTag {
    pub fn set_type(&mut self, ty: Type) {
        self.ty = Scheme::Mono(P(ty));
    }

    pub fn set_scheme(&mut self, scm: Scheme) {
        self.ty = scm;
    }

    pub fn ref_type(&self) -> &Type {
        self.ty.body()
    }

    pub fn clone_type(&self) -> Type {
        self.ty.body().clone()
    }

    pub fn ref_scheme(&self) -> &Scheme {
        &self.ty
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Copy)]
pub struct Pos {
    line: usize,
    col: usize,
}

impl Pos {
    pub fn new(l: usize, c: usize) -> Pos {
        Pos { line: l, col: c }
    }
}


/// Type scheme
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Scheme {
    /// A monomorphism type
    /// `Int * Int -> Double`
    Mono(P<Type>),

    /// Polymorphism type
    /// `forall a. a * a -> a`
    Poly(Vec<Name>, P<Type>),

    /// Unknown type
    Slot,
}

impl Scheme {
    pub fn slot() -> Scheme {
        Scheme::Slot
    }

    pub fn con<S: ToString>(name: S) -> Scheme {
        Scheme::Mono(P(Type::Con(name.to_string())))
    }

    pub fn body(&self) -> &Type {
        match *self {
            Scheme::Mono(ref t) |
            Scheme::Poly(_, ref t) => (*t).deref(),
            Scheme::Slot => unreachable!(),
        }
    }

    pub fn arrow(from: Type, to: Type) -> Scheme {
        Scheme::Mono(P(Type::Arr(P(from), P(to))))
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Type {
    Void,
    /// Type variable
    Var(Name),
    /// Constant type name
    Con(Name),
    /// Arrow (->) type
    Arr(P<Type>, P<Type>),
    /// Product (*) type
    Prod(P<Type>, P<Type>),
    /// Composite type
    Comp(P<Type>, P<Type>),
}

impl Type {
    pub fn product(left: Type, right: Type) -> Type {
        Type::Prod(P(left), P(right))
        // Type::Comp(P(Type::Con("->".to_string())))
    }
    pub fn compose(callee: Type, arg: Type) -> Type {
        Type::Comp(P(callee), P(arg))
    }

    pub fn product_n<'a, I>(ts: I) -> Type
        where I: IntoIterator<Item = Type>,
              <I as IntoIterator>::IntoIter: DoubleEndedIterator
    {

        let mut it = ts.into_iter().rev();
        let last = it.next().unwrap();
        let mut res = last;
        for ty in it {
            res = Type::Prod(P(ty), P(res));
        }
        res
    }
}

/// Represents a expression evalutated to a value
#[derive(Clone, PartialEq, Debug)]
pub enum Expr {
    /// Literial value
    Lit(Lit),
    /// Identifier (binding/definition)
    Var(Name),
    /// List (array)
    /// e.g. `[fuck, shit]`
    List(Vec<E>),
    /// Block (statement sequence)
    /// e.g. `{ print(fuck); print(shit); 1 }`
    Block(Vec<E>),
    /// Function apply
    /// `fuck(shit, 1)`
    Apply(E, Vec<E>),

    /// Abstruction (function)
    /// e.g. `(fuck, shit) -> fuck + shit`
    Abs(Lambda),

    /// Binary operator expression
    /// e.g. `fuck + shit`
    Binary(BinOp, E, E),
    /// Unary operator expression
    /// e.g. `!fuck`
    /// e.g. `-shit`
    Unary(UnOp, E),

    /// Let-in expression
    /// e.g. `let fuck = shit in fuck + 1`
    Let(VarDecl, E, E),
    /// Conditional expression
    /// e.g. `if (fuck == shit) 1 else 0`
    If(E, E, E),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Lambda {
    pub param: Vec<VarDecl>,
    pub body: E,
}

#[derive(Debug, PartialEq, Clone)]
pub struct VarDecl(pub Name, pub Scheme);

impl VarDecl {
    pub fn name(&self) -> Name {
        let &VarDecl(ref name, _) = self;
        name.clone()
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Lit {
    Int(i32),
    Float(f64),
    Str(String),
    Bool(bool),
}

impl Lit {
    pub fn lit_type(&self) -> Type {
        use self::Lit::*;
        let ty_str = match *self {
            Lit::Int(_) => "Int",
            Lit::Float(_) => "Float",
            Lit::Str(_) => "String",
            Lit::Bool(_) => "Bool",
        };

        Type::Con(ty_str.to_string())
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Copy)]
pub enum BinOp {
    /// The `+` operator (addition)
    Add,
    /// The `-` operator (subtraction)
    Sub,
    /// The `*` operator (multiplication)
    Mul,
    /// The `/` operator (division)
    Div,
    /// The `%` operator (modulus)
    Rem,
    /// The `&&` operator (logical and)
    And,
    /// The `||` operator (logical or)
    Or,
    /// The `^` operator (bitwise xor)
    BitXor,
    /// The `&` operator (bitwise and)
    BitAnd,
    /// The `|` operator (bitwise or)
    BitOr,
    /// The `<<` operator (shift left)
    Shl,
    /// The `>>` operator (shift right)
    Shr,
    /// The `==` operator (equality)
    Eq,
    /// The `<` operator (less than)
    Lt,
    /// The `<=` operator (less than or equal to)
    Le,
    /// The `!=` operator (not equal to)
    Ne,
    /// The `>=` operator (greater than or equal to)
    Ge,
    /// The `>` operator (greater than)
    Gt,
}
impl BinOp {
    pub fn as_str(&self) -> &'static str {
        use self::BinOp::*;
        match *self {
            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
            Rem => "%",
            And => "&&",
            Or => "||",
            BitXor => "^",
            BitAnd => "&",
            BitOr => "|",
            Shl => "<<",
            Shr => ">>",
            Eq => "==",
            Lt => "<",
            Le => "<=",
            Ne => "!=",
            Ge => ">=",
            Gt => ">",
        }
    }
    pub fn take(op_str: &str) -> BinOp {
        use self::BinOp::*;
        match op_str {
            "+" => Add,
            "-" => Sub,
            "*" => Mul,
            "/" => Div,
            "%" => Rem,
            "<" => Lt,
            "<=" => Le,
            "!=" => Ne,
            ">=" => Ge,
            ">" => Gt,
            "==" => Eq,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Copy)]
pub enum UnOp {
    /// The `!` operator for logical inversion
    Not,
    /// The `-` operator for negation
    Neg,
}

impl UnOp {
    pub fn to_string(op: UnOp) -> &'static str {
        match op {
            UnOp::Not => "!",
            UnOp::Neg => "-",
        }
    }
}