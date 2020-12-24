use proc_macro2::TokenStream;
use regex::Regex;
use syn::{Expr, Lit};

/// Parse token stream like ("foo")
pub fn parse_string_arg(item: &TokenStream) -> Result<String, String> {
    let s = (move || -> Result<String, ()> {
        let expr = syn::parse2::<syn::ExprParen>(item.clone()).map_err(|_| ())?;
        let lit = match *expr.expr {
            Expr::Lit(lit) => lit,
            _ => return Err(()),
        };
        let s = match lit.lit {
            Lit::Str(s) => s.value(),
            _ => return Err(()),
        };

        Ok(s)
    })()
    .map_err(|_| format!("Parse token error, expect (\"\\w*\")"))?;

    Ok(s)
}

/// Parse token stream like "foo"
pub fn parse_string_token(item: &TokenStream) -> Result<String, String> {
    let s = (move || -> Result<String, ()> {
        let lit = syn::parse2::<syn::Lit>(item.clone()).map_err(|_| ())?;
        let s = match lit {
            Lit::Str(s) => s.value(),
            _ => return Err(()),
        };

        Ok(s)
    })()
    .map_err(|_| format!("Parse token error, expect \"\\w*\""))?;

    Ok(s)
}

/// "//a//b/c/" => "/a/b/c"
pub fn normalize_url<S: AsRef<str>>(url: S) -> String {
    let re = Regex::new(r"/+").unwrap();
    let mut s = re.replace_all(url.as_ref(), "/").to_string();

    if s.chars().next() != Some('/') {
        s = format!("/{}", s);
    }
    if s.chars().last() == Some('/') {
        s.pop();
    }

    s
}

/// Get vec[a, b] from url "/api/{a}/{b}"
pub fn get_args_from_url<S: AsRef<str>>(url: S) -> Vec<String> {
    let url = url.as_ref();
    let re = Regex::new(r"\{(\w+)\}").unwrap();
    re.find_iter(url)
        .map(|m| (&url[(m.start() + 1)..(m.end() - 1)]).to_string())
        .collect()
}

/// Parse token stream to syn type, call proc_macro_error::abort! if error occurred
///
/// ```rust
/// let list: syn::ExprTuple = parse2! { attr.tokens,
///     "Syntax error of module imports";
///     note = "Syntax is #[imports(MODULE_A as TYPE_A, MODULEB as TYPE_B,)]";
/// };
/// ```
macro_rules! parse2 {
    ($tokens:expr, $($tts:tt)*) => {
        match syn::parse2($tokens.clone()) {
            Ok(v) => v,
            Err(_) => {
                proc_macro_error::abort! { $tokens,
                    $($tts)*
                };
            },
        }
    };
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_normalize_url() {
        assert_eq!(normalize_url("a").as_str(), "/a");
        assert_eq!(normalize_url("a/a").as_str(), "/a/a");
        assert_eq!(normalize_url("a/a/").as_str(), "/a/a");
        assert_eq!(normalize_url("a//a").as_str(), "/a/a");
        assert_eq!(normalize_url("a//////////a///b").as_str(), "/a/a/b");
        assert_eq!(normalize_url("/////////a///b").as_str(), "/a/b");
        assert_eq!(normalize_url("/////////a///b//////////").as_str(), "/a/b");
    }
}
