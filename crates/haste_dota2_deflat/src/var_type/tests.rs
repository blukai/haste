use super::{parse, ArrayLength, Decl, Token, Tokenizer};

fn lex(input: &str) -> Vec<Token<'_>> {
    Tokenizer::new(input).collect()
}

#[test]
fn test_primitive() {
    let input = "bool";
    assert_eq!(lex(input), vec![Token::Ident("bool"),]);
    assert_eq!(parse(input), Decl::Ident("bool"));
}

#[test]
fn test_template() {
    let input = "CHandle< CBaseAnimatingActivity >";
    assert_eq!(
        lex(input),
        vec![
            Token::Ident("CHandle"),
            Token::LAngle,
            Token::Ident("CBaseAnimatingActivity"),
            Token::RAngle,
        ]
    );
    assert_eq!(
        parse(input),
        Decl::Template {
            ident: "CHandle",
            argument: Box::new(Decl::Ident("CBaseAnimatingActivity"))
        }
    );
}

#[test]
fn test_array() {
    let input = "uint64[256]";
    assert_eq!(
        lex(input),
        vec![
            Token::Ident("uint64"),
            Token::LSquare,
            Token::Number("256"),
            Token::RSquare,
        ]
    );
    assert_eq!(
        parse(input),
        Decl::Array {
            decl: Box::new(Decl::Ident("uint64")),
            length: ArrayLength::Number(256)
        }
    );
}

#[test]
fn test_pointer() {
    let input = "CDOTAGameManager*";
    assert_eq!(
        lex(input),
        vec![Token::Ident("CDOTAGameManager"), Token::Asterisk]
    );
    assert_eq!(
        parse(input),
        Decl::Pointer(Box::new(Decl::Ident("CDOTAGameManager")))
    );
}

#[test]
fn test_nested_template() {
    let input = "CNetworkUtlVectorBase< CHandle< CBasePlayerController > >";
    assert_eq!(
        lex(input),
        vec![
            Token::Ident("CNetworkUtlVectorBase"),
            Token::LAngle,
            Token::Ident("CHandle"),
            Token::LAngle,
            Token::Ident("CBasePlayerController"),
            Token::RAngle,
            Token::RAngle
        ]
    );
    assert_eq!(
        parse(input),
        Decl::Template {
            ident: "CNetworkUtlVectorBase",
            argument: Box::new(Decl::Template {
                ident: "CHandle",
                argument: Box::new(Decl::Ident("CBasePlayerController"))
            })
        }
    );
}

#[test]
fn test_template_array() {
    let input = "CHandle< CDOTASpecGraphPlayerData >[24]";
    assert_eq!(
        lex(input),
        vec![
            Token::Ident("CHandle"),
            Token::LAngle,
            Token::Ident("CDOTASpecGraphPlayerData"),
            Token::RAngle,
            Token::LSquare,
            Token::Number("24"),
            Token::RSquare
        ]
    );
    assert_eq!(
        parse(input),
        Decl::Array {
            decl: Box::new(Decl::Template {
                ident: "CHandle",
                argument: Box::new(Decl::Ident("CDOTASpecGraphPlayerData"))
            }),
            length: ArrayLength::Number(24)
        }
    );
}

#[test]
fn test_array_with_constant_length() {
    let input = "CDOTA_AbilityDraftAbilityState[MAX_ABILITY_DRAFT_ABILITIES]";
    assert_eq!(
        lex(input),
        vec![
            Token::Ident("CDOTA_AbilityDraftAbilityState"),
            Token::LSquare,
            Token::Ident("MAX_ABILITY_DRAFT_ABILITIES"),
            Token::RSquare
        ],
    );
    assert_eq!(
        parse(input),
        Decl::Array {
            decl: Box::new(Decl::Ident("CDOTA_AbilityDraftAbilityState")),
            length: ArrayLength::Ident("MAX_ABILITY_DRAFT_ABILITIES")
        }
    );
}
